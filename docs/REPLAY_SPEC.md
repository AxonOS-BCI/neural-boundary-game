<!-- SPDX-FileCopyrightText: 2026 Denis Yermakou
SPDX-FileContributor: AxonOS
SPDX-License-Identifier: CC-BY-NC-ND-4.0 -->

# Replay Specification — Neural Boundary Game v7.3.0

**Schema**: `neural-boundary-replay-v7.3.0`
**Hash**: `fnv1a64-v1` · **RNG**: `xorshift64star-v1` · **ABI**: 1

## File format

JSON object (deny-unknown-fields for verifier). Max 1 MiB. Max 10 000 input events.
Inputs must have strictly increasing ticks. No duplicate (tick, action) pairs.

## Required top-level fields

```json
{
  "schema": "neural-boundary-replay-v7.3.0",
  "product_version": "7.3.0",
  "core_version": "7.3.0",
  "abi_version": 1,
  "hash_algorithm": "fnv1a64-v1",
  "rng_algorithm": "xorshift64star-v1",
  "tick_rate_hz": 60,
  "mode": "STANDARD",
  "difficulty": 1,
  "seed": "000000000000001f",
  "title": "...",
  "generated_by": "...",
  "inputs": [...],
  "expected": {...}
}
```

`mode` is uppercase: GUIDED, STANDARD, AUDIT, GRAND, DAILY, PRIVACY_VAULT, KERNEL_TRIAL.
`difficulty` is 0=Calm, 1=Standard, 2=Intense.
`seed` is exactly 16 lowercase hex digits (no 0x prefix).
Daily replays require a `date` field ("YYYY-MM-DD"); verifier recomputes seed.

## Input events

```json
{ "tick": 42, "lane": 2, "action": "VALIDATE" }
```

Actions: VALIDATE, CONVERT, QUARANTINE, CONSENT, EVIDENCE, RELEASE.
Lanes 0-4.

## Expected block

```json
"expected": {
  "terminal_tick": 3408,
  "status": "SEALED",
  "terminal_reason": "SUCCESS_RELEASE",
  "grade": "SEALED",
  "trust": 742, "risk": 183, "integrity": 868,
  "evidence_level": "L2",
  "evidence_bits": 3,
  "gate_mask": 127,
  "gates_passed": 7,
  "raw_leaks": 0,
  "typed_intents": 5,
  "quarantined": 12,
  "wrong_actions": 0,
  "score": 9322,
  "best_combo": 8,
  "revocations": 0,
  "state_hash": "0x75bf9ada81839bc1"
}
```

`state_hash` is `"0x"` + 16 lowercase hex digits.

## Canonical vectors

8 mandatory vectors in `vectors/`, verified by `vectors/checksums.sha256` (SHA-256 per file).
Commands: `neural-boundary-cli verify-all` — checks checksums then replays all 8.
