# WebAssembly ABI Contract — v3.0.0

## Purpose

`neural-boundary-web.wasm` is a narrow numeric adapter over the authoritative Rust core. It owns no alternate policy engine. The JavaScript presentation layer must fail closed when this contract is incomplete or incompatible.

## Runtime handshake

Before starting a run, the loader verifies:

- ABI version: `3_000_000`;
- tick rate: `60`;
- lane count: `5`;
- logical boundary X: `840`;
- presence of every canonical function export.

## Canonical exports

The contract contains 41 functions:

```text
nbg_abi_version
nbg_tick_rate
nbg_lane_count
nbg_boundary_x
nbg_init
nbg_daily_seed_low
nbg_daily_seed_high
nbg_tick
nbg_action
nbg_select_lane
nbg_move_lane
nbg_tick_value
nbg_selected_lane
nbg_trust
nbg_risk
nbg_integrity
nbg_evidence
nbg_review_gates
nbg_raw_leaks
nbg_raw_leak_limit
nbg_score
nbg_streak
nbg_best_streak
nbg_status
nbg_terminal_reason
nbg_feedback
nbg_release_blockers
nbg_release_ready
nbg_consent_active
nbg_consent_scope
nbg_consent_expiry_tick
nbg_state_hash_low
nbg_state_hash_high
nbg_entity_capacity
nbg_active_entity_count
nbg_entity_active
nbg_entity_id
nbg_entity_kind
nbg_entity_lane
nbg_entity_position
nbg_entity_flags
```

## Numeric safety

All inputs are validated before narrowing. Unknown mode, difficulty, and action values select explicit conservative behavior rather than wrapping through integer casts. Lane and entity access are clamped or rejected at the boundary.

## Verification ownership

- `web/app.js` exports the canonical JavaScript list and validates runtime metadata.
- `tools/deep_audit.py` compares the JavaScript list with Rust `#[no_mangle]` exports.
- `web/tests/wasm-smoke.mjs` validates the actual built module and boundary behavior.
- `scripts/verify_release.sh --full` is the release gate.

Changing any export name, meaning, type, constant, or state transition is an ABI change and requires compatibility review.
