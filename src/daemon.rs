use std::fs;
use std::io;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::Context;

use crate::communication::{Request, Response};
use crate::supervisor::Supervisor;

pub struct Daemon {
    socket_path: PathBuf,
}

impl Daemon {
    pub fn with_socket(socket_path: PathBuf) -> anyhow::Result<Self> {
        Self::new(socket_path, Supervisor::new())
    }

    pub fn new(socket_path: PathBuf, supervisor: Supervisor) -> anyhow::Result<Self> {
        let listener =
            UnixListener::bind(&socket_path).context("Could not create the daemon socket")?;
        listener
            .set_nonblocking(true)
            .context("Could not configure the daemon socket")?;
        thread::spawn(move || {
            start(listener, supervisor);
        });
        Ok(Self { socket_path })
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
        self.stop()
            .unwrap_or_else(|err| eprintln!("Could not request the daemon to shut down: {}", err));
        fs::remove_file(&self.socket_path)
            .unwrap_or_else(|err| eprintln!("An error occurred during shutdown: {}", err));
    }
}

fn start(listener: UnixListener, supervisor: Supervisor) {
    let (stop_sender, stop_receiver) = mpsc::channel::<UnixStream>();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let supervisor_for_stream = supervisor.clone();
                let stop_sender_for_stream = stop_sender.clone();
                thread::spawn(|| {
                    stream.set_nonblocking(false).unwrap_or_else(|err| {
                        eprintln!("Could not configure the server socket: {}", err)
                    });
                    handle_connection(stream, supervisor_for_stream, stop_sender_for_stream)
                        .unwrap_or_else(|err| eprintln!("Request error: {}", err));
                });
            }
            Err(err) => match err.kind() {
                io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(100));
                }
                _ => {
                    eprintln!("Connection failed: {}", err);
                    break;
                }
            },
        };
        if stop_requested(&stop_receiver) {
            break;
        }
    }
}

fn handle_connection(
    mut stream: UnixStream,
    supervisor: Supervisor,
    stop_sender: mpsc::Sender<UnixStream>,
) -> anyhow::Result<()> {
    let request =
        bincode::deserialize_from(&mut stream).context("Failed to deserialize the request")?;
    match request {
        Request::Start { service, wait } => match supervisor.start(service, wait) {
            Ok(()) => bincode::serialize_into(&mut stream, &Response::Success)
                .context("Failed to serialize the response"),
            Err(err) => {
                eprintln!("Failed to start a program: {}", err);
                bincode::serialize_into(&mut stream, &Response::Failure(err.to_string()))
                    .context("Failed to serialize the response")
            }
        },
        Request::Shutdown => {
            supervisor.stop_all().unwrap(); // stop everything before responding
            stop_sender
                .send(stream)
                .context("Failed to shut down the daemon")?;
            Ok(())
        }
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
