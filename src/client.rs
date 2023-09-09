use std::os::unix::net::UnixStream;
use std::path::Path;

use crate::communication::*;
use crate::error::{ClientError, ClientResult};
use crate::log;

pub struct Client {
    socket: UnixStream,
}

impl Client {
    pub fn connect_to(socket_path: &Path) -> ClientResult<Self> {
        log::debug!(socket = socket_path);
        let socket = UnixStream::connect(socket_path)
            .map_err(|error| ClientError::SocketConnectionError(error.into()))?;
        Ok(Client { socket })
    }

    pub fn start(&mut self, instruction: Start) -> ClientResult<()> {
        self.send(&Request::Start(instruction))
    }

    pub fn shutdown(&mut self) -> ClientResult<()> {
        self.send(&Request::Shutdown)
    }

    fn send(&mut self, request: &Request) -> ClientResult<()> {
        log::debug!(request);
        bincode::serialize_into(&mut self.socket, request)
            .map_err(|error| ClientError::RequestSerializationError(error.to_string()))?;
        let response = bincode::deserialize_from(&mut self.socket)
            .map_err(|error| ClientError::ResponseDeserializationError(error.to_string()))?;
        log::debug!(response);
        match response {
            Response::Success => Ok(()),
            Response::Failure(error) => Err(ClientError::DaemonError(error)),
        }
    }
}
