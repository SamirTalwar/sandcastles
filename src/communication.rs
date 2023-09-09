use crate::error::DaemonError;
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
