use std::error::Error;
use std::fmt::Display;

use crate::log::LoggableIoError;

pub type ClientResult<A> = std::result::Result<A, ClientError>;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
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
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
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
    TimeOut,
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
            Self::TimeOut => write!(f, "timed out"),
        }
    }
}

impl Error for DaemonError {}

pub type CommunicationResult<A> = Result<A, CommunicationError>;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommunicationError {
    SerializationError { message: String },
    DeserializationError { message: String },
}

impl Display for CommunicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationError { message } => {
                write!(f, "serialization error: {}", message)
            }
            Self::DeserializationError { message } => {
                write!(f, "deserialization error: {}", message)
            }
        }
    }
}

impl Error for CommunicationError {}
