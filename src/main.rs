use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use signal_hook::consts::signal;

use sandcastles::*;

mod args {
    use std::path::PathBuf;

    use sandcastles::Argument;

    #[derive(Debug, clap::Parser)]
    #[command(author, version, about, long_about = None)]
    pub struct Arguments {
        #[command(subcommand)]
        pub command: Command,
        #[arg(long = "socket-path")]
        pub socket_path: Option<PathBuf>,
    }

    #[derive(Debug, clap::Subcommand)]
    pub enum Command {
        Daemon,
        Start {
            command: Argument,
            arguments: Vec<Argument>,
            #[arg(long = "env", value_parser = parse_env)]
            environment: Vec<(Argument, Argument)>,
        },
        Shutdown,
    }

    fn parse_env(arg: &str) -> Result<(Argument, Argument), &'static str> {
        if let [name, value] = arg.splitn(2, '=').collect::<Vec<&str>>()[..] {
            Ok((name.into(), value.into()))
        } else {
            Err("must be in the format `NAME=VALUE`")
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = args::Arguments::parse();
    let socket_path = args.socket_path.unwrap_or_else(default_socket_path);
    match args.command {
        args::Command::Daemon => {
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
        args::Command::Start {
            command,
            arguments,
            environment,
        } => Client::connect_to(&socket_path)?.start(Start {
            service: Service::Program(Program {
                command,
                arguments,
                environment: environment.into_iter().collect(),
            }),
            wait: WaitFor::None,
        }),
        args::Command::Shutdown => Client::connect_to(&socket_path)?.shutdown(),
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
        .join("sandcastles")
        .join("daemon.socket")
}
