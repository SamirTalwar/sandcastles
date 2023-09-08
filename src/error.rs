use std::error::Error;
use std::fmt::Display;

use crate::log::LoggableIoError;

pub type DaemonResult<A> = std::result::Result<A, DaemonError>;

#[serde_with::serde_as]
#[derive(Debug, serde::Serialize)]
#[serde(tag = "code")]
pub enum DaemonError {
    SocketCreationError(LoggableIoError),
    SocketConfigurationError(LoggableIoError),
    RequestDeserializationError(#[serde_as(as = "serde_with::DisplayFromStr")] bincode::ErrorKind),
    ResponseSerializationError(#[serde_as(as = "serde_with::DisplayFromStr")] bincode::ErrorKind),
    ShutdownRequestError(
        #[serde_as(as = "serde_with::DisplayFromStr")]
        std::sync::mpsc::SendError<std::os::unix::net::UnixStream>,
    ),
}

impl Display for DaemonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SocketCreationError(inner) => write!(f, "socket creation error: {}", inner),
            Self::SocketConfigurationError(inner) => {
                write!(f, "socket configuration error: {}", inner)
            }
            Self::RequestDeserializationError(inner) => {
                write!(f, "request deserialization error: {}", inner)
            }
            Self::ResponseSerializationError(inner) => {
                write!(f, "response serialization error: {}", inner)
            }
            Self::ShutdownRequestError(inner) => write!(f, "shutdown request error: {}", inner),
        }
    }
}

impl Error for DaemonError {}
