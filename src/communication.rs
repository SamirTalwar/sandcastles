use std::io;

use crate::error::{CommunicationError, CommunicationResult, DaemonError};
use crate::names::Name;
use crate::services::Service;
use crate::wait::WaitFor;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum Request {
    Ping,
    Start(Start),
    Stop(Stop),
    List,
    Shutdown,
}

pub trait Response: Ship {}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum PingResponse {
    Pong,
}

impl Response for PingResponse {}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum StartResponse {
    Success(Name),
    Failure(DaemonError),
}

impl Response for StartResponse {}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum StopResponse {
    Success(ExitStatus),
    Failure(DaemonError),
}

impl Response for StopResponse {}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum ListResponse {
    Success(Services),
    Failure(DaemonError),
}

impl Response for ListResponse {}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum ShutdownResponse {
    Success,
}

impl Response for ShutdownResponse {}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ExitStatus {
    None,
    ExitedWithCode(u8),
    ExitedWithSignal(u8),
}

impl From<ExitStatus> for std::process::ExitCode {
    fn from(value: ExitStatus) -> Self {
        match value {
            ExitStatus::None => Self::SUCCESS,
            ExitStatus::ExitedWithCode(code) => Self::from(code),
            ExitStatus::ExitedWithSignal(signal) => Self::from(128 + signal),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Start {
    pub name: Option<Name>,
    pub service: Service,
    pub wait: WaitFor,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Stop {
    pub name: Name,
}

pub type Services = Vec<ServiceDetails>;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, tabled::Tabled)]
pub struct ServiceDetails {
    pub name: Name,
}

pub trait Ship: Sized {
    fn read_from(reader: impl io::Read) -> CommunicationResult<Self>;

    fn deserialize(buffer: &[u8]) -> CommunicationResult<Self> {
        Self::read_from(buffer)
    }

    fn write_to(&self, writer: impl io::Write) -> CommunicationResult<()>;

    fn serialize(&self) -> CommunicationResult<Vec<u8>> {
        let mut buffer = Vec::new();
        self.write_to(&mut buffer)?;
        Ok(buffer)
    }
}

impl<A: serde::Serialize + for<'de> serde::Deserialize<'de> + Sized> Ship for A {
    fn read_from(reader: impl io::Read) -> CommunicationResult<Self> {
        rmp_serde::decode::from_read(reader).map_err(|error| match error {
            rmp_serde::decode::Error::InvalidMarkerRead(io_error)
                if io_error.kind() == io::ErrorKind::UnexpectedEof =>
            {
                CommunicationError::ConnectionTerminated
            }
            error => CommunicationError::DeserializationError {
                message: error.to_string(),
            },
        })
    }

    fn write_to(&self, mut writer: impl io::Write) -> CommunicationResult<()> {
        rmp_serde::encode::write(&mut writer, self).map_err(|error| {
            CommunicationError::SerializationError {
                message: error.to_string(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::io;
    use std::os::unix::net::{UnixListener, UnixStream};

    use anyhow::Context;

    use crate::services::programs::Program;
    use crate::timing::Duration;

    use super::*;

    #[test]
    fn test_requests_are_serializable_and_deserializable() -> anyhow::Result<()> {
        let requests = vec![
            Request::Start(Start {
                name: Some("hello".parse()?),
                service: Service::Program(Program {
                    command: "program".into(),
                    arguments: vec!["one".into(), "two".into(), "three".into()],
                    environment: BTreeMap::from([
                        ("ONE".into(), "1".into()),
                        ("TWO".into(), "2".into()),
                    ]),
                }),
                wait: WaitFor::Time {
                    duration: Duration::QUANTUM,
                },
            }),
            Request::Stop(Stop {
                name: "enough".parse()?,
            }),
            Request::List,
            Request::Shutdown,
        ];

        for request in requests {
            let serialized = request.serialize()?;
            let deserialized = Request::deserialize(&serialized)?;
            assert_eq!(deserialized, request);
        }

        Ok(())
    }

    #[test]
    fn test_errors_are_serializable_and_deserializable() -> anyhow::Result<()> {
        let errors = vec![
            DaemonError::SocketCreationError(io::Error::new(io::ErrorKind::Other, "one").into()),
            DaemonError::SocketConfigurationError(
                io::Error::new(io::ErrorKind::Other, "two").into(),
            ),
            DaemonError::CommunicationError(CommunicationError::SerializationError {
                message: "three".to_owned(),
            }),
            DaemonError::CommunicationError(CommunicationError::DeserializationError {
                message: "four".to_owned(),
            }),
            DaemonError::ShutdownRequestError,
            DaemonError::StartProcessError(io::Error::new(io::ErrorKind::Other, "five").into()),
            DaemonError::CheckProcessError(io::Error::new(io::ErrorKind::Other, "six").into()),
            DaemonError::StopProcessError {
                process_id: 7,
                inner: io::Error::new(io::ErrorKind::Other, "seven").into(),
            },
            DaemonError::TimeOut,
        ];

        for error in errors {
            let serialized = error
                .serialize()
                .context(format!("serializing {:?}", error))?;
            let deserialized = DaemonError::deserialize(&serialized)
                .context(format!("deserializing {:?}", error))?;
            assert_eq!(deserialized, error);
        }

        Ok(())
    }

    // This is a fairly complicated test case that uses Unix sockets to create
    // blocking I/O buffers that do not terminate until we ask them to.
    //
    // This allows us to verify that serialization and deserialization work
    // across network I/O.
    //
    // We have to start a separate thread for the server side, and tell it what
    // to do using an `mpsc` side-channel.
    #[test]
    fn test_serialization_across_io() -> anyhow::Result<()> {
        // Create a Unix socket, with a listener (server) and a stream (client).
        let socket_dir = tempfile::Builder::new()
            .prefix("sandcastles-test")
            .tempdir()?;
        let socket_path = socket_dir.path().join("socket");
        let server = UnixListener::bind(&socket_path)?;
        let mut client = UnixStream::connect(&socket_path)?;

        // Start a new thread that listens on the server socket and handles requests.
        let (server_sender, server_receiver) = std::sync::mpsc::channel::<simple_server::Comm>();
        let (client_sender, client_receiver) = std::sync::mpsc::channel::<simple_server::Comm>();
        let thread_handle =
            std::thread::spawn(|| simple_server::run(server, server_receiver, client_sender));

        // Send a request from client to server, and ensure the server receives it.
        let request = simple_server::TestValue { value: 7 };
        request.write_to(&mut client)?;
        server_sender.send(simple_server::Comm::Receive)?;
        assert_eq!(client_receiver.recv()?, simple_server::Comm::Send(request));

        // Send a response from server to client, and ensure the client receives it.
        let response = simple_server::TestValue { value: 1 };
        server_sender.send(simple_server::Comm::Send(response.clone()))?;
        assert_eq!(client_receiver.recv()?, simple_server::Comm::Receive);
        assert_eq!(simple_server::TestValue::read_from(&mut client)?, response);

        // Shut down the server.
        server_sender.send(simple_server::Comm::Shutdown)?;
        assert_eq!(client_receiver.recv()?, simple_server::Comm::Shutdown);
        thread_handle.join().unwrap()?;

        Ok(())
    }

    pub(crate) mod simple_server {
        use crate::communication::Ship;
        use std::os::unix::net::UnixListener;
        use std::sync::mpsc;

        pub fn run(
            server: UnixListener,
            server_receiver: mpsc::Receiver<Comm>,
            client_sender: mpsc::Sender<Comm>,
        ) -> anyhow::Result<()> {
            let (mut stream, _) = server.accept()?;
            loop {
                match server_receiver.recv()? {
                    Comm::Receive => {
                        let response = TestValue::read_from(&mut stream)?;
                        client_sender.send(Comm::Send(response))?;
                    }
                    Comm::Send(value) => {
                        value.write_to(&mut stream)?;
                        client_sender.send(Comm::Receive)?;
                    }
                    Comm::Shutdown => {
                        client_sender.send(Comm::Shutdown)?;
                        break;
                    }
                }
            }
            Ok(())
        }

        #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        pub struct TestValue {
            pub value: i32,
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum Comm {
            Receive,
            Send(TestValue),
            Shutdown,
        }
    }
}
