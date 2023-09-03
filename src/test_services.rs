#![cfg(test)]

use std::path::{Path, PathBuf};

use crate::programs::{Argument, Program};
use crate::services::Service;

pub fn file_watch(output_path: &Path, mut command: Vec<Argument>) -> Service {
    let program = root().join("tests/services/file_watch.sh");
    let mut arguments: Vec<Argument> = vec![output_path.into()];
    arguments.append(&mut command);
    Service::Program(Program {
        command: program.into(),
        arguments,
    })
}

pub fn http_hello_world() -> Service {
    let script = root().join("tests/services/http_hello_world.js");
    Service::Program(Program {
        command: "node".into(),
        arguments: vec![script.into()],
    })
}

fn root() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("Missing CARGO_MANIFEST_DIR"))
}
