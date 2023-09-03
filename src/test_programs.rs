#![cfg(test)]

use std::path::PathBuf;

use crate::programs::Program;

pub fn waits_for_termination() -> Program {
    let script = root().join("tests/programs/waits_for_termination.sh");
    Program {
        command: "bash".into(),
        arguments: vec![script.into()],
        environment: Default::default(),
    }
}

pub fn ignores_termination() -> Program {
    let script = root().join("tests/programs/ignores_termination.sh");
    Program {
        command: "bash".into(),
        arguments: vec![script.into()],
        environment: Default::default(),
    }
}

fn root() -> PathBuf {
    std::env::var("CARGO_MANIFEST_DIR")
        .expect("Missing CARGO_MANIFEST_DIR")
        .into()
}
