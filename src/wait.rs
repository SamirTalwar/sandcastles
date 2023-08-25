use std::thread;
use std::time::{Duration, Instant};

use crate::ports::Port;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum WaitFor {
    None,
    Time(Duration),
    Port(Port),
}

impl WaitFor {
    pub(crate) fn block_until_ready(&self, timeout: Duration) -> anyhow::Result<()> {
        match self {
            Self::None => Ok(()),
            Self::Time(duration) => {
                if *duration >= timeout {
                    anyhow::bail!("Timed out waiting for {:?}.", duration);
                }
                thread::sleep(*duration);
                Ok(())
            }
            Self::Port(port) => {
                let start_time = Instant::now();
                while port.is_available() {
                    thread::sleep(Duration::from_millis(100));
                    if Instant::now() - start_time > timeout {
                        anyhow::bail!("Timed out waiting for port {}.", port);
                    }
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net;
    use std::time::Instant;

    use super::*;

    #[test]
    fn test_no_wait() -> anyhow::Result<()> {
        WaitFor::None.block_until_ready(Duration::ZERO)?;

        Ok(())
    }

    #[test]
    fn test_wait_for_time() -> anyhow::Result<()> {
        let wait = WaitFor::Time(Duration::from_secs(1));

        let start_time = Instant::now();
        wait.block_until_ready(Duration::from_secs(2))?;
        let end_time = Instant::now();

        let elapsed = end_time - start_time;
        assert!(
            elapsed > Duration::from_millis(750) && elapsed <= Duration::from_millis(1500),
            "Expected the elapsed time of {:?} to be approximately 1s.",
            elapsed
        );
        Ok(())
    }

    #[test]
    fn test_time_out_waiting_for_time() -> anyhow::Result<()> {
        let wait = WaitFor::Time(Duration::from_secs(1));

        let actual = wait.block_until_ready(Duration::from_millis(100));

        assert!(actual.is_err(), "Expected an error but got {:?}", actual);
        Ok(())
    }

    #[test]
    fn test_wait_for_port() -> anyhow::Result<()> {
        let port = Port::next_available()?;
        let wait = WaitFor::Port(port);

        thread::spawn(move || {
            let socket_address = net::SocketAddrV4::new(net::Ipv4Addr::UNSPECIFIED, port.0);
            let listener = net::TcpListener::bind(socket_address).unwrap();
            listener.accept().unwrap(); // block until we receive a connection
        });

        wait.block_until_ready(Duration::from_secs(1))?;

        Ok(())
    }

    #[test]
    fn test_time_out_waiting_for_port() -> anyhow::Result<()> {
        let port = Port::next_available()?;
        if port.is_in_use() {
            panic!("Port {} is supposed to be available but is in use.", port);
        }
        let wait = WaitFor::Port(port);

        let actual = wait.block_until_ready(Duration::from_millis(100));

        assert!(actual.is_err(), "Expected an error but got {:?}", actual);
        Ok(())
    }
}
