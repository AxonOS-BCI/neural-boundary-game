#!/usr/bin/env bash
set -euo pipefail

echo "== Neural Boundary Game v2.1.2 smoke check =="

if command -v rustup >/dev/null 2>&1; then
  rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
fi

cargo fmt --all --check
cargo test -p neural-boundary-core
cargo test -p neural-boundary-cli
cargo check -p neural-boundary-web
cargo build -p neural-boundary-web --target wasm32-unknown-unknown

python3 tools/validate_replay.py
python3 tools/check_hygiene.py
python3 tools/release_check.py

echo "OK"
