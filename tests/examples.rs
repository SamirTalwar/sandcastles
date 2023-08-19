use std::net;
use std::path::PathBuf;

use anyhow::Context;
use boof::*;

#[test]
fn example_program() -> anyhow::Result<()> {
    let root =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").context("Missing CARGO_MANIFEST_DIR")?);
    let server_script = root.join("tests/server.js");

    static SERVER_PORT: Port = 8080;
    let server_url = format!("http://localhost:{}/", SERVER_PORT);

    let daemon_socket_dir = tempfile::Builder::new()
        .prefix("boof-test-daemon")
        .tempdir()?;
    let daemon_socket = daemon_socket_dir.path().join("socket");

    {
        let daemon = Daemon::with_socket(daemon_socket.clone())?;

        assert!(
            daemon_socket.exists(),
            "the daemon socket has not been created"
        );

        let mut client = Client::connect_to(&daemon)?;
        client.start(
            Service::Program(Program {
                command: "node".into(),
                arguments: vec![server_script.into()],
            }),
            WaitFor::Port(SERVER_PORT),
        )?;

        std::thread::sleep(std::time::Duration::from_secs(1));
        assert!(
            service_running_on_port(SERVER_PORT),
            "the service has not started correctly"
        );

        let response_body = reqwest::blocking::get(server_url)?.text()?;

        assert_eq!(response_body, "Hello, world!");

        Ok::<(), anyhow::Error>(())
    }?;

    std::thread::sleep(std::time::Duration::from_secs(1));
    assert!(
        !service_running_on_port(8080),
        "the service has not shut down correctly"
    );

    assert!(
        !daemon_socket.exists(),
        "the daemon socket has not been removed"
    );

    Ok(())
}

fn service_running_on_port(port: Port) -> bool {
    let socket_address = net::SocketAddrV4::new(net::Ipv4Addr::UNSPECIFIED, port);
    let result = net::TcpListener::bind(socket_address);
    result.is_err()
}
