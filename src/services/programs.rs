use std::ffi::OsString;
use std::process::{Child, Command};
use std::time::Instant;

use anyhow::Context;

use crate::timing::Duration;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Program {
    pub command: OsString,
    pub arguments: Vec<OsString>,
}

pub struct RunningProgram {
    process: Child,
}

impl Program {
    pub(crate) fn start(&self) -> anyhow::Result<RunningProgram> {
        let process = Command::new(&self.command).args(&self.arguments).spawn()?;
        Ok(RunningProgram { process })
    }
}

impl RunningProgram {
    pub(crate) fn stop(&mut self, timeout: Duration) -> anyhow::Result<()> {
        let process_id = nix::unistd::Pid::from_raw(self.process.id().try_into()?);
        nix::sys::signal::kill(process_id, nix::sys::signal::Signal::SIGTERM)
            .context(format!("Failed to stop the process with ID {}", process_id))?;
        let sigterm_time = Instant::now();
        while !matches!(self.process.try_wait(), Ok(Some(_))) {
            if Instant::now() - sigterm_time > timeout.into() {
                nix::sys::signal::kill(process_id, nix::sys::signal::Signal::SIGKILL)
                    .context(format!("Failed to kill the process with ID {}", process_id))?;
            }
            Duration::QUANTUM.sleep();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test_programs;
    use crate::timing::{Duration, DurationUnit};

    #[test]
    #[ntest::timeout(2000)]
    fn test_starting_and_stopping() -> anyhow::Result<()> {
        let program = test_programs::waits_for_termination();
        let mut running_program = program.start()?;

        Duration::QUANTUM.sleep();
        let exit_code_before_stop = running_program.process.try_wait()?;
        assert_eq!(exit_code_before_stop, None);

        running_program.stop(Duration::of(5, DurationUnit::Seconds))?;

        let exit_code_after_stop = running_program.process.try_wait()?;
        if exit_code_after_stop.is_none() {
            panic!("Expected the process to have stopped.");
        }
        Ok(())
    }

    #[test]
    #[ntest::timeout(2000)]
    fn test_killing() -> anyhow::Result<()> {
        let program = test_programs::ignores_termination();
        let mut running_program = program.start()?;

        Duration::QUANTUM.sleep();
        let exit_code_before_stop = running_program.process.try_wait()?;
        assert_eq!(exit_code_before_stop, None);

        running_program.stop(Duration::of(1, DurationUnit::Seconds))?;

        let exit_code_after_stop = running_program.process.try_wait()?;
        if exit_code_after_stop.is_none() {
            panic!("Expected the process to have stopped.");
        }
        Ok(())
    }
}
