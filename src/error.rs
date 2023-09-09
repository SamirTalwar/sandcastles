use std::error::Error;
use std::fmt::Display;

use crate::log::LoggableIoError;

pub type DaemonResult<A> = std::result::Result<A, DaemonError>;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "code")]
pub enum DaemonError {
    SocketCreationError(LoggableIoError),
    SocketConfigurationError(LoggableIoError),
    RequestDeserializationError(String),
    ResponseSerializationError(String),
    ShutdownRequestError,
    StartProcessError(LoggableIoError),
    CheckProcessError(LoggableIoError),
    StopProcessError {
        process_id: u32,
        #[serde(flatten)]
        inner: LoggableIoError,
    },
    TimeOut(crate::WaitFor),
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
            Self::ShutdownRequestError => write!(f, "shutdown request error"),
            Self::StartProcessError(inner) => write!(f, "start process error: {}", inner),
            Self::CheckProcessError(inner) => write!(f, "check process error: {}", inner),
            Self::StopProcessError { process_id, inner } => {
                write!(f, "stop process error (id: {}): {}", process_id, inner)
            }
            Self::TimeOut(wait) => write!(f, "timed out waiting for {}", wait),
        }
    }
}

impl Error for DaemonError {}
