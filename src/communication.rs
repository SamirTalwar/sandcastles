use crate::services::Start;

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
