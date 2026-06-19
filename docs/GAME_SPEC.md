<!-- SPDX-FileCopyrightText: 2026 Denis Yermakou
SPDX-FileContributor: AxonOS
SPDX-License-Identifier: CC-BY-NC-ND-4.0 -->

# Neural Boundary Game v8.0.1 — Game Specification

**Canonical schema**: `neural-boundary-replay-v8.0.1`
**RNG**: `xorshift64star-v1` · **Hash**: `fnv1a64-v1` · **Tick rate**: 60 Hz

## §7 Field geometry

Logical field 1024×576. Boundary at x=704. Action window 544..703.
Signal zone 0..543. Application zone 705..1024. Five lanes (y=96/192/288/384/480).
Positions stored as Q24.8 (stored_x = logical_x × 256). Pool capacity: 32 slots (min-free-slot policy).

## §8 RNG

xorshift64star-v1: `x ^= x>>12; x ^= x<<25; x ^= x>>27; return x * 0x2545F4914F6CDD1D`.
Seed 0 remapped to `0x9E3779B97F4A7C15`.

Daily seed: FNV-1a 64-bit over `"NBG|8.0.1|YYYY-MM-DD|DAILY"` then one xorshift64star round.

## §9 Entity taxonomy (19 stable IDs)

| ID | Kind | Correct action |
|----|------|---------------|
| 0 | EMPTY | — |
| 1 | RAW_FRAME | QUARANTINE |
| 2 | ARTIFACT | QUARANTINE |
| 3 | UNKNOWN_PACKET | VALIDATE |
| 4 | CANDIDATE_INTENT | VALIDATE |
| 5 | VALIDATED_INTENT | CONVERT |
| 6 | TYPED_INTENT | (crosses freely) |
| 7 | CONSENT_GRANT | CONSENT |
| 8 | CONSENT_REVOKE | CONSENT |
| 9 | EVIDENCE_TRACE | EVIDENCE |
| 10 | CHECKSUM_PROOF | EVIDENCE |
| 11 | CI_PROOF | EVIDENCE |
| 12 | UNSUPPORTED_CLAIM | QUARANTINE |
| 13 | UNTRACEABLE_CLAIM | QUARANTINE |
| 14 | ROADMAP_AS_FACT | QUARANTINE |
| 15 | STIMULATION_COMMAND | QUARANTINE |
| 16 | DEADLINE_HAZARD | VALIDATE |
| 17 | VAULT_RECORD | QUARANTINE |
| 18 | RAW_EXPORT_REQUEST | QUARANTINE |

## §10 Actions

VALIDATE(1), CONVERT(2), QUARANTINE(3), CONSENT(4), EVIDENCE(5), RELEASE(6).

CONVERT requires: CONSENT_GRANT scope active, epoch matching, not expired, evidence ≥ mode.convert_evidence. Violation: risk+100, integrity-60, trust-80, combo=0.

NO_TARGET: combo=0, risk+10 (except RELEASE).

RELEASE: checks all 7 gates + consent has RELEASE scope + phase≥3 (Grand only).
Release cooldown: 30 ticks. Spam: risk+5.

## §11 Neural Permissions (Consent epoch model)

Scope bits: CONVERT=0x0001, RELEASE=0x0002, LOCAL_SUMMARY=0x0004, AUDIT=0x0008.
Token valid when: epoch == state.consent_epoch AND tick < expires AND scope ⊆ mask.
CONSENT_REVOKE: epoch += 1, active_scope = 0, expires = 0 (immediate).

## §12 Evidence chain (strict ordering)

L0 → TRACE(bit 0x01) → L1 → CHECKSUM(bit 0x02) → L2 → CI(bit 0x04) → L3.
Out-of-order proof is rejected. Escaped claims block the EVIDENCE gate.

## §13 Privacy Vault

States: EMPTY/OPEN/SEALING/SEALED/COMPROMISED.
Raw frame quarantine: EMPTY→SEALING→SEALED. Crossing: COMPROMISED.
Vault gate requires {EMPTY, SEALED} AND pending_raw=0 AND export_violations=0.

