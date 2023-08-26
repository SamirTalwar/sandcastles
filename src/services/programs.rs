use std::ffi::OsString;
use std::process::Child;
use std::process::Command;

use anyhow::Context;

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
    pub(crate) fn stop(&mut self) -> anyhow::Result<()> {
        let process_id = self.process.id();
        nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(process_id.try_into()?),
            nix::sys::signal::Signal::SIGTERM,
        )
        .context(format!("Failed to stop the process with ID {}", process_id))?;
        self.process.wait().context(format!(
            "Failed to wait for the process with ID {} to stop.",
            process_id
        ))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::services::Service;
    use crate::test_services;

    #[test]
    fn test_starting_and_stopping() -> anyhow::Result<()> {
        let Service::Program(program) = test_services::http_hello_world();
        let mut running_program = program.start()?;
        let exit_code_before_stop = running_program.process.try_wait()?;
        assert_eq!(exit_code_before_stop, None);

        running_program.stop()?;

        let exit_code_after_stop = running_program.process.try_wait()?;
        if exit_code_after_stop.is_none() {
            panic!("Expected the process to have stopped.");
        }
        Ok(())
    }
}
