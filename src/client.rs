use std::os::unix::net::UnixStream;
use std::path::Path;

use anyhow::Context;

use crate::communication::{Request, Response};
use crate::services::Start;

pub struct Client {
    socket: UnixStream,
}

impl Client {
    pub fn connect_to(socket: &Path) -> anyhow::Result<Self> {
        let socket =
            UnixStream::connect(socket).context("Could not connect to the daemon socket")?;
        Ok(Client { socket })
    }

    pub fn start(&mut self, instruction: Start) -> anyhow::Result<()> {
        self.send(&Request::Start(instruction))
    }

    pub fn shutdown(&mut self) -> Result<(), anyhow::Error> {
        self.send(&Request::Shutdown)
    }

    fn send(&mut self, request: &Request) -> Result<(), anyhow::Error> {
        bincode::serialize_into(&mut self.socket, request)
            .context("Could not serialize the request")?;
        let response = bincode::deserialize_from(&mut self.socket)
            .context("Could not deserialize the response")?;
        match response {
            Response::Success => Ok(()),
            Response::Failure(message) => anyhow::bail!(message),
        }
    }
}
