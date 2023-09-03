#!/usr/bin/env nu

use std

def run [name operation] {
  print --stderr $"+ ($name)"
  do $operation
}

run 'cargo build' { cargo build --all-targets }
run 'cargo test' { cargo test }
run 'cargo clippy' { cargo clippy -- --deny=clippy::all }
run 'cargo fmt' { cargo fmt --check }
run 'cargo machete' { cargo machete }

if 'IN_NIX_SHELL' in $env {
  run 'check Rust version in Nix' {
    std assert equal (open rust-toolchain.toml | get toolchain.channel) (rustc --version | split row (char space) | $in.1)
  }
  run 'nix build' { nix build --offline }
} else {
  print --stderr 'Skipping Nix checks.'
}
