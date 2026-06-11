<<<<<<< HEAD
# Game Spec — Neural Boundary Game v2.1.2

Deterministic specification of the simulation implemented in
`crates/neural-boundary-core` (`#![no_std]`, `#![forbid(unsafe_code)]`).
Everything below is normative: the WASM front-end, the CLI verifier and the
replay vectors all derive from this model. Version: 2.1.2.

Core rule:
=======
# Game Spec

The game demonstrates a single boundary rule:
>>>>>>> origin/main

```text
Do not ship raw signal.
Ship typed intent.
```

<<<<<<< HEAD
## 1. Field model

| Constant | Value | Meaning |
|---|---|---|
| `TICKS_PER_SECOND` | 60 | fixed simulation step |
| `LANES` | 5 | parallel signal lanes |
| `SPAWN_X` | 0 | entities enter on the left |
| `BOUNDARY_X` | 680 | the membrane between signal zone and app zone |
| `FIELD_END_X` | 1000 | application threshold (right edge) |
| `GATE_WINDOW` | 320 | actions only reach the frontmost entity inside `[BOUNDARY_X − 320, BOUNDARY_X)` of the selected lane |
| `MAX_ENTITIES` | 24 | fixed entity pool (no allocation) |
| `ACTION_COOLDOWN` | 18 ticks | after a successful action |
| `WHIFF_COOLDOWN` | 6 ticks | after an action with no valid target |
| `CONSENT_DURATION` | 1500 ticks (25 s) | consent window after gating a token |
| `CLAIM_SPEED_BONUS` | +2 | unsupported claims travel faster than evidence |

Positions are integer units; an entity at `x ≥ BOUNDARY_X` has crossed the
membrane. `TypedIntent` is the only kind that may legally cross; it counts as
delivered when it reaches `FIELD_END_X`.

## 2. Entity taxonomy

| Kind | Label | Class | Evidence pts | Crossing the membrane |
|---|---|---|---|---|
| `IntentCandidate` | `INTENT` | artifact | — | missed intent, trust −1 |
| `ConsentToken` | `CONSENT` | artifact | — | artifact lost |
| `Evidence` | `EVIDENCE` | artifact | 2 | artifact lost |
| `Checksum` | `CHECKSUM` | artifact | 1 | artifact lost |
| `CiTest` | `CI TEST` | artifact | 1 | artifact lost |
| `TypedIntent` | `TYPED` | typed | — | legal; delivery at field end, trust +6 |
| `RawLeak` | `RAW` | hazard | — | raw leak: integrity −10, leak counter +1 |
| `DirectStim` | `STIM` | hazard | — | immediate boundary breach (defeat) |
| `UnsafeBlock` | `UNSAFE` | hazard | — | major breach: integrity −8, risk +6 |
| `Unbounded` | `UNBOUND` | hazard | — | major breach: integrity −8, risk +6 |
| `Overclaim` | `CLAIM` | claim | — | minor breach: integrity −6 |
| `NoTrace` | `NO TRACE` | claim | — | minor breach: integrity −6 |
| `RoadmapFact` | `ROADMAP` | claim | — | minor breach: integrity −6 |
| `UnknownPacket` | `?PKT` | unknown | — | resolves to its concealed kind |

Claims carry `CLAIM_SPEED_BONUS`: unsupported statements move faster than
evidence, so containment is a real-time discipline, not an afterthought.
`UnknownPacket` conceals one of the other kinds; it cannot be quarantined
until it is classified with **Validate**.

## 3. Actions

All actions apply to the frontmost entity inside the gate window of the
selected lane. A successful action starts `ACTION_COOLDOWN`; an action with no
valid target starts `WHIFF_COOLDOWN`.

| # | Action | Effect |
|---|---|---|
| 1 | `Validate` | types an `IntentCandidate` (marks it validated) or classifies a `?PKT` (reveals concealed kind, risk +4) |
| 2 | `Convert` | validated `IntentCandidate` → `TypedIntent`; requires active consent and evidence ≥ L1 |
| 3 | `Quarantine` | removes a hazard or claim; quarantining an artifact is a false positive (trust −3); `?PKT` must be classified first |
| 4 | `ConsentGate` | gates a `ConsentToken`: consent active for `CONSENT_DURATION` |
| 5 | `EvidenceGate` | logs `Evidence` / `Checksum` / `CiTest`, adding its evidence points |
| ⏎ | `Release` | attempts to seal the boundary (see §6) |

