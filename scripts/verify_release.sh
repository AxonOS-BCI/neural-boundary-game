#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

step() { printf '\n== %s ==\n' "$1"; }

command -v cargo >/dev/null 2>&1 || { echo "FAIL: cargo not found"; exit 127; }
command -v python3 >/dev/null 2>&1 || { echo "FAIL: python3 not found"; exit 127; }

if command -v rustup >/dev/null 2>&1; then
  rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
fi

step "cargo fmt --all --check"
cargo fmt --all --check

step "cargo clippy --workspace --all-targets --all-features -- -D warnings"
cargo clippy --workspace --all-targets --all-features -- -D warnings

step "cargo test --workspace --all-features --locked"
cargo test --workspace --all-features --locked

step "build web"
bash scripts/build_web.sh dist

step "verify-all"
cargo run -q -p neural-boundary-cli --release --locked -- verify-all

step "python gates"
python3 tools/validate_replay.py
python3 tools/check_hygiene.py
python3 tools/check_version_consistency.py
python3 tools/check_links.py
python3 tools/check_abi_exports.py
python3 tools/check_pwa_cache.py
python3 tools/generate_manifest.py
python3 tools/build_evidence_report.py

printf '\nOK: release gates passed.\n'
