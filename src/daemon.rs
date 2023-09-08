use std::fs;
use std::io;
use std::mem;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use crate::awaiter::Awaiter;
use crate::communication::{Request, Response};
use crate::error::DaemonError;
use crate::error::DaemonResult;
use crate::log;
use crate::supervisor::Supervisor;
use crate::timing::Duration;

enum StopHandle {
    Thread(thread::JoinHandle<()>),
    Awaiter(Awaiter),
}

pub struct Daemon {
    socket_path: PathBuf,
    stop_handle: Mutex<StopHandle>,
    stop_signal: Arc<AtomicBool>,
}

impl Daemon {
    pub fn start_on_socket(socket_path: PathBuf) -> DaemonResult<Self> {
        Self::start(socket_path, Supervisor::new())
    }

    pub fn start(socket_path: PathBuf, supervisor: Supervisor) -> DaemonResult<Self> {
        let listener = UnixListener::bind(&socket_path)
            .map_err(|error| DaemonError::SocketCreationError(error.into()))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| DaemonError::SocketConfigurationError(error.into()))?;
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_for_start = Arc::clone(&stop_signal);
        let thread_handle = thread::spawn(move || {
            start(&supervisor, listener, stop_signal_for_start.as_ref());
        });
        Ok(Self {
            socket_path,
            stop_handle: Mutex::new(StopHandle::Thread(thread_handle)),
            stop_signal,
        })
    }

    pub fn socket(&self) -> &Path {
        &self.socket_path
    }

    pub fn stop(&self) {
        self.stop_signal.store(true, Ordering::Relaxed);
    }

    pub fn wait(&self) {
        let (thread_handle, awaiter) = {
            let mut stop_handle = self.stop_handle.lock().unwrap();
            match &*stop_handle {
                StopHandle::Thread(_) => {
                    let awaiter = Awaiter::new();
                    let StopHandle::Thread(handle) =
                        mem::replace(&mut *stop_handle, StopHandle::Awaiter(awaiter.clone())) else {
                            unreachable!()
                        };
                    (Some(handle), awaiter)
                }
                StopHandle::Awaiter(awaiter) => (None, awaiter.clone()),
            }
        };
        match thread_handle {
            Some(handle) => {
                handle
                    .join()
                    .expect("Failed to wait for the daemon to shut down.");
                awaiter.unlock();
            }
            None => {
                awaiter.wait();
            }
        }
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        self.stop();
        self.wait();
        fs::remove_file(&self.socket_path)
            .unwrap_or_else(|error| log::error!(event = "SHUTDOWN", error = error.log()));
    }
}

fn start(supervisor: &Supervisor, listener: UnixListener, internal_stop_signal: &AtomicBool) {
    log::debug!(event = "STARTED");
    let (stop_sender, stop_receiver) = mpsc::channel();
    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                let supervisor_for_connection = supervisor.clone();
                let stop_sender_for_connection = stop_sender.clone();
                thread::spawn(move || {
                    stream
                        .set_nonblocking(false)
                        .map_err(|error| DaemonError::SocketConfigurationError(error.into()))
                        .and_then(|_| {
                            handle_connection(
                                stream,
                                &supervisor_for_connection,
                                stop_sender_for_connection,
                            )
                        })
                        .unwrap_or_else(|error| log::error!(event = "ACCEPT", error))
                });
            }
            Err(error) => match error.kind() {
                io::ErrorKind::WouldBlock => {
                    Duration::QUANTUM.sleep();
                }
                _ => {
                    log::fatal!(event = "ACCEPT", error = error.log());
                    break;
                }
            },
        };
        if stop_requested(supervisor, internal_stop_signal, &stop_receiver) {
            break;
        }
    }
    log::debug!(event = "STOPPED");
}

fn handle_connection(
    mut stream: UnixStream,
    supervisor: &Supervisor,
    stop_sender: mpsc::Sender<UnixStream>,
) -> DaemonResult<()> {
    let request = bincode::deserialize_from(&mut stream)
        .map_err(|err| DaemonError::RequestDeserializationError(*err))?;
    log::debug!(event = "HANDLE", request);
    match request {
        Request::Start(instruction) => {
            log::info!(event = "START", instruction);
            let response = match supervisor.start(&instruction) {
                Ok(()) => Response::Success,
                Err(error) => {
                    log::warning!(event = "START", instruction, error = error.log());
                    Response::Failure(error.to_string())
                }
            };
            log::debug!(event = "HANDLE", response);
            bincode::serialize_into(&mut stream, &response)
                .map_err(|err| DaemonError::ResponseSerializationError(*err))
        }
        Request::Shutdown => stop_sender
            .send(stream)
            .map_err(DaemonError::ShutdownRequestError),
    }
}

fn stop_requested(
    supervisor: &Supervisor,
    internal_stop_signal: &AtomicBool,
    external_stop_receiver: &mpsc::Receiver<UnixStream>,
) -> bool {
    if internal_stop_signal.load(Ordering::Relaxed) {
        log::debug!(event = "SHUTDOWN");
        // stop everything before responding
        supervisor
            .stop_all()
            .unwrap_or_else(|error| log::error!(event = "SHUTDOWN", error = error.log()));
        return true;
    }
    match external_stop_receiver.try_recv() {
        Ok(mut stream) => {
            log::debug!(event = "SHUTDOWN");
            // stop everything before responding
            supervisor
                .stop_all()
                .unwrap_or_else(|error| log::error!(event = "SHUTDOWN", error = error.log()));

            let response = Response::Success;
            log::debug!(event = "HANDLE", response);
            bincode::serialize_into(&mut stream, &response).unwrap_or_else(|error| {
                log::error!(event = "ACCEPT", error = error.log());
            });
            true
        }
        Err(mpsc::TryRecvError::Empty) => false,
        Err(mpsc::TryRecvError::Disconnected) => {
            log::fatal!(event = "DISCONNECT");
            true
        }
    }
}
