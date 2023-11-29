use thiserror::Error;

use crate::log::LoggableIoError;
use crate::names::Name;

pub type ClientResult<A> = std::result::Result<A, ClientError>;

#[derive(Debug, Clone, PartialEq, Eq, Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClientError {
    #[error("socket connection error: {0}")]
    SocketConnectionError(LoggableIoError),
    #[error("{0}")]
    CommunicationError(CommunicationError),
    #[error("daemon error: {0}")]
    DaemonError(DaemonError),
}

pub type DaemonResult<A> = std::result::Result<A, DaemonError>;

#[derive(Debug, Clone, PartialEq, Eq, Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DaemonError {
    #[error("socket creation error: {0}")]
    SocketCreationError(LoggableIoError),
    #[error("socket configuration error: {0}")]
    SocketConfigurationError(LoggableIoError),
    #[error("{0}")]
    CommunicationError(CommunicationError),
    #[error("shutdown request error")]
    ShutdownRequestError,
    #[error("no such service error (name: {name})")]
    NoSuchServiceError { name: Name },
    #[error("service already exists error (name: {name})")]
    ServiceAlreadyExistsError { name: Name },
    #[error("service crashed")]
    ServiceCrashedError,
    #[error("start process error: {0}")]
    StartProcessError(LoggableIoError),
    #[error("check process error: {0}")]
    CheckProcessError(LoggableIoError),
    #[error("stop process error (id: {process_id}): {inner}")]
    StopProcessError {
        process_id: u32,
        #[serde(flatten)]
        inner: LoggableIoError,
    },
    #[error("timed out")]
    TimeOut,
}

pub type CommunicationResult<A> = Result<A, CommunicationError>;

#[derive(Debug, Clone, PartialEq, Eq, Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommunicationError {
    #[error("serialization error: {message}")]
    SerializationError { message: String },
    #[error("deserialization error: {message}")]
    DeserializationError { message: String },
    #[error("connection terminated")]
    ConnectionTerminated,
}
