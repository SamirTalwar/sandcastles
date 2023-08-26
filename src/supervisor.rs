use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Context;

use crate::services::*;
use crate::wait::WaitFor;

#[derive(Clone)]
pub struct Supervisor(Arc<Mutex<RunningServices>>);

impl Default for Supervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl Supervisor {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(RunningServices::new())))
    }

    pub fn start(&self, service: Service, wait: WaitFor) -> anyhow::Result<()> {
        let running = service.start().context("Failed to start a service")?;
        let mut inner = self.0.lock().unwrap();
        inner.add(running);
        wait.block_until_ready(Duration::MAX)?; // we need to pick a global timeout here
        Ok(())
    }

    pub fn stop_all(&self) -> anyhow::Result<()> {
        self.0
            .lock()
            .unwrap()
            .stop_all()
            .context("Failed to stop running services")
    }
}

struct RunningServices(Vec<RunningService>);

impl RunningServices {
    const TIMEOUT: Duration = Duration::from_secs(10);

    fn new() -> Self {
        Self(Vec::new())
    }

    fn add(&mut self, service: RunningService) {
        self.0.push(service);
    }

    fn stop_all(&mut self) -> anyhow::Result<()> {
        self.0
            .drain(..)
            .map(|mut service| service.stop(Self::TIMEOUT))
            .collect::<Vec<anyhow::Result<()>>>()
            .into_iter()
            .collect::<anyhow::Result<()>>()
    }
}

impl Drop for RunningServices {
    fn drop(&mut self) {
        self.stop_all().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::test_services;
    use crate::wait::WaitFor;

    use super::*;

    #[test]
    fn test_single_service() -> anyhow::Result<()> {
        let supervisor = Supervisor::new();
        supervisor.start(
            test_services::http_hello_world(),
            WaitFor::Time(Duration::from_millis(100)),
        )?;

        let response_body = reqwest::blocking::get("http://localhost:8080/")?.text()?;

        assert_eq!(response_body, "Hello, world!");

        Ok(())
    }
}
