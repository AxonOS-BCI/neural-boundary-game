# Neural Boundary Game

[![CI](https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/ci.yml/badge.svg)](https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/ci.yml)
[![Pages](https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/pages.yml/badge.svg)](https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/pages.yml)
![Rust](https://img.shields.io/badge/Rust-no__std%20core-f97316)
![WASM](https://img.shields.io/badge/WASM-browser%20demo-20d6ff)
![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-8ea0b6)

**Foundation Grande AxonOS Standard Edition.**

> **Do not ship raw signal. Ship typed intent.**

![Neural Boundary Game preview](preview.png)

## Live demo

After GitHub Pages deploy:

```text
https://axonos-bci.github.io/neural-boundary-game/
```

## What this is

`neural-boundary-game` is a deterministic Rust/WASM game that explains one AxonOS boundary rule:

```text
Signal layer    -> raw frames, noise, artifacts
Boundary layer  -> consent, confidence, evidence, checks
App layer       -> typed intent only
```

The player protects the boundary by validating evidence, quarantining hazards, activating consent, and releasing only after review gates are passed.

## Why it matters

BCI-adjacent software needs strong interface discipline.

The game makes the rule visible in 10 seconds:

- raw signal stays inside the device boundary;
- apps receive typed intent only;
- claims require evidence;
- releases must pass review gates;
- unsafe shortcuts create trust failure.

## v1.0.3 highlights

- AxonOS Standard / Foundation Grande visual system;
- strict 1280×720 premium technical composition;
- no skewed text, no clutter, no toy-like interface;
- `#![no_std]` deterministic Rust core;
- `#![forbid(unsafe_code)]`;
- Rust/WASM canvas frontend;
- replay vector and checksum validation;
- CLI replay verifier;
- CI and GitHub Pages workflows;
- corrected Termux push flow for `https://github.com/AxonOS-BCI/neural-boundary-game.git`;
- GitHub setup notes, issue templates, PR template, release checklist;
- reviewer-safe limitations and claim hygiene.

## Architecture

```text
crates/
  neural-boundary-core   no_std deterministic state machine
  neural-boundary-cli    replay verifier
  neural-boundary-web    Rust/WASM canvas frontend

vectors/
  replay-v1.0.3.json
  checksums.txt

docs/
  boundary, style, claims, commercial services, release process
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
H                 help
R                 restart
Esc               pause
```

Click/touch the bottom action bar on mobile.

## Run locally

```bash
rustup target add wasm32-unknown-unknown

cargo fmt --all
cargo test -p neural-boundary-core
cargo test -p neural-boundary-cli
cargo check -p neural-boundary-web
cargo build -p neural-boundary-web --target wasm32-unknown-unknown

python3 tools/validate_replay.py
python3 tools/check_hygiene.py
python3 tools/release_check.py
```

Browser:

```bash
cargo install trunk
trunk serve --open
```

Replay verifier:

```bash
cargo run -p neural-boundary-cli -- vectors/replay-v1.0.3.json
```

## Publish from Termux

From inside the unpacked repository:

```bash
bash scripts/termux_push.sh
```

The script defaults to:

```text
https://github.com/AxonOS-BCI/neural-boundary-game.git
```

Then enable Pages:

```text
Settings -> Pages -> Source: GitHub Actions
Actions -> Pages -> verify deploy
```

## GitHub repository attributes

Recommended About:

```text
Playable Rust/WASM demo of the core BCI safety rule: raw signal stays inside the device; apps receive typed intent only.
```

Recommended topics:

```text
rust wasm webassembly no-std bci privacy embedded deterministic-game axonos neurotechnology
```

See [`docs/GITHUB_SETUP.md`](docs/GITHUB_SETUP.md).

## Commercial services

This repository is a proof-of-work artifact.

If your embedded Rust, firmware, protocol, or BCI-adjacent repository needs reviewer-ready documentation, conformance vectors, `no_std` architecture review, or release hygiene, see:

[`docs/COMMERCIAL_SERVICES.md`](docs/COMMERCIAL_SERVICES.md)

Contact: **connect@axonos.org**

## Limitations

This is an educational technical demo.

It is not a medical device, not a clinical system, not a safety-certified system, does not process real signal data, and does not control stimulation hardware.

See [`docs/LIMITATIONS.md`](docs/LIMITATIONS.md).

## License

Dual licensed under MIT or Apache-2.0.
