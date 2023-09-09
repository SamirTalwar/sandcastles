pub mod programs;

pub use programs::*;

use crate::error::DaemonResult;
use crate::timing::Duration;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Service {
    Program(Program),
}

impl Service {
    pub(crate) fn start(&self) -> DaemonResult<RunningService> {
        match self {
            Self::Program(p) => p.start().map(RunningService::Program),
        }
    }
}

pub(crate) enum RunningService {
    Program(RunningProgram),
}

impl RunningService {
    pub(crate) fn stop(&mut self, timeout: Duration) -> DaemonResult<()> {
        match self {
            Self::Program(p) => p.stop(timeout),
        }
    }
}
