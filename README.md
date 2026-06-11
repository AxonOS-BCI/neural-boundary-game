# Neural Boundary Game

**Do not ship raw signal. Ship typed intent.**

[![CI](https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/ci.yml/badge.svg)](https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/ci.yml)
[![Pages](https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/pages.yml/badge.svg)](https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/pages.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-1f6feb)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-no__std%20deterministic%20core-f74c00)](crates/neural-boundary-core/src/lib.rs)

[![Neural Boundary Game — Foundation Grande stage](preview.png)](https://axonos-bci.github.io/neural-boundary-game/)

Playable Rust/WASM demo of the core BCI safety rule: **raw signal stays
inside the device; applications receive typed intent only.**

**Play it now:** <https://axonos-bci.github.io/neural-boundary-game/>

## What this demonstrates

In a BCI-adjacent stack there is one boundary that decides whether the whole
system is reviewable: the line between the signal layer and the application
layer. This repository turns that line into a playfield. Entities stream
toward a membrane across five lanes; you validate intent, gate consent, log
evidence, quarantine hazards — and only then release. Raw frames must never
cross. Unsupported claims move faster than evidence, because they always do.

The game is also a deterministic reference implementation:

- the simulation core is `#![no_std]` Rust with `#![forbid(unsafe_code)]`,
  zero allocation, fixed entity pool, integer math, 60 Hz fixed step;
- every run is reproducible from a seed and an input script;
- shipped replay vectors pin the exact terminal state — down to a 64-bit
  FNV-1a hash of the entire simulation — and CI re-verifies them on every
  push.

Win by sealing the boundary: `TRUST ≥ 90 · RISK ≤ 20 · INTEGRITY ≥ 80 ·
EVIDENCE ≥ L2 · all 5 review gates · 0 raw leaks`. Lose when integrity
collapses, risk overflows, a third raw frame leaks, or a stimulation command
crosses. Full rules: [`docs/GAME_SPEC.md`](docs/GAME_SPEC.md).

## Controls

| Input | Effect |
|---|---|
| `↑ ↓` / `W S` / click a lane | select lane |
| `1` Validate | type an `INTENT`, classify a `?PKT` |
| `2` Convert | validated intent → `TYPED` (needs consent + evidence ≥ L1) |
| `3` Quarantine | contain a hazard or claim |
| `4` Consent | gate a `CONSENT` token (25 s window) |
| `5` Evidence | log `EVIDENCE` / `CHECKSUM` / `CI TEST` |
| `⏎` Release | seal the boundary |
| `P` / `R` / `H` | pause / restart / help |

## Workspace

```text
crates/
  neural-boundary-core   #![no_std] deterministic simulation (game rules, RNG, state hash)
  neural-boundary-cli    replay verifier + vector toolkit (verify / record / search / trace)
  neural-boundary-web    wasm-bindgen front-end: canvas playfield + Foundation Grande stage
vectors/                 pinned replay vectors + sha256 checksums
docs/                    game spec, replay spec, boundary docs, style standard
tools/                   replay validator, claim-hygiene gate, release gate, preview generator
scripts/                 smoke check, Termux push, release tagging
```

## Deterministic replays

Verify the canonical vector against the core:

```bash
cargo run -p neural-boundary-cli --release -- verify
```

```text
Replay OK
Final trust: 92
Final risk: 12
Final integrity: 88
Boundary status: SEALED
```

Both shipped vectors use **seed 58** on standard difficulty — the same world
twice. Played with boundary discipline (38 recorded actions), it ends at tick
1862: trust 92, risk 12, integrity 88, five gates, zero leaks, `SEALED`. Left
idle, the very same seed breaches at tick 948 when the third raw frame
crosses:

```bash
cargo run -p neural-boundary-cli --release -- verify vectors/replay-breach-demo-v2.1.2.json
```

```text
Replay OK
Final trust: 47
Final risk: 0
Final integrity: 58
Boundary status: BREACHED
```

The only difference between `SEALED` and `BREACHED` is what you do at the
membrane. Schema, hash definition and regeneration commands:
[`docs/REPLAY_SPEC.md`](docs/REPLAY_SPEC.md).

## Build and run locally

Prerequisites: stable Rust with the `wasm32-unknown-unknown` target, and
[Trunk](https://trunkrs.dev) for the web stage.

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk

trunk serve --open          # web stage at http://127.0.0.1:8080
cargo test -p neural-boundary-core
cargo test -p neural-boundary-cli
```

Push from Termux (runs every gate, commits, pushes):

```bash
bash scripts/termux_push.sh
# default commit message:
# feat: release Neural Boundary Game v2.1.2 Foundation Grande Edition
```

## Quality gates

`bash scripts/smoke_check.sh` runs exactly what CI runs:

```text
cargo fmt --all --check
cargo test -p neural-boundary-core
cargo test -p neural-boundary-cli
cargo check -p neural-boundary-web
cargo build -p neural-boundary-web --target wasm32-unknown-unknown
python3 tools/validate_replay.py     # vector schema + checksums
python3 tools/check_hygiene.py       # forbidden capability claims
python3 tools/release_check.py       # version consistency + release layout
```

## Repository presentation

About, topics, website, social preview and release steps are documented in
[`docs/GITHUB_SETUP.md`](docs/GITHUB_SETUP.md). The stage's visual tokens are
normative: [`docs/AXONOS_STANDARD_STYLE.md`](docs/AXONOS_STANDARD_STYLE.md).

## Scope and claims

This is an educational technical demo of a software boundary principle. It
does not process real signal data and does not control hardware — the full
list is in [`docs/LIMITATIONS.md`](docs/LIMITATIONS.md), and
[`docs/CLAIM_HYGIENE.md`](docs/CLAIM_HYGIENE.md) is enforced by CI. Boundary
background: [`docs/BCI_BOUNDARY.md`](docs/BCI_BOUNDARY.md),
[`docs/NO_RAW_NEURAL_DATA.md`](docs/NO_RAW_NEURAL_DATA.md).

## Working with the author

Repository review, `no_std` architecture, protocol boundaries, conformance
vectors and reviewer-ready documentation:
[`docs/COMMERCIAL_SERVICES.md`](docs/COMMERCIAL_SERVICES.md) ·
[axonos.org](https://axonos.org) ·
[medium.com/@AxonOS](https://medium.com/@AxonOS) · <connect@axonos.org>

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option.

---

`v2.1.2 • AxonOS Standard Foundation Grande Style • deterministic Rust core`
