use std::thread;
use std::time::Duration;

use crate::ports::Port;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum WaitFor {
    None,
    Time(Duration),
    Port(Port),
}

impl WaitFor {
    pub(crate) fn block_until_ready(&self) -> anyhow::Result<()> {
        match self {
            Self::None => Ok(()),
            Self::Time(duration) => {
                thread::sleep(*duration);
                Ok(())
            }
            Self::Port(port) => {
                while port.is_available() {
                    thread::yield_now();
                }
                Ok(())
            }
        }
    }
}
