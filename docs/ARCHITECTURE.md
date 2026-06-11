# Architecture

## Design objective

Neural Boundary Game must prove one architectural point through behavior: the application layer never owns raw signal and never owns the policy engine that decides what may cross the boundary.

## Trust boundaries

### Private signal domain

Contains raw frames, artifacts, unknown packets, private classification context, and unsafe stimulation commands. Objects in this domain are not application payloads.

### Sovereign boundary

Owns validation, conversion, consent, evidence, quarantine, release invariants, deterministic scheduling, and terminal policy.

### Application domain

Receives only typed intent that has survived the policy path. In the game this is a visual zone, not a real application integration.

## Component model

### `neural-boundary-core`

The authoritative state machine. It is `no_std`, allocation-free during simulation, integer-based, bounded, and independent of browser APIs. It owns:

- fixed tick progression;
- entity spawning and movement;
- seeded randomness;
- input validation;
- consent activation, scope, expiry, and revocation;
- evidence maturity;
- trust, risk, integrity, score, leak and streak metrics;
- review gates;
- release blockers;
- terminal status and reason;
- canonical state hash.

### `neural-boundary-web`

A narrow ABI adapter compiled to WebAssembly. It converts stable numeric calls into core methods and exposes read-only accessors. It must not duplicate policy.

### `web/app.js`

A presentation adapter. It loads WASM, schedules fixed-step advancement, renders canvas and DOM state, handles keyboard/touch input, manages local preferences, and exposes accessibility text. It may interpolate visuals but may not invent authoritative state.

### `neural-boundary-cli`

A native verifier for the same core. It loads replay JSON, applies events, advances to the expected terminal tick, and compares every expected field and state hash.

## Determinism contract

For the same core version, replay schema, mode, difficulty, seed, initial configuration, and ordered input events, the terminal state must be identical across supported native and WASM targets.

Authoritative calculations use integers. Browser frame time never enters the core. The JS loop accumulates elapsed time and advances in fixed 60 Hz steps with a bounded catch-up limit.

## Memory and timing discipline

The core uses a fixed array of entity slots and a known upper bound on per-tick iteration. It performs no heap allocation during a run. This supports predictable execution structure, but the repository does **not** claim certified WCET, hard-real-time operation, or suitability for safety-critical hardware.

## Fail-closed behavior

- Unknown enum values map to conservative defaults.
- Invalid release attempts remain blocked.
- Unsafe stimulation crossing terminates immediately.
- Missing WASM displays a fatal state instead of running a substitute engine.
- Internal invariant failure maps to an unsafe terminal condition.

## Version boundaries

`release.toml` defines product identity. Replay compatibility is explicit through `neural-boundary-replay-v3.0.0`. A change that affects deterministic output requires a new schema or documented compatibility decision.


## ABI handshake

The browser accepts a module only when all 41 canonical exports are present and the runtime metadata matches the compiled product contract: ABI version `3_000_000`, tick rate `60`, lane count `5`, and boundary X `840`. A missing, stale, or extra policy path is treated as a fatal incompatibility; JavaScript never emulates the core.

The canonical export list is owned by `web/app.js`, statically compared with Rust by `tools/deep_audit.py`, and exercised against the built module by `web/tests/wasm-smoke.mjs`. See [ABI Contract](ABI_CONTRACT.md).

## Release completeness

A state can seal only after all review gates pass, required consent scopes are active, quantitative thresholds pass, raw leaks are zero, the mode review interval has elapsed, and the complete entity pool is empty. This prevents a release decision from ignoring unresolved off-lane or off-screen state.


## Unsafe-code boundary

The deterministic core forbids unsafe Rust at crate level. The WASM adapter must permit `#[no_mangle]` export attributes, which are covered by Rust's broad `unsafe_code` lint on the pinned toolchain; it does not permit unsafe blocks, unsafe functions, unsafe impls, or mutable statics. `tools/deep_audit.py` enforces this narrow exception in addition to compiler lints.
