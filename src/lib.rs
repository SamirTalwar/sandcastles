use std::ffi::OsString;
use std::fs;
use std::io;
use std::net;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::process::Command;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::Context;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Port(pub u16);

impl std::fmt::Display for Port {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Port {
    pub fn is_available(&self) -> bool {
        let socket_address = net::SocketAddrV4::new(net::Ipv4Addr::UNSPECIFIED, self.0);
        let result = net::TcpListener::bind(socket_address);
        result.is_ok()
    }

    pub fn is_in_use(&self) -> bool {
        !self.is_available()
    }
}

pub struct Daemon {
    socket_path: PathBuf,
    running_services: RunningServices,
}

impl Daemon {
    pub fn with_socket(socket_path: PathBuf) -> anyhow::Result<Self> {
        let listener =
            UnixListener::bind(&socket_path).context("Could not create the daemon socket")?;
        listener
            .set_nonblocking(true)
            .context("Could not configure the daemon socket")?;
        let (stop_sender, stop_receiver) = mpsc::channel::<UnixStream>();
        let running_services = RunningServices::new();
        let running_services_ = running_services.clone();
        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let stop_sender_ = stop_sender.clone();
                        let running_services__ = running_services_.clone();
                        thread::spawn(|| {
                            stream.set_nonblocking(false).unwrap_or_else(|err| {
                                eprintln!("Could not configure the server socket: {}", err)
                            });
                            Self::handle_connection(stream, running_services__, stop_sender_)
                                .unwrap_or_else(|err| eprintln!("Request error: {}", err));
                        });
                    }
                    Err(err) => match err.kind() {
                        io::ErrorKind::WouldBlock => {
                            match stop_receiver.try_recv() {
                                Ok(mut stream) => {
                                    bincode::serialize_into(&mut stream, &Response::Success)
                                        .unwrap_or_else(|err| {
                                            eprintln!(
                                                "Failed to serialize the shutdown response: {}",
                                                err
                                            )
                                        });
                                    break;
                                }
                                Err(mpsc::TryRecvError::Empty) => {}
                                Err(mpsc::TryRecvError::Disconnected) => {
                                    eprintln!("The stop signal has been disconnected. Aborting.");
                                    break;
                                }
                            }
                            thread::sleep(Duration::from_millis(100));
                        }
                        _ => {
                            eprintln!("Connection failed: {}", err);
                            break;
                        }
                    },
                }
            }
        });
        Ok(Self {
            socket_path,
            running_services,
        })
    }

    pub fn socket(&self) -> &Path {
        &self.socket_path
    }

    fn handle_connection(
        mut stream: UnixStream,
        running_services: RunningServices,
        stop_sender: mpsc::Sender<UnixStream>,
    ) -> anyhow::Result<()> {
        let request =
            bincode::deserialize_from(&mut stream).context("Failed to deserialize the request")?;
        match request {
            Request::Shutdown => stop_sender
                .send(stream)
                .context("Failed to shut down the daemon"),
            Request::Start { service, wait } => match service.start() {
                Ok(running_service) => {
                    running_services.add(running_service);
                    wait.block_until_ready()?;
                    bincode::serialize_into(&mut stream, &Response::Success)
                        .context("Failed to serialize the response")
                }
                Err(err) => {
                    eprintln!("Failed to start a program: {}", err);
                    bincode::serialize_into(&mut stream, &Response::Failure(err.to_string()))
                        .context("Failed to serialize the response")
                }
            },
        }
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        let mut socket = UnixStream::connect(self.socket())?;
        bincode::serialize_into(&mut socket, &Request::Shutdown)?;
        match bincode::deserialize_from(&mut socket)? {
            Response::Success => Ok(()),
            Response::Failure(message) => Err(anyhow::anyhow!(message)),
        }
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        (self.stop())
            .unwrap_or_else(|err| eprintln!("Could not request the daemon to shut down: {}", err));
        fs::remove_file(&self.socket_path)
            .unwrap_or_else(|err| eprintln!("An error occurred during shutdown: {}", err));
        self.running_services
            .shutdown()
            .unwrap_or_else(|err| eprintln!("An error occurred during shutdown: {}", err));
    }
}

#[derive(Clone)]
struct RunningServices(Arc<Mutex<Vec<RunningService>>>);

impl RunningServices {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }

    fn add(&self, service: RunningService) {
        let mut inner = self.0.lock().unwrap();
        inner.push(service);
    }

    fn shutdown(&mut self) -> anyhow::Result<()> {
        let mut inner = self.0.lock().unwrap();
        inner
            .drain(..)
            .map(|mut service| service.stop())
            .collect::<Vec<anyhow::Result<()>>>()
            .into_iter()
            .collect::<anyhow::Result<()>>()
    }
}

pub struct Client {
    socket: UnixStream,
}

impl Client {
    pub fn connect_to(daemon: &Daemon) -> anyhow::Result<Self> {
        let socket = UnixStream::connect(daemon.socket())
            .context("Could not connect to the daemon socket")?;
        Ok(Client { socket })
    }

    pub fn start(&mut self, service: Service, wait: WaitFor) -> anyhow::Result<()> {
        bincode::serialize_into(&mut self.socket, &Request::Start { service, wait })
            .context("Could not serialize the request")?;
        let response = bincode::deserialize_from(&mut self.socket)
            .context("Could not deserialize the response")?;
        match response {
            Response::Success => Ok(()),
            Response::Failure(message) => anyhow::bail!(message),
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Service {
    Program(Program),
}

impl Service {
    fn start(&self) -> anyhow::Result<RunningService> {
        match self {
            Self::Program(Program { command, arguments }) => {
                let process = Command::new(command).args(arguments).spawn()?;
                Ok(RunningService::Program(process))
            }
        }
    }
}

enum RunningService {
    Program(Child),
}

impl RunningService {
    fn stop(&mut self) -> anyhow::Result<()> {
        match self {
            Self::Program(process) => {
                let process_id = process.id();
                nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(process_id.try_into()?),
                    nix::sys::signal::Signal::SIGTERM,
                )
                .context(format!("Failed to stop the process with ID {}", process_id))
            }
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Program {
    pub command: OsString,
    pub arguments: Vec<OsString>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum WaitFor {
    Port(Port),
}

impl WaitFor {
    fn block_until_ready(&self) -> anyhow::Result<()> {
        match self {
            Self::Port(port) => {
                while port.is_available() {
                    thread::yield_now();
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum Request {
    Start { service: Service, wait: WaitFor },
    Shutdown,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum Response {
    Success,
    Failure(String),
}
