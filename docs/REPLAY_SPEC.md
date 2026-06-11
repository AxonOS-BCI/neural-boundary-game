<<<<<<< HEAD
# Replay Spec — neural-boundary-replay-v2.1.2

A replay vector pins one deterministic run of the core: seed, difficulty, the
full input script, and the expected terminal state. `neural-boundary-cli
verify` re-executes the script against `neural-boundary-core` and compares
every expected field, including a 64-bit state hash.

Current schema:

```text
neural-boundary-replay-v2.1.2
```

## File layout

```json
{
  "schema": "neural-boundary-replay-v2.1.2",
  "title": "Canonical clean run — boundary sealed",
  "generated_by": "neural-boundary-cli record --policy clean --seed 58 --difficulty standard",
  "seed": 58,
  "difficulty": "standard",
  "actions": [
    { "tick": 130, "lane": 2, "action": "ConsentGate" }
  ],
  "expected": {
    "final_tick": 1862,
    "trust": 92,
    "risk": 12,
    "integrity": 88,
    "evidence_level": "L3",
    "raw_leaks": 0,
    "gates_passed": 5,
    "status": "victory",
    "boundary": "SEALED",
    "state_hash": "0x3385d1c44b271154"
  }
}
```

Rules:

- `seed` is a positive integer; `difficulty` is `calm | standard | intense`.
- `actions[*].tick` is strictly increasing, ≥ 1 and ≤ `expected.final_tick`;
  `lane` is 0–4; `action` is one of `Validate`, `Convert`, `Quarantine`,
  `ConsentGate`, `EvidenceGate`, `Release`.
- `expected.boundary` is `SEALED` (victory), `BREACHED` (defeat) or `HOLDING`.
- Defeat vectors may add `expected.cause`:
  `integrity_collapse | risk_overflow | raw_leaks | direct_stim`.
- `state_hash` is `0x` + 16 lowercase hex digits.

## State hash

FNV-1a (64-bit, offset basis `0xcbf29ce484222325`, prime `0x100000001b3`)
over, in order: seed, difficulty code, RNG state, tick, status code, selected
lane, trust, risk, integrity, evidence points, consent deadline, gate mask,
raw leaks, validated/quarantined/delivered/reveal/miss/false-positive/breach
counters, cooldown, spawn timer, spawn count, next id, and every entity slot
(id, kind code, lane, x, speed, validated flag, concealed kind). Any
divergence anywhere in the run changes the final hash.

## Verifier semantics

```bash
cargo run -p neural-boundary-cli --release -- verify            # default vector
cargo run -p neural-boundary-cli --release -- verify vectors/replay-breach-demo-v2.1.2.json
```

On success the verifier prints exactly:

```text
Replay OK
Final trust: 92
Final risk: 12
Final integrity: 88
Boundary status: SEALED
```

On mismatch it prints `Replay FAILED` plus a field-by-field diff and exits 1.

## Shipped vectors

Both vectors use **seed 58**, standard difficulty — the same world twice:

| Vector | Policy | Result |
|---|---|---|
| `replay-v2.1.2.json` | `clean` (38 actions) | victory at tick 1862 — trust 92, risk 12, integrity 88, L3, 5/5 gates, 0 leaks, `SEALED` |
| `replay-breach-demo-v2.1.2.json` | `idle` (0 actions) | defeat at tick 948 — third raw leak crosses, `BREACHED` (`cause: raw_leaks`) |

The only difference between the two runs is boundary discipline.

## Regenerating vectors

```bash
cargo run -p neural-boundary-cli --release -- record \
  --seed 58 --difficulty standard --policy clean \
  --title "Canonical clean run — boundary sealed" \
  --out vectors/replay-v2.1.2.json

cargo run -p neural-boundary-cli --release -- record \
  --seed 58 --difficulty standard --policy idle \
  --out vectors/replay-breach-demo-v2.1.2.json

cargo run -p neural-boundary-cli --release -- search \
  --from 1 --to 30000 --difficulty standard --target 92,12,88
```

After regenerating, refresh `vectors/checksums.txt`
(`sha256sum vectors/*.json`) and run `python3 tools/validate_replay.py`.
Game-rule changes require new vectors and a schema bump.
=======
# Replay Spec

Current replay schema:

```text
neural-boundary-replay-v2.0.0
```

Replay vectors are validated by `tools/validate_replay.py`.

Checksums are validated by `tools/release_check.py`.
>>>>>>> origin/main
