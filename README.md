<p align="center">
  <a href="https://axonos-bci.github.io/neural-boundary-game/" aria-label="Run Neural Boundary Game v3.0.0">
    <img src="preview.png" alt="Neural Boundary Game v3.0.0 — Sovereign Boundary Edition" width="100%">
  </a>
</p>

<h1 align="center">Neural Boundary Game v3.0.0</h1>
<p align="center"><strong>Sovereign Boundary Edition</strong></p>
<p align="center">Keep raw signal private. Release typed intent only.</p>

<p align="center">
  <a href="https://axonos-bci.github.io/neural-boundary-game/"><strong>RUN THE GAME</strong></a>
  · <a href="docs/GAME_SPEC.md">Game specification</a>
  · <a href="docs/ARCHITECTURE.md">Architecture</a>
  · <a href="docs/REPLAY_SPEC.md">Replay protocol</a>
  · <a href="docs/COMMERCIAL_SERVICES.md">Commercial deployment</a>
</p>

> **Status:** source-complete release candidate for `v3.0.0`. The browser experience requires the Rust/WASM build produced by `scripts/build_web.sh`; the interface deliberately refuses to substitute a JavaScript simulation when the WASM core is unavailable.

## What this is

Neural Boundary Game is an **Educational technical simulation** of a privacy boundary between a private signal domain and an application domain. The player classifies packets, converts validated intent, applies scoped consent, registers evidence, quarantines unsafe objects, and seals the boundary only when every release invariant is satisfied.

It is not a medical device, BCI driver, clinical system, diagnostic tool, stimulation controller, or representation of production neural decoding accuracy. It connects to no sensor and processes no neural data.

## Product thesis

A trusted cognitive stack must not expose raw signal as an application primitive. The application-facing unit should be a typed, consented, evidence-backed intent with explicit provenance and revocation semantics.

The game turns that boundary contract into an executable deterministic system:

- **Raw frames and artifacts remain private.** They must be quarantined.
- **Unknown packets require classification.** A validated intent still cannot cross directly.
- **Conversion requires active scoped consent and evidence.** The transition is policy-gated.
- **Revocation is immediate.** A revoked capability cannot be treated as stale authorization.
- **Unsafe stimulation commands fail closed.** Crossing the membrane terminates the run.
- **Release is an invariant check, not a score button.** Five review gates and quantitative thresholds must all pass.

## Run modes

| Mode | Purpose | Character |
|---|---|---|
| Guided | Learn the policy | Scripted pacing and wider decision windows |
| Standard | Canonical public run | Balanced deterministic challenge |
| Audit | Adversarial review | Higher density, unsafe claims, stricter evidence pressure |
| Grand | Full boundary review | Extended multi-phase run |
| Daily Seed | Shared UTC challenge | Same calendar date, same deterministic world, no backend |

## Release invariants

A release may seal only when all of the following are true:

- trust is at least `90`;
- risk is at most `20`;
- integrity is at least `80`;
- evidence is at least `L2`;
- type-safety, consent, evidence, privacy, and determinism gates are closed;
- scoped consent is active, unexpired, and grants both conversion and release scopes;
- raw leak count is zero;
- no active entity remains unresolved anywhere in the fixed-capacity pool;
- the mode-specific minimum review interval has elapsed.

The authoritative implementation is in `crates/neural-boundary-core`. Browser code only renders state and submits explicit input events.

## Architecture

```text
keyboard / touch / pointer
          │
          ▼
      web/app.js                 presentation, accessibility, audio, canvas
          │ plain numeric ABI
          ▼
neural-boundary-web.wasm         C-compatible adapter, no policy logic
          │
          ▼
neural-boundary-core             no_std deterministic authority
          │
          ├── fixed 60 Hz ticks
          ├── seeded RNG
          ├── consent/evidence policy
          ├── terminal conditions
          └── canonical FNV-1a state hash
```

There is no second JavaScript game engine. Missing or incompatible WASM produces a visible fail-closed error.

## Repository map

```text
crates/neural-boundary-core/   allocation-free deterministic state machine
crates/neural-boundary-web/    dependency-light WebAssembly ABI
crates/neural-boundary-cli/    replay verifier and vector tooling
web/                           browser presentation and tests
vectors/                       canonical replay conformance vectors
docs/                          protocol, architecture, UX, safety and release docs
tools/                         version, hygiene and replay gates
scripts/                       build, smoke, local serve and Termux release helpers
.github/workflows/             CI, Pages and tagged-release automation
```

## Build and run

Requirements:

- Rust `1.81.0` with `wasm32-unknown-unknown`;
- Python `3.11+`;
- Node.js `20+` for browser adapter tests;
- a static HTTP server for local WASM loading.

```bash
rustup target add wasm32-unknown-unknown
./scripts/verify_release.sh --full
./scripts/build_web.sh
./scripts/serve_dist.sh
```

Open the local URL printed by the server. Direct `file://` loading is not supported because browsers restrict WebAssembly module loading.

### Deterministic verification

```bash
cargo run --locked -p neural-boundary-cli -- verify-all
python3 tools/validate_replay.py
python3 tools/check_version_consistency.py
python3 tools/check_hygiene.py
python3 tools/deep_audit.py
```

Canonical replay schema: `neural-boundary-replay-v3.0.0`

State hash: `fnv1a64-v1`
Tick rate: `60 Hz`

## Controls

| Input | Action |
|---|---|
| `W/S` or arrows | Select lane |
| `1` | Validate |
| `2` | Convert |
| `3` | Quarantine |
| `4` | Consent |
| `5` | Evidence |
| `Enter` | Release / seal attempt |
| `P` or `Space` | Pause |
| `R` | Restart current run |
| `H` | Open protocol reference |

Touch controls expose the same six actions and lane selection without hover-dependent affordances.

## Privacy and security posture

The public build is local-only. It contains no analytics SDK, ad network, remote font, telemetry endpoint, account system, cookie, wallet connection, or neural-data ingestion path. Browser persistence is namespaced under `axonos_nbg_v300_` and can be deleted from the UI.

The Content Security Policy limits runtime resources to the same origin. Production deployments should additionally set equivalent HTTP response headers; HTML meta policy is defense in depth, not a replacement for server headers.

See [SECURITY.md](SECURITY.md), [Threat Model](docs/THREAT_MODEL.md), [No Raw Neural Data](docs/NO_RAW_NEURAL_DATA.md), and [BCI Boundary](docs/BCI_BOUNDARY.md).

## Commercial deployment

The open-source edition demonstrates the boundary model and deterministic verification surface. AxonOS commercial work may include private deployment, customer-specific policy packs, hardware gateway integration, conformance engineering, security review, evidence pipelines, and support. Commercial services do not change the open-source license or imply that this educational game is a medical product.

See [Commercial Services](docs/COMMERCIAL_SERVICES.md).

## Release integrity

`release.toml` is the canonical release identity. CI rejects divergent versions, stale active version strings, unresolved merge markers, remote runtime assets, invalid replay vectors, broken internal links, and incomplete release surfaces.

The ABI is documented in [docs/ABI_CONTRACT.md](docs/ABI_CONTRACT.md). The release sequence is documented in [docs/RELEASE_PROCESS.md](docs/RELEASE_PROCESS.md). Do not tag `v3.0.0` until the exact commit passes CI and Pages smoke tests.

## License

Licensed under either of:

- Apache License, Version 2.0; or
- MIT License

at your option: **MIT OR Apache-2.0**.

AxonOS names, logos, and product marks are not granted by the code license. See the license files for terms.
