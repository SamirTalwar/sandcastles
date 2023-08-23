use std::sync::{Arc, Mutex};

use anyhow::Context;

use crate::services::*;

#[derive(Clone)]
pub struct Supervisor(Arc<Mutex<Vec<RunningService>>>);

impl Default for Supervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl Supervisor {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }

    pub fn start(&self, service: Service, wait: WaitFor) -> anyhow::Result<()> {
        let running = service.start().context("Failed to start a service")?;
        let mut inner = self.0.lock().unwrap();
        inner.push(running);
        wait.block_until_ready()?;
        Ok(())
    }

    pub fn stop_all(&self) -> anyhow::Result<()> {
        let mut inner = self.0.lock().unwrap();
        inner
            .drain(..)
            .map(|mut service| service.stop())
            .collect::<Vec<anyhow::Result<()>>>()
            .into_iter()
            .collect::<anyhow::Result<()>>()
    }
}
