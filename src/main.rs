use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use signal_hook::consts::signal;

use boof::Daemon;

#[derive(Debug, clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Arguments {
    #[command(subcommand)]
    command: Command,
    #[arg(long = "socket-path")]
    socket_path: Option<PathBuf>,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    Daemon,
    Start,
    Shutdown,
}

fn main() -> anyhow::Result<()> {
    let args = Arguments::parse();
    let socket_path = args.socket_path.unwrap_or_else(default_socket_path);
    match args.command {
        Command::Daemon => {
            if let Some(socket_dir) = socket_path.parent() {
                fs::create_dir_all(socket_dir)?;
            }
            let daemon = Arc::new(Daemon::start_on_socket(socket_path)?);
            unsafe {
                for signal in [signal::SIGINT, signal::SIGQUIT, signal::SIGTERM] {
                    let daemon_for_signal = Arc::downgrade(&Arc::clone(&daemon));
                    signal_hook::low_level::register(signal, move || {
                        if let Some(d) = daemon_for_signal.upgrade() {
                            d.stop();
                        }
                    })?;
                }
            }
            daemon.wait();
            Ok(())
        }
        Command::Start => todo!("start"),
        Command::Shutdown => todo!("shutdown"),
    }
}

fn default_socket_path() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            [
                env::var_os("HOME").expect("No home directory set."),
                ".local".into(),
                "state".into(),
            ]
            .into_iter()
            .collect()
        })
        .join("boof")
        .join("daemon.socket")
}
