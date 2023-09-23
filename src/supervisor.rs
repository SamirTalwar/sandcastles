use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::communication::{ExitStatus, Start, Stop};
use crate::error::{DaemonError, DaemonResult};
use crate::names::{random_name, Name};
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
        let mut inner = self.0.lock().unwrap();
        let name = instruction.name.clone().unwrap_or_else(random_name);
        if inner.has_service_named(&name) {
            return Err(DaemonError::ServiceAlreadyExistsError { name });
        }
        let running = instruction.service.start()?;
        let running = inner.add(name.clone(), running);
        instruction.wait.block_until_ready(Duration::FOREVER)?; // we need to pick a global timeout here
        if running.is_running()? {
            Ok(name)
        } else {
            Err(DaemonError::ServiceCrashedError)
        }
    }

    pub fn stop(&self, instruction: &Stop) -> DaemonResult<ExitStatus> {
        let mut inner = self.0.lock().unwrap();
        let name = &instruction.name;
        match inner.retrieve(name) {
            Some(mut service) => service.stop(Duration::STOP_TIMEOUT),
            None => Err(DaemonError::NoSuchServiceError { name: name.clone() }),
        }
    }

    pub fn stop_all(&self) -> DaemonResult<()> {
        self.0.lock().unwrap().stop_all()
    }
}

struct RunningServices(HashMap<Name, RunningService>);

impl RunningServices {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn has_service_named(&self, name: &Name) -> bool {
        self.0.contains_key(name)
    }

    fn add(&mut self, name: Name, service: RunningService) -> &mut RunningService {
        match self.0.entry(name) {
            Entry::Occupied(_) => unreachable!("The service name was stolen."),
            Entry::Vacant(entry) => entry.insert(service),
        }
    }

    fn retrieve(&mut self, name: &Name) -> Option<RunningService> {
        self.0.remove(name)
    }

    fn stop_all(&mut self) -> DaemonResult<()> {
        self.0
            .drain()
            .map(|(_, mut service)| service.stop(Duration::STOP_TIMEOUT).map(|_| ()))
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
        let output_file = output_directory.path().join("output.txt");

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
    fn test_refuses_to_start_a_service_with_a_name_that_is_taken() -> anyhow::Result<()> {
        let name: Name = "double".parse()?;
        let output_directory = tempfile::tempdir()?;
        let output_file = output_directory.path().join("output.txt");
        let supervisor = Supervisor::new();
        supervisor.start(&Start {
            name: Some(name.clone()),
            service: test_services::file_watch(&output_file, vec!["echo".into(), "output".into()]),
            wait: WaitFor::AMoment,
        })?;

        let result = supervisor.start(&Start {
            name: Some(name.clone()),
            service: test_services::file_watch(&output_file, vec!["echo".into(), "output".into()]),
            wait: WaitFor::AMoment,
        });

        assert_eq!(result, Err(DaemonError::ServiceAlreadyExistsError { name }));
        Ok(())
    }

    #[test]
    fn test_stops_an_individual_service() -> anyhow::Result<()> {
        let service_port = Port::next_available()?;
        let supervisor = Supervisor::new();
        let service_name = supervisor.start(&Start {
            name: None,
            service: test_services::http_hello_world(service_port),
            wait: WaitFor::Port { port: service_port },
        })?;

        let response_status =
            reqwest::blocking::get(format!("http://localhost:{}/", service_port))?.status();
        assert_eq!(response_status, 200);

        supervisor.stop(&Stop { name: service_name })?;

        assert!(
            service_port.is_available(),
            "The service did not stop correctly."
        );
        Ok(())
    }

    #[test]
    fn test_refuses_to_stop_a_service_with_an_unknown_name() -> anyhow::Result<()> {
        let name: Name = "something".parse()?;
        let supervisor = Supervisor::new();

        let result = supervisor.stop(&Stop { name: name.clone() });

        assert_eq!(result, Err(DaemonError::NoSuchServiceError { name }));
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
        let output_file = output_directory.path().join("output.txt");

        let supervisor = Supervisor::new();
        let name = supervisor.start(&Start {
            name: Some("thingamabob".parse()?),
            service: test_services::file_watch(&output_file, vec!["echo".into(), "output".into()]),
            wait: WaitFor::AMoment,
        })?;

        assert_eq!(name, "thingamabob".parse()?);
        Ok(())
    }

    #[test]
    fn test_generates_a_random_name_if_one_is_not_provided() -> anyhow::Result<()> {
        let output_directory = tempfile::tempdir()?;
        let output_file_1 = output_directory.path().join("one.txt");
        let output_file_2 = output_directory.path().join("two.txt");

        let supervisor = Supervisor::new();
        let name_1 = supervisor.start(&Start {
            name: None,
            service: test_services::file_watch(
                &output_file_1,
                vec!["echo".into(), "output".into()],
            ),
            wait: WaitFor::AMoment,
        })?;
        let name_2 = supervisor.start(&Start {
            name: None,
            service: test_services::file_watch(
                &output_file_2,
                vec!["echo".into(), "output".into()],
            ),
            wait: WaitFor::AMoment,
        })?;

        assert_ne!(name_1, name_2);
        Ok(())
    }
}
