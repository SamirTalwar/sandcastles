use crate::services::Service;
use crate::WaitFor;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) enum Request {
    Start { service: Service, wait: WaitFor },
    Shutdown,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) enum Response {
    Success,
    Failure(String),
}
