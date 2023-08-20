use std::ffi::OsString;
use std::process::Child;
use std::process::Command;
use std::thread;

use anyhow::Context;

use crate::ports::Port;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Service {
    Program(Program),
}

impl Service {
    pub(crate) fn start(&self) -> anyhow::Result<RunningService> {
        match self {
            Self::Program(Program { command, arguments }) => {
                let process = Command::new(command).args(arguments).spawn()?;
                Ok(RunningService::Program(process))
            }
        }
    }
}

pub(crate) enum RunningService {
    Program(Child),
}

impl RunningService {
    pub(crate) fn stop(&mut self) -> anyhow::Result<()> {
        match self {
            Self::Program(process) => {
                let process_id = process.id();
                nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(process_id.try_into()?),
                    nix::sys::signal::Signal::SIGTERM,
                )
                .context(format!("Failed to stop the process with ID {}", process_id))?;
                process.wait().context(format!(
                    "Failed to wait for the process with ID {} to stop.",
                    process_id
                ))?;
                Ok(())
            }
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Program {
    pub command: OsString,
    pub arguments: Vec<OsString>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum WaitFor {
    Port(Port),
}

impl WaitFor {
    pub(crate) fn block_until_ready(&self) -> anyhow::Result<()> {
        match self {
            Self::Port(port) => {
                while port.is_available() {
                    thread::yield_now();
                }
                Ok(())
            }
        }
    }
}
