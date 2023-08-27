pub mod programs;

use std::time::Duration;

pub use programs::*;

use crate::wait::WaitFor;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Start {
    pub service: Service,
    pub wait: WaitFor,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Service {
    Program(Program),
}

impl Service {
    pub(crate) fn start(&self) -> anyhow::Result<RunningService> {
        match self {
            Self::Program(p) => p.start().map(RunningService::Program),
        }
    }
}

pub(crate) enum RunningService {
    Program(RunningProgram),
}

impl RunningService {
    pub(crate) fn stop(&mut self, timeout: Duration) -> anyhow::Result<()> {
        match self {
            Self::Program(p) => p.stop(timeout),
        }
    }
}
