use std::io;

use crate::error::{CommunicationError, CommunicationResult, DaemonError};
use crate::services::Service;
use crate::wait::WaitFor;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum Request {
    Start(Start),
    Shutdown,
}

impl Ship for Request {}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum Response {
    Success,
    Failure(DaemonError),
}

impl Ship for Response {}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Start {
    pub service: Service,
    pub wait: WaitFor,
}

pub(crate) trait Ship: serde::Serialize + for<'de> serde::Deserialize<'de> + Sized {
    fn read_from(reader: impl io::Read) -> CommunicationResult<Self> {
        rmp_serde::decode::from_read(reader)
            .map_err(|error| CommunicationError::DeserializationError(error.to_string()))
    }

    fn deserialize(buffer: &[u8]) -> CommunicationResult<Self> {
        Self::read_from(buffer)
    }

    fn write_to(&self, mut writer: impl io::Write) -> CommunicationResult<()> {
        rmp_serde::encode::write(&mut writer, self)
            .map_err(|error| CommunicationError::SerializationError(error.to_string()))
    }

    fn serialize(&self) -> CommunicationResult<Vec<u8>> {
        let mut buffer = Vec::new();
        self.write_to(&mut buffer)?;
        Ok(buffer)
    }
}
