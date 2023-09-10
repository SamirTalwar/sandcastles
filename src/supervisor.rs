use std::sync::{Arc, Mutex};

use crate::communication::{Name, Start};
use crate::error::{DaemonError, DaemonResult};
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

    pub fn start(&self, instruction: &Start) -> DaemonResult<Name> {
        let running = instruction.service.start()?;
        let mut inner = self.0.lock().unwrap();
        let running = inner.add(running);
        instruction.wait.block_until_ready(Duration::FOREVER)?; // we need to pick a global timeout here
        if running.is_running()? {
            Ok(instruction.name.clone().unwrap_or_else(|| "".into()))
        } else {
            Err(DaemonError::ServiceCrashedError)
        }
    }

    pub fn stop_all(&self) -> DaemonResult<()> {
        self.0.lock().unwrap().stop_all()
    }
}

struct RunningServices(Vec<RunningService>);

impl RunningServices {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn add(&mut self, service: RunningService) -> &mut RunningService {
        self.0.push(service);
        self.0.last_mut().unwrap()
    }

    fn stop_all(&mut self) -> DaemonResult<()> {
        self.0
            .drain(..)
            .map(|mut service| service.stop(Duration::STOP_TIMEOUT))
            .collect::<Vec<DaemonResult<()>>>()
            .into_iter()
            .collect::<DaemonResult<()>>()
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
    use crate::test_helpers::*;
    use crate::test_services;
    use crate::wait::WaitFor;

    use super::*;

    #[test]
    fn test_starts_a_single_service() -> anyhow::Result<()> {
        let output_directory = tempfile::tempdir()?;
        let output_file = output_directory.path().join("timestamp.txt");

        let supervisor = Supervisor::new();
        supervisor.start(&Start {
            name: None,
            service: test_services::file_watch(&output_file, vec!["echo".into(), "output".into()]),
            wait: WaitFor::AMoment,
        })?;

        eventually(|| {
            let output = fs::read_to_string(&output_file)?;
            test_eq(output.as_str(), "output\n")
        })
    }

    #[test]
    fn test_starts_a_single_service_and_waits_for_a_port() -> anyhow::Result<()> {
        let service_port = Port::next_available()?;
        let supervisor = Supervisor::new();
        supervisor.start(&Start {
            name: None,
            service: test_services::http_hello_world(service_port),
            wait: WaitFor::Port { port: service_port },
        })?;

        let response_body =
            reqwest::blocking::get(format!("http://localhost:{}/", service_port))?.text()?;

        assert_eq!(response_body, "Hello, world!");
        Ok(())
    }

    #[test]
    fn test_starts_a_single_service_and_waits_a_little() -> anyhow::Result<()> {
        let supervisor = Supervisor::new();
        let result = supervisor.start(&Start {
            name: None,
            service: Service::Program(Program {
                command: "true".into(),
                arguments: Default::default(),
                environment: Default::default(),
            }),
            wait: WaitFor::AMoment,
        });

        assert_eq!(result, Err(DaemonError::ServiceCrashedError));
        Ok(())
    }

    #[test]
    fn test_stops_all_services_on_drop() -> anyhow::Result<()> {
        let service_port = Port::next_available()?;

        {
            let supervisor = Supervisor::new();
            supervisor.start(&Start {
                name: None,
                service: test_services::http_hello_world(service_port),
                wait: WaitFor::Port { port: service_port },
            })?;

            assert!(
                service_port.is_in_use(),
                "The service did not start correctly."
            );
        }

        assert!(
            service_port.is_available(),
            "The service did not stop correctly."
        );
        Ok(())
    }

    #[test]
    fn test_responds_with_the_name_if_one_is_provided() -> anyhow::Result<()> {
        let output_directory = tempfile::tempdir()?;
        let output_file = output_directory.path().join("timestamp.txt");

        let supervisor = Supervisor::new();
        let name = supervisor.start(&Start {
            name: Some("thingamabob".into()),
            service: test_services::file_watch(&output_file, vec!["echo".into(), "output".into()]),
            wait: WaitFor::AMoment,
        })?;

        assert_eq!(name, "thingamabob".into());
        Ok(())
    }
}
