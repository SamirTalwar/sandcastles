#![cfg(test)]

use std::path::PathBuf;

use crate::programs::Program;
use crate::Service;

pub fn http_hello_world() -> Service {
    let root =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("Missing CARGO_MANIFEST_DIR"));
    let server_script = root.join("tests/services/http_hello_world.js");
    Service::Program(Program {
        command: "node".into(),
        arguments: vec![server_script.into()],
    })
}
