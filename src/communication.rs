use std::io;

use crate::error::{CommunicationError, CommunicationResult, DaemonError};
use crate::services::Service;
use crate::wait::WaitFor;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum Request {
    Start(Start),
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum Response {
    Success,
    Failure(DaemonError),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Start {
    pub service: Service,
    pub wait: WaitFor,
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
    fn test_successful_responses_are_serializable_and_deserializable() -> anyhow::Result<()> {
        let response = Response::Success;

        let serialized = response.serialize()?;
        let deserialized = Response::deserialize(&serialized)?;
        assert_eq!(deserialized, response);

        Ok(())
    }

    #[test]
    fn test_failure_responses_are_serializable_and_deserializable() -> anyhow::Result<()> {
        let responses = vec![
            Response::Failure(DaemonError::SocketCreationError(
                io::Error::new(io::ErrorKind::Other, "one").into(),
            )),
            Response::Failure(DaemonError::SocketConfigurationError(
                io::Error::new(io::ErrorKind::Other, "two").into(),
            )),
            Response::Failure(DaemonError::CommunicationError(
                CommunicationError::SerializationError {
                    message: "three".to_owned(),
                },
            )),
            Response::Failure(DaemonError::CommunicationError(
                CommunicationError::DeserializationError {
                    message: "four".to_owned(),
                },
            )),
            Response::Failure(DaemonError::ShutdownRequestError),
            Response::Failure(DaemonError::StartProcessError(
                io::Error::new(io::ErrorKind::Other, "five").into(),
            )),
            Response::Failure(DaemonError::CheckProcessError(
                io::Error::new(io::ErrorKind::Other, "six").into(),
            )),
            Response::Failure(DaemonError::StopProcessError {
                process_id: 7,
                inner: io::Error::new(io::ErrorKind::Other, "seven").into(),
            }),
            Response::Failure(DaemonError::TimeOut),
        ];

        for response in responses {
            let serialized = response.serialize()?;
            let deserialized = Response::deserialize(&serialized)?;
            assert_eq!(deserialized, response);
        }

        Ok(())
    }
}
