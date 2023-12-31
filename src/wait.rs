use std::time::Instant;

use crate::error::{DaemonError, DaemonResult};
use crate::ports::Port;
use crate::timing::Duration;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WaitFor {
    AMoment,
    Time { duration: Duration },
    Port { port: Port },
}

impl WaitFor {
    pub(crate) fn block_until_ready(&self, timeout: Duration) -> DaemonResult<()> {
        match self {
            Self::AMoment => {
                Duration::QUANTUM.sleep();
                Ok(())
            }
            Self::Time { duration } => {
                if *duration >= timeout {
                    return Err(DaemonError::TimeOut);
                }
                duration.sleep();
                Ok(())
            }
            Self::Port { port } => {
                let start_time = Instant::now();
                while port.is_available() {
                    Duration::QUANTUM.sleep();
                    if Instant::now() - start_time > timeout.into() {
                        return Err(DaemonError::TimeOut);
                    }
                }
                Ok(())
            }
        }
    }
}

impl std::fmt::Display for WaitFor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WaitFor::AMoment => write!(f, "a moment"),
            WaitFor::Time { duration } => write!(f, "{}", duration),
            WaitFor::Port { port } => write!(f, "port {}", port),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net;
    use std::thread;
    use std::time::Instant;

    use crate::timing::DurationUnit;

    use super::*;

    #[test]
    fn test_wait_a_moment() -> anyhow::Result<()> {
        let start_time = Instant::now();
        WaitFor::AMoment.block_until_ready(Duration::ZERO)?;
        let end_time = Instant::now();

        let elapsed = end_time - start_time;
        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "Expected the elapsed time of {:?} to be a very short amount of time.",
            elapsed
        );
        Ok(())
    }

    #[test]
    fn test_wait_for_time() -> anyhow::Result<()> {
        let wait = WaitFor::Time {
            duration: Duration::of(1, DurationUnit::Seconds),
        };

        let start_time = Instant::now();
        wait.block_until_ready(Duration::of(2, DurationUnit::Seconds))?;
        let end_time = Instant::now();

        let elapsed = end_time - start_time;
        assert!(
            elapsed > std::time::Duration::from_millis(750)
                && elapsed <= std::time::Duration::from_millis(1500),
            "Expected the elapsed time of {:?} to be approximately 1s.",
            elapsed
        );
        Ok(())
    }

    #[test]
    fn test_time_out_waiting_for_time() -> anyhow::Result<()> {
        let wait = WaitFor::Time {
            duration: Duration::of(1, DurationUnit::Seconds),
        };

        let actual = wait.block_until_ready(Duration::of(100, DurationUnit::Milliseconds));

        assert!(actual.is_err(), "Expected an error but got {:?}", actual);
        Ok(())
    }

    #[test]
    fn test_wait_for_port() -> anyhow::Result<()> {
        let port = Port::next_available()?;
        let wait = WaitFor::Port { port };

        thread::spawn(move || {
            let socket_address = net::SocketAddrV6::new(net::Ipv6Addr::LOCALHOST, port.0, 0, 0);
            let listener = net::TcpListener::bind(socket_address).unwrap();
            listener.accept().unwrap(); // block until we receive a connection
        });

        wait.block_until_ready(Duration::of(1, DurationUnit::Seconds))?;

        Ok(())
    }

    #[test]
    fn test_time_out_waiting_for_port() -> anyhow::Result<()> {
        let port = Port::next_available()?;
        if port.is_in_use() {
            panic!("Port {} is supposed to be available but is in use.", port);
        }
        let wait = WaitFor::Port { port };

        let actual = wait.block_until_ready(Duration::of(100, DurationUnit::Milliseconds));

        assert!(actual.is_err(), "Expected an error but got {:?}", actual);
        Ok(())
    }
}
