use std::error::Error;
use std::fmt::Display;

use crate::log::LoggableIoError;

pub type ClientResult<A> = std::result::Result<A, ClientError>;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "code")]
pub enum ClientError {
    SocketConnectionError(LoggableIoError),
    CommunicationError(CommunicationError),
    DaemonError(DaemonError),
}

impl Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SocketConnectionError(inner) => write!(f, "socket connection error: {}", inner),
            Self::CommunicationError(inner) => write!(f, "{}", inner),
            Self::DaemonError(inner) => {
                write!(f, "daemon error: {}", inner)
            }
        }
    }
}

impl Error for ClientError {}

pub type DaemonResult<A> = std::result::Result<A, DaemonError>;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "code")]
pub enum DaemonError {
    SocketCreationError(LoggableIoError),
    SocketConfigurationError(LoggableIoError),
    CommunicationError(CommunicationError),
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
            Self::CommunicationError(inner) => write!(f, "{}", inner),
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

pub type CommunicationResult<A> = Result<A, CommunicationError>;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "code")]
pub enum CommunicationError {
    SerializationError(String),
    DeserializationError(String),
}

impl Display for CommunicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationError(inner) => {
                write!(f, "serialization error: {}", inner)
            }
            Self::DeserializationError(inner) => {
                write!(f, "deserialization error: {}", inner)
            }
        }
    }
}

impl Error for CommunicationError {}
