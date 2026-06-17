<!-- SPDX-FileCopyrightText: 2026 Denis Yermakou
SPDX-FileContributor: AxonOS
SPDX-License-Identifier: CC-BY-NC-ND-4.0 -->

# Architecture — Neural Boundary Game v7.3.0

## Crate structure

```
neural-boundary-game/
├── crates/
│   ├── neural-boundary-core/   # no_std deterministic simulation
│   ├── neural-boundary-cli/    # replay verifier and conformance toolkit
│   └── neural-boundary-web/    # flat WASM ABI (41 exports, no wasm-bindgen)
├── web/                        # JavaScript UI (ES modules)
│   ├── app.js                  # state machine, RAF loop
│   ├── abi.js                  # typed WASM wrapper
│   ├── render.js               # canvas renderer
│   ├── hud.js                  # DOM metrics, feedback grammar
│   ├── storage.js              # axonos_nbg_v5512_* local storage
│   └── a11y.js                 # WCAG 2.2 AA helpers
├── vectors/                    # 8 canonical replay vectors
└── scripts/build_web.sh        # production build (no bundler)
```

## Trust hierarchy

The Rust/WASM core is authoritative. The browser receives read-only snapshots.
No game rule exists in JavaScript; all determinism lives in the core.

## WASM ABI (§26.1)

41 named exports, all `#[unsafe(no_mangle)] extern "C"`. No wasm-bindgen.
`nbg_init(mode, seed_hi, seed_lo, difficulty) → u32`
`nbg_step(ticks) → u32`
`nbg_apply_action(lane, action_id) → u32`
Entity pool queried per-slot: `nbg_entity_kind/lane/x/flags(slot) → u32`.
Seed passed as (hi: u32, lo: u32) pair; combined as `(hi as u64) << 32 | lo`.

## State hash

fnv1a64-v1 little-endian over: version strings, seed, mode, difficulty,
RNG state, tick, phase, status, reason, lane, all 0..1000 metrics,
consent (epoch+scope+expires), evidence_bits, vault, gate_mask, WCET peak,
all counters, and all 32 entity slots (ordered by slot_id).

## Replay verification

`neural-boundary-cli verify-all`:
1. SHA-256 integrity of each vector file against `vectors/checksums.sha256`
2. Deterministic re-execution: same seed + same inputs → identical state_hash

Across all 8 canonical vectors in CI and offline.
