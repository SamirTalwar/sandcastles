pub mod programs;

pub use programs::*;

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
    pub(crate) fn stop(&mut self) -> anyhow::Result<()> {
        match self {
            Self::Program(p) => p.stop(),
        }
    }
}
