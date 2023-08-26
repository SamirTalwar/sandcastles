use std::path::PathBuf;

use boof::*;

#[test]
fn example_program() -> anyhow::Result<()> {
    static SERVER_PORT: Port = Port(8080);
    let server_url = format!("http://localhost:{}/", SERVER_PORT);

    let daemon_socket_dir = tempfile::Builder::new()
        .prefix("boof-test-daemon")
        .tempdir()?;
    let daemon_socket = daemon_socket_dir.path().join("socket");

    {
        let daemon = Daemon::start_on_socket(daemon_socket.clone())?;

        assert!(
            daemon_socket.exists(),
            "the daemon socket has not been created"
        );

        let mut client = Client::connect_to(daemon.socket())?;
        client.start(http_hello_world(), WaitFor::Port(SERVER_PORT))?;

        assert!(
            SERVER_PORT.is_in_use(),
            "the service has not started correctly"
        );

        let response_body = reqwest::blocking::get(server_url)?.text()?;

        assert_eq!(response_body, "Hello, world!");

        Ok::<(), anyhow::Error>(())
    }?;

    assert!(
        SERVER_PORT.is_available(),
        "the service has not shut down correctly"
    );

    assert!(
        !daemon_socket.exists(),
        "the daemon socket has not been removed"
    );

    Ok(())
}

pub fn http_hello_world() -> Service {
    let root =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("Missing CARGO_MANIFEST_DIR"));
    let server_script = root.join("tests/services/http_hello_world.js");
    Service::Program(Program {
        command: "node".into(),
        arguments: vec![server_script.into()],
    })
}
