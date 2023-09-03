use std::sync::{Arc, Mutex};

use anyhow::Context;

use crate::services::*;
use crate::timing::Duration;

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

    pub fn start(&self, instruction: Start) -> anyhow::Result<()> {
        let running = instruction
            .service
            .start()
            .context("Failed to start a service")?;
        let mut inner = self.0.lock().unwrap();
        inner.add(running);
        instruction.wait.block_until_ready(Duration::FOREVER)?; // we need to pick a global timeout here
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
    fn new() -> Self {
        Self(Vec::new())
    }

    fn add(&mut self, service: RunningService) {
        self.0.push(service);
    }

    fn stop_all(&mut self) -> anyhow::Result<()> {
        self.0
            .drain(..)
            .map(|mut service| service.stop(Duration::STOP_TIMEOUT))
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
    use std::fs;

    use crate::ports::Port;
    use crate::test_services;
    use crate::wait::WaitFor;

    use super::*;

    #[test]
    fn test_starts_a_single_service() -> anyhow::Result<()> {
        let output_directory = tempfile::tempdir()?;
        let output_file = output_directory.path().join("timestamp.txt");

        let supervisor = Supervisor::new();
        supervisor.start(Start {
            service: test_services::file_watch(&output_file, vec!["echo".into(), "output".into()]),
            wait: WaitFor::None,
        })?;

        eventually(|| {
            let output = fs::read_to_string(&output_file)?;
            if output == "output\n" {
                Ok(())
            } else {
                anyhow::bail!("Incorrect output: {:?}", output)
            }
        })
    }

    #[test]
    fn test_starts_a_single_service_and_waits_for_a_port() -> anyhow::Result<()> {
        let supervisor = Supervisor::new();
        supervisor.start(Start {
            service: test_services::http_hello_world(),
            wait: WaitFor::Port(Port(8080)),
        })?;

        let response_body = reqwest::blocking::get("http://localhost:8080/")?.text()?;

        assert_eq!(response_body, "Hello, world!");

        Ok(())
    }

    fn eventually<A: std::fmt::Debug>(action: impl Fn() -> anyhow::Result<A>) -> anyhow::Result<A> {
        let start_time = std::time::Instant::now();
        loop {
            let result = action();
            match result {
                Ok(_) => {
                    return result;
                }
                Err(_) => {
                    // fail if we've taken too long, otherwise retry after a short delay
                    if std::time::Instant::now() - start_time >= std::time::Duration::from_secs(3) {
                        return result;
                    }
                }
            }
            Duration::QUANTUM.sleep();
        }
    }
}