## 4. Metrics

Start of run: trust 50, risk 0, integrity 100, evidence 0 pts, gates 0/5.

| Event | Δ |
|---|---|
| typed intent delivered | trust +6 |
| intent candidate missed at membrane | trust −1 |
| false-positive quarantine | trust −3 |
| `?PKT` classified | risk +4 |
| major breach (`UNSAFE`, `UNBOUND` cross) | risk +6, integrity −8 |
| release rejected | risk +6 |
| minor breach (claim crosses) | integrity −6 |
| raw leak (`RAW` crosses) | integrity −10, raw-leak counter +1 |

Evidence levels: `L0` < 2 pts · `L1` 2–3 · `L2` 4–6 · `L3` ≥ 7.

## 5. Review gates

| Gate | Name | Passes when |
|---|---|---|
| 0 | `SCHEMA` | 3 successful validations |
| 1 | `CONSENT` | first consent gate opened |
| 2 | `EVIDENCE` | evidence level reaches L2 |
| 3 | `CONTAIN` | 3 hazards/claims quarantined |
| 4 | `DELIVERY` | 5 typed intents delivered |

## 6. Win and lose conditions

`Release` seals the boundary (victory) only if **all** hold:

```text
TRUST ≥ 90 · RISK ≤ 20 · INTEGRITY ≥ 80 · EVIDENCE ≥ L2 (4 pts)
ALL 5 GATES PASSED · RAW LEAKS = 0
```

A rejected release reports the first blocker and adds risk +6.

Defeat (boundary breach) on any of:

```text
INTEGRITY ≤ 0 · RISK ≥ 100 · RAW LEAKS ≥ 3 · any DIRECT STIM crossing
```

Victory copy: “REVIEWER SURVIVAL ACHIEVED / The boundary held. / Raw signal
stayed inside. / Applications received typed intent only.”
Failure copy: “BOUNDARY BREACH / Raw signal or unsafe claims crossed the
boundary. / Fix the evidence path. Seal the interface. Try again.”

## 7. Warm-up wave

Before random traffic, six scripted entities teach the loop (kind, lane):

```text
1. CONSENT  lane 2     4. INTENT    lane 3
2. INTENT   lane 2     5. CHECKSUM  lane 1
3. EVIDENCE lane 1     6. ?PKT      lane 0   (conceals RAW)
```

Consent arrives before intent: the player can open the gate, validate,
log evidence and convert in the order the boundary itself demands.

## 8. Spawn system

After the warm-up the spawner draws kinds from a per-mille table:

```text
INTENT 230 · CONSENT 90 · EVIDENCE 80 · CHECKSUM 60 · CI TEST 60
RAW 90 · STIM 40 · UNSAFE 60 · UNBOUND 50
CLAIM 80 · NO TRACE 50 · ROADMAP 50 · ?PKT 60
```

Difficulty controls cadence and speed (intervals in ticks):

| Difficulty | base interval | jitter | min interval | shrink every | speed |
|---|---|---|---|---|---|
| Calm | 68 | ±20 | 44 | 700 ticks | 2 + 0..1 |
| Standard | 52 | ±18 | 30 | 600 ticks | 2 + 0..2 |
| Intense | 40 | ±14 | 22 | 480 ticks | 2 + 0..3 |

The interval floor shrinks by one step every `shrink_every` ticks, so long
runs get denser. Claims additionally receive `CLAIM_SPEED_BONUS`.

## 9. Determinism

- RNG: `xorshift64*` (multiplier `0x2545F4914F6CDD1D`); seed 0 is remapped.
- One `GameState::step(Input)` per tick; `Input` is `{select_lane, action}`.
- State hash: FNV-1a (64-bit) over seed, difficulty, RNG state, tick, status,
  selected lane, all metrics and counters, cooldown, spawn timer and the full
  entity pool — see `docs/REPLAY_SPEC.md`.
- Same seed + same input script ⇒ identical run, hash-for-hash, on every
  platform. The shipped vectors in `vectors/` pin this guarantee in CI.
=======
A successful release requires high trust, low risk, high integrity, evidence gates, and zero raw leaks.
>>>>>>> origin/main
