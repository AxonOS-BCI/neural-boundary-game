#!/usr/bin/env bash
# Top-level release gate for Neural Boundary Game v3.0.1.
# Runs every mandatory check, stops at the first failure, and names the
# failing step. Works on standard Linux and Android Termux; never assumes
# /tmp and never prints DONE after a failed command.
set -euo pipefail

cd "$(dirname "$0")/.."

step() {
  printf '\n== %s ==\n' "$1"
}

require() {
  command -v "$1" >/dev/null 2>&1 || {
    printf 'FAIL: required command not found: %s\n' "$1" >&2
    exit 127
  }
}

require cargo
require python3

if command -v rustup >/dev/null 2>&1; then
  rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
elif command -v pkg >/dev/null 2>&1; then
  # Termux: the wasm std lives in a separate package.
  pkg install -y rust-std-wasm32-unknown-unknown >/dev/null 2>&1 || true
fi

step "cargo fmt --all --check"
cargo fmt --all --check

step "cargo clippy --workspace --all-targets --all-features -- -D warnings"
cargo clippy --workspace --all-targets --all-features -- -D warnings

step "cargo test --workspace --all-features"
cargo test --workspace --all-features

step "cargo check --workspace --all-targets --all-features"
cargo check --workspace --all-targets --all-features

step "cargo build -p neural-boundary-web --target wasm32-unknown-unknown --release"
cargo build -p neural-boundary-web --target wasm32-unknown-unknown --release

step "cargo run -p neural-boundary-cli --release -- verify-all"
cargo run -q -p neural-boundary-cli --release -- verify-all

step "python3 tools/validate_replay.py"
python3 tools/validate_replay.py

step "python3 tools/check_hygiene.py"
python3 tools/check_hygiene.py

step "python3 tools/check_version_consistency.py"
python3 tools/check_version_consistency.py

step "python3 tools/check_links.py"
python3 tools/check_links.py

step "python3 tools/release_check.py"
python3 tools/release_check.py

printf '\nOK: every release gate passed.\n'
