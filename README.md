# Neural Boundary Game

[![CI](https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/ci.yml/badge.svg)](https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/ci.yml)
[![Pages](https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/pages.yml/badge.svg)](https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/pages.yml)
![Rust](https://img.shields.io/badge/Rust-no__std%20core-f97316)
![WASM](https://img.shields.io/badge/WASM-boundary%20artifact-20d6ff)
![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-8ea0b6)

**Elite AxonOS Standard Foundation Grande Style.**

> **Do not ship raw signal. Ship typed intent.**

![Neural Boundary Game preview](preview.png)

## Live demo

```text
https://axonos-bci.github.io/neural-boundary-game/
```

## What this is

Neural Boundary Game is a deterministic Rust/WASM demo of the core AxonOS BCI boundary rule:

```text
Signal layer    -> raw frames, noise, artifacts
Boundary layer  -> consent, confidence, evidence, checks
App layer       -> typed intent only
```

The project is built from a clean v1.6 baseline: no `web-sys`, no `wasm-bindgen`, no `#[no_mangle]`, no browser-binding dependency surface, and no patched legacy glue.

## Architecture

```text
crates/
  neural-boundary-core   no_std deterministic state machine
  neural-boundary-cli    dependency-light replay verifier
  neural-boundary-web    dependency-light WASM boundary adapter

vectors/
  replay-v1.6.0.json
  checksums.txt

docs/
  GitHub setup, claim hygiene, BCI boundary, release process
```

## Controls

```text
W / ArrowUp       move up
S / ArrowDown     move down
1 / Space         validate
2                 convert
3                 quarantine
4                 consent gate
5                 evidence gate
Enter             release
```

Click or touch the bottom action bar on mobile.

## Run locally

```bash
rustup target add wasm32-unknown-unknown
bash scripts/smoke_check.sh
```

Manual checks:

```bash
cargo fmt --all --check
cargo test -p neural-boundary-core
cargo test -p neural-boundary-cli
cargo test -p neural-boundary-web
cargo check -p neural-boundary-web
cargo build -p neural-boundary-web --target wasm32-unknown-unknown --release
python3 tools/validate_replay.py
python3 tools/check_hygiene.py
python3 tools/release_check.py
```

## Clean signed release push

This rewrites the remote `main` history. Use only when intentionally replacing the previous history with clean v1.6:

```bash
I_UNDERSTAND_REWRITE_HISTORY=YES bash scripts/force_clean_push_signed.sh
```

## GitHub repository attributes

About:

```text
Playable Rust/WASM demo of the core BCI safety rule: raw signal stays inside the device; apps receive typed intent only.
```

Website:

```text
https://axonos-bci.github.io/neural-boundary-game/
```

Topics:

```text
rust wasm webassembly no-std bci privacy embedded deterministic-game axonos neurotechnology
```

## Commercial services

See [`docs/COMMERCIAL_SERVICES.md`](docs/COMMERCIAL_SERVICES.md).

Contact: **connect@axonos.org**

## Limitations

This is an educational technical demo. It does not process real signal data and does not control stimulation hardware.

See [`docs/LIMITATIONS.md`](docs/LIMITATIONS.md).

## License

Dual licensed under MIT or Apache-2.0.
