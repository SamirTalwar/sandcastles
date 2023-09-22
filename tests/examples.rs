use std::path::PathBuf;

use sandcastles::*;

#[test]
fn example_program() -> anyhow::Result<()> {
    static SERVER_PORT: Port = Port(8080);
    let server_url = format!("http://localhost:{}/", SERVER_PORT);

    let daemon_socket_dir = tempfile::Builder::new()
        .prefix("sandcastles-test-daemon")
        .tempdir()?;
    let daemon_socket = daemon_socket_dir.path().join("socket");

    {
        let _daemon = Daemon::start_on_socket(daemon_socket.clone())?;

        assert!(
            daemon_socket.exists(),
            "the daemon socket has not been created"
        );

        let mut client = Client::connect_to(&daemon_socket)?;
        client.start(Start {
            name: Some("hello".parse()?),
            service: http_hello_world(),
            wait: WaitFor::Port { port: SERVER_PORT },
        })?;

        assert!(
            SERVER_PORT.is_in_use(),
            "the service has not started correctly"
        );

        let running_services = client.list()?;
        assert_eq!(
            running_services,
            vec![ServiceDetails {
                name: "hello".parse()?
            }]
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
        environment: Default::default(),
    })
}
