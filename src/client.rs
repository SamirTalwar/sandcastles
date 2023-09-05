use std::os::unix::net::UnixStream;
use std::path::Path;

use anyhow::Context;

use crate::communication::*;
use crate::log;

pub struct Client {
    socket: UnixStream,
}

impl Client {
    pub fn connect_to(socket_path: &Path) -> anyhow::Result<Self> {
        log::debug!(socket = socket_path);
        let socket =
            UnixStream::connect(socket_path).context("Could not connect to the daemon socket")?;
        Ok(Client { socket })
    }

    pub fn start(&mut self, instruction: Start) -> anyhow::Result<()> {
        self.send(&Request::Start(instruction))
    }

    pub fn shutdown(&mut self) -> Result<(), anyhow::Error> {
        self.send(&Request::Shutdown)
    }

    fn send(&mut self, request: &Request) -> Result<(), anyhow::Error> {
        log::debug!(request = request);
        bincode::serialize_into(&mut self.socket, request)
            .context("Could not serialize the request")?;
        let response = bincode::deserialize_from(&mut self.socket)
            .context("Could not deserialize the response")?;
        log::debug!(response = response);
        match response {
            Response::Success => Ok(()),
            Response::Failure(message) => anyhow::bail!(message),
        }
    }
}
