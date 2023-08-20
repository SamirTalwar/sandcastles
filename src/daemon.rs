use std::fs;
use std::io;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::Context;

use crate::communication::{Request, Response};
use crate::services::RunningService;

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
        let running_services = RunningServices::new();
        let running_services_ = running_services.clone();
        thread::spawn(move || {
            start(listener, running_services_);
        });
        Ok(Self {
            socket_path,
            running_services,
        })
    }

    pub fn socket(&self) -> &Path {
        &self.socket_path
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

fn start(listener: UnixListener, running_services: RunningServices) {
    let (stop_sender, stop_receiver) = mpsc::channel::<UnixStream>();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let stop_sender_ = stop_sender.clone();
                let running_services_ = running_services.clone();
                thread::spawn(|| {
                    stream.set_nonblocking(false).unwrap_or_else(|err| {
                        eprintln!("Could not configure the server socket: {}", err)
                    });
                    handle_connection(stream, running_services_, stop_sender_)
                        .unwrap_or_else(|err| eprintln!("Request error: {}", err));
                });
            }
            Err(err) => match err.kind() {
                io::ErrorKind::WouldBlock => {
                    if stop_requested(&stop_receiver) {
                        break;
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

fn stop_requested(stop_receiver: &mpsc::Receiver<UnixStream>) -> bool {
    match stop_receiver.try_recv() {
        Ok(mut stream) => {
            bincode::serialize_into(&mut stream, &Response::Success).unwrap_or_else(|err| {
                eprintln!("Failed to serialize the shutdown response: {}", err)
            });
            true
        }
        Err(mpsc::TryRecvError::Empty) => false,
        Err(mpsc::TryRecvError::Disconnected) => {
            eprintln!("The stop signal has been disconnected. Aborting.");
            true
        }
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