## §15 WCET budget

Budget: 618 logical units per tick.
base=40, entity=4/each, VALIDATE=55, CONVERT=70, QUARANTINE=45, CONSENT=50, EVIDENCE=50, RELEASE=120, spawn=12.
Gate fails if wcet_peak > 618 OR deadline_misses > 0 OR capacity_pressure ≥ 3.

## §16 7 Review Gates

| Bit | Gate | Condition |
|-----|------|-----------|
| 0x01 | PRIVACY | raw_leaks=0, pending_raw=0, export_violations=0 |
| 0x02 | TYPING | unvalidated_conversions=0 |
| 0x04 | CONSENT | consent_violations=0 |
| 0x08 | EVIDENCE | level≥mode.release_evidence AND escaped_claims=0 |
| 0x10 | DETERMINISM | no replay faults |
| 0x20 | VAULT | vault∈{EMPTY,SEALED}, pending_raw=0 |
| 0x40 | WCET | peak≤618, deadline_misses=0, capacity_pressure<3 |

## §17 Metrics and scoring

All metrics 0..1000. Initial values:
- Guided: trust=800, risk=50, integrity=1000
- Standard/Daily/Grand/Vault: trust=700, risk=100, integrity=1000
- Audit/Kernel: trust=650, risk=150, integrity=1000

Scoring formula (§17.5):
```
safety_margin = clamp((704 - entity_x) / 2, 0, 80)
combo_percent = 100 + 5 * min(combo, 20)
delta = (base_score + safety_margin) * combo_percent / 100
```

Base scores: VALIDATE=100, CONVERT=180, QUARANTINE=140, CONSENT=120, EVIDENCE=160, RELEASE=1000.

## §18 Terminal model

Status: RUNNING(0), SEALED(1), BREACHED(2), UNSAFE(3), ABORTED(4), FATAL_RUNTIME(5).
Reasons: SUCCESS_RELEASE, TIMEOUT_UNSEALED, RISK_OVERFLOW, INTEGRITY_COLLAPSE, RAW_LEAK_LIMIT, UNSAFE_STIMULATION_ESCAPE, DEADLINE_BREACH, DETERMINISM_MISMATCH, REPLAY_SCHEMA_ERROR, WASM_INIT_FAILURE, USER_ABORT.

## §19 Grades

| Grade | Condition |
|-------|-----------|
| SOVEREIGN | SEALED + all 7 gates + trust≥900 + risk≤100 + integrity≥900 + L3 + leaks=0 + wrong=0 |
| SEALED | SEALED + all 7 gates + trust≥750 + risk≤250 + integrity≥750 + leaks=0 |
| REVIEWABLE | SEALED or ABORTED, 5+ gates, integrity≥650, risk≤450 |
| DEGRADED | 3+ gates, integrity>0, risk<1000 |
| BREACHED | Status BREACHED |
| UNSAFE | Status UNSAFE or FATAL_RUNTIME |

## §20 Run modes

| Mode | Max ticks | Leak limit | Evidence req |
|------|-----------|------------|--------------|
| GUIDED(1) | 3600 | 2 | L1/L2 |
| STANDARD(2) | 7200 | 2 | L1/L2 |
| AUDIT(3) | 7200 | 1 | L2/L3 |
| GRAND(4) | 10800 | 1 | L2/L3 |
| DAILY(5) | 7200 | 2 | L1/L2 |
| PRIVACY_VAULT(6) | 7200 | 1 | L2/L3 |
| KERNEL_TRIAL(7) | 6000 | 1 | L2/L3 |

## §24 State hash (fnv1a64-v1)

Little-endian FNV-1a 64-bit over version strings, seed, mode, difficulty, RNG state, tick, phase, status, reason, lane, all metrics, consent, evidence, vault, gates, counters, and all 32 entity slots (in slot order).
