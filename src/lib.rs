use std::ffi::OsString;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::process::Command;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::{fs, io};

use anyhow::Context;

pub type Port = u16;

pub struct Daemon {
    socket_path: PathBuf,
    live_processes: LiveProcesses,
}

impl Daemon {
    pub fn with_socket(socket_path: PathBuf) -> anyhow::Result<Self> {
        let listener =
            UnixListener::bind(&socket_path).context("Could not create the daemon socket")?;
        listener
            .set_nonblocking(true)
            .context("Could not configure the daemon socket")?;
        let (stop_sender, stop_receiver) = mpsc::channel::<UnixStream>();
        let live_processes = LiveProcesses::new();
        let live_processes_ = live_processes.clone();
        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let stop_sender_ = stop_sender.clone();
                        let live_processes__ = live_processes_.clone();
                        thread::spawn(|| {
                            stream.set_nonblocking(false).unwrap_or_else(|err| {
                                eprintln!("Could not configure the server socket: {}", err)
                            });
                            Self::handle_connection(stream, live_processes__, stop_sender_);
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
            live_processes,
        })
    }

    pub fn socket(&self) -> &Path {
        &self.socket_path
    }

    fn handle_connection(
        mut stream: UnixStream,
        live_processes: LiveProcesses,
        stop_sender: mpsc::Sender<UnixStream>,
    ) {
        match bincode::deserialize_from(&mut stream) {
            Ok(Request::Shutdown) => {
                stop_sender
                    .send(stream)
                    .unwrap_or_else(|err| eprintln!("Failed to shut down the daemon: {}", err));
            }
            Ok(Request::Start(Service::Program(Program {
                command,
                arguments,
                wait,
            }))) => {
                match Command::new(command).args(arguments).spawn() {
                    Ok(process) => {
                        live_processes.add(process);
                        bincode::serialize_into(&mut stream, &Response::Success)
                    }
                    Err(err) => {
                        eprintln!("Failed to start a process.");
                        bincode::serialize_into(&mut stream, &Response::Failure(err.to_string()))
                    }
                }
                .unwrap_or_else(|serialize_err| {
                    eprintln!("Failed to serialize the response: {}", serialize_err)
                });
            }
            Err(err) => {
                eprintln!("Failed to deserialize the request: {}", err);
            }
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
        self.live_processes
            .shutdown()
            .unwrap_or_else(|err| eprintln!("An error occurred during shutdown: {}", err));
    }
}

#[derive(Clone)]
struct LiveProcesses(Arc<Mutex<Vec<Child>>>);

impl LiveProcesses {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }

    fn add(&self, process: Child) {
        let mut inner = self.0.lock().unwrap();
        inner.push(process);
    }

    fn shutdown(&mut self) -> anyhow::Result<()> {
        let mut inner = self.0.lock().unwrap();
        inner
            .drain(..)
            .map(|process| -> anyhow::Result<()> {
                nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(process.id().try_into()?),
                    nix::sys::signal::Signal::SIGTERM,
                )?;
                Ok(())
            })
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

    pub fn start(&mut self, service: Service) -> anyhow::Result<()> {
        bincode::serialize_into(&mut self.socket, &Request::Start(service))
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Program {
    pub command: OsString,
    pub arguments: Vec<OsString>,
    pub wait: WaitFor,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum WaitFor {
    Port(Port),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum Request {
    Start(Service),
    Shutdown,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum Response {
    Success,
    Failure(String),
}
