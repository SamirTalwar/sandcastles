use std::collections::BTreeMap;
use std::process::{Child, Command};
use std::time::Instant;

use anyhow::Context;

use crate::timing::Duration;

#[derive(Debug, Clone)]
pub struct Argument {
    value: std::ffi::OsString,
    rendered: String,
}

impl PartialEq for Argument {
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value)
    }
}

impl Eq for Argument {}

impl PartialOrd for Argument {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl Ord for Argument {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl std::hash::Hash for Argument {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state)
    }
}

impl serde::Serialize for Argument {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Argument {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        std::ffi::OsString::deserialize(deserializer).map(Self::new)
    }
}

impl Argument {
    pub fn new(into_value: impl Into<std::ffi::OsString>) -> Self {
        let value = into_value.into();
        let rendered = value.to_string_lossy().to_string();
        Self { value, rendered }
    }
}

impl<Value: Into<std::ffi::OsString>> From<Value> for Argument {
    fn from(value: Value) -> Self {
        Self::new(value)
    }
}

impl AsRef<std::ffi::OsStr> for Argument {
    fn as_ref(&self) -> &std::ffi::OsStr {
        &self.value
    }
}

impl std::fmt::Display for Argument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.rendered.fmt(f)
    }
}

pub type Environment = BTreeMap<Argument, Argument>;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Program {
    pub command: Argument,
    pub arguments: Vec<Argument>,
    pub environment: Environment,
}

pub struct RunningProgram {
    process: Child,
}

impl Program {
    pub(crate) fn start(&self) -> anyhow::Result<RunningProgram> {
        let process = Command::new(&self.command)
            .args(&self.arguments)
            .envs(&self.environment)
            .spawn()?;
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
    use crate::test_helpers::*;
    use crate::test_programs;
    use crate::timing::{Duration, DurationUnit};

    use super::*;

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
    fn test_environment_variables() -> anyhow::Result<()> {
        let temporary_directory = tempfile::tempdir()?;
        let test_file = temporary_directory.path().join("test.file");

        let program = Program {
            command: "bash".into(),
            arguments: vec!["-c".into(), "echo $INPUT > $TEST_FILE".into()],
            environment: Environment::from([
                ("INPUT".into(), "hello there".into()),
                ("TEST_FILE".into(), test_file.clone().into()),
            ]),
        };
        program.start()?;

        eventually(|| {
            let output = std::fs::read_to_string(&test_file)?;
            test_eq(output.as_str(), "hello there\n")
        })
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
