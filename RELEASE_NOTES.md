# Neural Boundary Game v3.0.0 — Sovereign Boundary Edition

`v3.0.0` is a ground-up product and architecture release. It replaces the fragmented legacy surface with one deterministic Rust/WASM system, one release identity, one replay protocol, and one verified GitHub Pages artifact.

## Release identity

- Product: Neural Boundary Game
- Version: `v3.0.0`
- Tag: `v3.0.0`
- Replay schema: `neural-boundary-replay-v3.0.0`
- State hash: `fnv1a64-v1`
- Storage namespace: `axonos_nbg_v300_`
- License: `MIT OR Apache-2.0`

## Product result

The release provides a complete interactive simulation rather than a banner that opens a static asset. A run starts in the browser, advances through the Rust/WASM authority at 60 deterministic ticks per second, accepts six policy actions, exposes five review gates, and produces a terminal state hash.

The UI includes Guided, Standard, Audit, Grand, and Daily Seed modes; responsive mobile controls; canvas letterboxing that preserves entity geometry; accessible state mirrors; local sound/haptics; protocol guidance; and explicit fail-closed handling when the WASM core cannot load.

## Technical result

- Allocation-free `no_std` core with bounded arrays and seeded xorshift64* RNG.
- Plain C-style WASM ABI with no `wasm-bindgen` runtime requirement.
- CLI verifier for canonical replay vectors.
- Eight vectors covering successful seal, idle breach, consent revocation, raw leak, stimulation fail-closed, Audit, Grand, and Daily Seed runs.
- CI gates for formatting, linting, tests, replay equivalence, active-version consistency, claim hygiene, links, Pages artifact structure, and browser adapter tests.

## Security and claim boundary

This is an educational technical simulation. It has no BCI sensor path, medical function, neural decoder, stimulation interface, telemetry, or remote account system. Commercial integrations require a separate engineering and assurance scope.

## Release acceptance

A tagged release is valid only when:

1. the tag points to the exact reviewed commit;
2. CI is green;
3. all canonical replays match Rust output;
4. the Pages artifact contains the compiled WASM module;
5. browser smoke tests confirm that Run starts an interactive session;
6. active release files contain no stale version or conflict marker;
7. the public URL is checked on desktop and mobile viewports.

## Final hardening

- Exact 41-export Rust/JavaScript/WASM ABI contract with version, tick-rate, lane-count, and boundary metadata handshake.
- Scoped consent enforcement separates conversion authority from release authority.
- Release is blocked until the complete fixed-capacity entity pool is empty.
- Strict replay inputs reject unknown fields, oversized payloads/event streams, malformed hashes, invalid ordering, and out-of-range values.
- Reproducible source and Pages packaging normalizes paths, timestamps, ownership, modes, and ordering.
- Deep audit and browser lifecycle tests cover accessibility references, keyboard navigation, help pause/resume, repeat announcements, workflow permissions, and source hygiene.
- Validation disables Python bytecode generation and fails if any cache artifact appears after a gate.
