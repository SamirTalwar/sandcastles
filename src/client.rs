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

    pub fn start(&mut self, instruction: Start) -> ClientResult<Name> {
        self.send(&Request::Start(instruction))
            .and_then(|response| match response {
                StartResponse::Success(name) => Ok(name),
                StartResponse::Failure(error) => Err(ClientError::DaemonError(error)),
            })
    }

    pub fn stop(&mut self, instruction: Stop) -> ClientResult<ExitStatus> {
        self.send(&Request::Stop(instruction))
            .and_then(|response| match response {
                StopResponse::Success(exit_status) => Ok(exit_status),
                StopResponse::Failure(error) => Err(ClientError::DaemonError(error)),
            })
    }

    pub fn shutdown(&mut self) -> ClientResult<()> {
        self.send(&Request::Shutdown)
            .map(|response| match response {
                ShutdownResponse::Success => (),
            })
    }

    fn send<R: Response + serde::Serialize>(&mut self, request: &Request) -> ClientResult<R> {
        log::debug!(request);
        request
            .write_to(&mut self.socket)
            .map_err(ClientError::CommunicationError)?;
        let response = R::read_from(&mut self.socket).map_err(ClientError::CommunicationError)?;
        log::debug!(response);
        Ok(response)
    }
}
