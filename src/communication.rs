use std::io;

use crate::error::{CommunicationError, CommunicationResult, DaemonError};
use crate::services::Service;
use crate::wait::WaitFor;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum Request {
    Start(Start),
    Stop(Stop),
    Shutdown,
}

pub trait Response: Ship {}

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
pub(crate) enum ShutdownResponse {
    Success,
}

impl Response for ShutdownResponse {}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Name(String);

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for Name {
    type Err = (); // should be `!` but that's experimental

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

impl From<&str> for Name {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for Name {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ExitStatus {
    None,
    ExitedWithCode(u8),
}

impl From<ExitStatus> for std::process::ExitCode {
    fn from(value: ExitStatus) -> Self {
        match value {
            ExitStatus::None => Self::SUCCESS,
            ExitStatus::ExitedWithCode(code) => Self::from(code),
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
        rmp_serde::decode::from_read(reader).map_err(|error| {
            CommunicationError::DeserializationError {
                message: error.to_string(),
            }
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

    use crate::services::programs::Program;
    use crate::timing::Duration;

    use super::*;

    #[test]
    fn test_requests_are_serializable_and_deserializable() -> anyhow::Result<()> {
        let requests = vec![
            Request::Start(Start {
                name: Some("hello".into()),
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
            let serialized = error.serialize()?;
            let deserialized = DaemonError::deserialize(&serialized)?;
            assert_eq!(deserialized, error);
        }

        Ok(())
    }
}
