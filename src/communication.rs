use crate::services::Service;
use crate::wait::WaitFor;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) enum Request {
    Start(Start),
    Shutdown,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) enum Response {
    Success,
    Failure(String),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Start {
    pub service: Service,
    pub wait: WaitFor,
}
