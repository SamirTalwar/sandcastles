use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command};
use std::time::Instant;

use bstr::{ByteSlice, ByteVec};

use crate::error::{DaemonError, DaemonResult};
use crate::timing::Duration;
use crate::ExitStatus;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Argument(OsString);

impl serde::Serialize for Argument {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // This actually adds an extra, pointless layer of escaping, but it's
        // better than replacing unknown characters with \uFFFD.
        serializer.serialize_str(
            &<[u8]>::from_os_str(&self.0)
                .expect("Could not encode the argument.")
                .escape_bytes()
                .collect::<String>(),
        )
    }
}

impl<'de> serde::Deserialize<'de> for Argument {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // This reverses the extra layer of escaping above.
        let result = String::deserialize(deserializer)?;
        Ok(Self(
            Vec::<u8>::unescape_bytes(result)
                .into_os_string()
                .expect("Could not decode the argument."),
        ))
    }
}

impl AsRef<std::ffi::OsStr> for Argument {
    fn as_ref(&self) -> &std::ffi::OsStr {
        &self.0
    }
}

impl From<&OsStr> for Argument {
    fn from(value: &OsStr) -> Self {
        Self(value.to_owned())
    }
}

impl From<OsString> for Argument {
    fn from(value: OsString) -> Self {
        Self(value)
    }
}

impl From<&str> for Argument {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<String> for Argument {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl From<&std::path::Path> for Argument {
    fn from(value: &std::path::Path) -> Self {
        Self(value.into())
    }
}

impl From<std::path::PathBuf> for Argument {
    fn from(value: std::path::PathBuf) -> Self {
        Self(value.into())
    }
}

pub type Environment = BTreeMap<Argument, Argument>;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Program {
    pub command: Argument,
    pub arguments: Vec<Argument>,
    pub environment: Environment,
}

pub struct RunningProgram {
    process: Child,
}

impl Program {
    pub(crate) fn start(&self) -> DaemonResult<RunningProgram> {
        let process = Command::new(&self.command)
            .args(&self.arguments)
            .envs(&self.environment)
            .spawn()
            .map_err(|error| DaemonError::StartProcessError(error.into()))?;
        Ok(RunningProgram { process })
    }
}

impl RunningProgram {
    pub(crate) fn is_running(&mut self) -> DaemonResult<bool> {
        let exit_code = self
            .process
            .try_wait()
            .map_err(|error| DaemonError::CheckProcessError(error.into()))?;
        Ok(exit_code.is_none())
    }

    pub(crate) fn stop(&mut self, timeout: Duration) -> DaemonResult<ExitStatus> {
        let timeout_sys = std::time::Duration::from(timeout);
        self.kill(nix::sys::signal::Signal::SIGTERM)?;
        let sigterm_time = Instant::now();
        loop {
            if let Ok(Some(exit_status)) = self.process.try_wait() {
                return Ok(match exit_status.code() {
                    None => match exit_status.signal() {
                        None => ExitStatus::None,
                        Some(signal) => match u8::try_from(signal).ok() {
                            None => ExitStatus::None,
                            Some(signal) => ExitStatus::ExitedWithSignal(signal),
                        },
                    },
                    Some(code) => match u8::try_from(code).ok() {
                        None => ExitStatus::None,
                        Some(code) => ExitStatus::ExitedWithCode(code),
                    },
                });
            }
            if Instant::now() - sigterm_time > timeout_sys {
                self.kill(nix::sys::signal::Signal::SIGKILL)?;
            }
            Duration::QUANTUM.sleep();
        }
    }

    fn kill(&self, signal: nix::sys::signal::Signal) -> DaemonResult<()> {
        let unwrapped_process_id = self.process.id();
        let process_id = nix::unistd::Pid::from_raw(
            unwrapped_process_id
                .try_into()
                .expect("Could not convert a process ID."),
        );
        match nix::sys::signal::kill(process_id, signal) {
            Ok(()) => Ok(()),
            Err(nix::errno::Errno::ESRCH) => Ok(()), // the process was already stopped
            Err(error) => Err(DaemonError::StopProcessError {
                process_id: unwrapped_process_id,
                inner: std::io::Error::from_raw_os_error(error as i32).into(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::prelude::OsStrExt;

    use crate::test_helpers::*;
    use crate::test_programs;
    use crate::timing::{Duration, DurationUnit};

    use super::*;

    #[test]
    #[ntest::timeout(2000)]
    fn test_starting_and_stopping() -> DaemonResult<()> {
        let program = test_programs::waits_for_termination();
        let mut running_program = program.start()?;

        Duration::QUANTUM.sleep();
        assert!(
            running_program.is_running()?,
            "The process stopped abruptly."
        );

        let exit_status = running_program.stop(Duration::of(5, DurationUnit::Seconds))?;

        assert!(
            !running_program.is_running()?,
            "Expected the process to have stopped."
        );
        assert_eq!(exit_status, ExitStatus::ExitedWithCode(0));
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
        assert!(
            running_program.is_running()?,
            "The process stopped abruptly."
        );

        let exit_status = running_program.stop(Duration::of(1, DurationUnit::Seconds))?;

        assert!(
            !running_program.is_running()?,
            "Expected the process to have stopped."
        );
        assert_eq!(exit_status, ExitStatus::ExitedWithSignal(9));
        Ok(())
    }

    #[test]
    #[ntest::timeout(2000)]
    fn test_stopping_a_stopped_process() -> anyhow::Result<()> {
        let program = Program {
            command: "true".into(),
            arguments: Default::default(),
            environment: Default::default(),
        };
        let mut running_program = program.start()?;

        Duration::QUANTUM.sleep();
        assert!(
            !running_program.is_running()?,
            "The process should have stopped."
        );

        let exit_status = running_program.stop(Duration::of(1, DurationUnit::Seconds))?;

        assert!(
            !running_program.is_running()?,
            "Expected the process to still be stopped."
        );
        assert_eq!(exit_status, ExitStatus::ExitedWithCode(0));
        Ok(())
    }

    #[test]
    fn test_serializing_an_argument() -> anyhow::Result<()> {
        let argument = Argument::from(OsStr::from_bytes(b"/path/to\x01/command"));

        let serialized = serde_json::to_string(&argument)?;

        assert_eq!(serialized, "\"/path/to\\\\x01/command\"");
        Ok(())
    }

    #[test]
    fn test_deserializing_an_argument() -> anyhow::Result<()> {
        let serialized = "\"wibble.\\\\xFF.wobble\"";

        let deserialized: Argument = serde_json::from_str(serialized)?;

        assert_eq!(
            deserialized,
            Argument::from(OsStr::from_bytes(b"wibble.\xFF.wobble"))
        );
        Ok(())
    }
}
