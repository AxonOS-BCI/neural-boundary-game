#!/usr/bin/env python3
<<<<<<< HEAD
"""Validate replay vectors for Neural Boundary Game v2.1.2.

Checks every vectors/*.json file:
  * schema == neural-boundary-replay-v2.1.2
  * seed is a positive integer, difficulty is calm|standard|intense
  * actions have strictly increasing ticks >= 1, lanes 0..4, known action names
  * no action is scheduled past expected.final_tick
  * expected block carries every required field with sane types
  * sha256 of each vector matches vectors/checksums.txt, and every vector
    is listed there

Exits non-zero on the first family of failures, printing all of them.
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VECTORS = ROOT / "vectors"
SCHEMA = "neural-boundary-replay-v2.1.2"
DIFFICULTIES = {"calm", "standard", "intense"}
ACTIONS = {"Validate", "Convert", "Quarantine", "ConsentGate", "EvidenceGate", "Release"}
EXPECTED_FIELDS = {
    "final_tick": int,
    "trust": int,
    "risk": int,
    "integrity": int,
    "evidence_level": str,
    "raw_leaks": int,
    "gates_passed": int,
    "status": str,
    "boundary": str,
    "state_hash": str,
}


def fail(errors: list[str], message: str) -> None:
    errors.append(message)


def validate_vector(path: Path, errors: list[str]) -> None:
    name = path.name
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        fail(errors, f"{name}: invalid JSON: {exc}")
        return

    if data.get("schema") != SCHEMA:
        fail(errors, f"{name}: schema must be {SCHEMA!r}, got {data.get('schema')!r}")
    seed = data.get("seed")
    if not isinstance(seed, int) or seed <= 0:
        fail(errors, f"{name}: seed must be a positive integer")
    if data.get("difficulty") not in DIFFICULTIES:
        fail(errors, f"{name}: difficulty must be one of {sorted(DIFFICULTIES)}")
    for key in ("title", "generated_by"):
        if not isinstance(data.get(key), str) or not data[key].strip():
            fail(errors, f"{name}: {key} must be a non-empty string")

    expected = data.get("expected")
    if not isinstance(expected, dict):
        fail(errors, f"{name}: expected block missing")
        return
    for field, kind in EXPECTED_FIELDS.items():
        if not isinstance(expected.get(field), kind):
            fail(errors, f"{name}: expected.{field} must be {kind.__name__}")
    final_tick = expected.get("final_tick", 0)
    state_hash = expected.get("state_hash", "")
    if not (isinstance(state_hash, str) and state_hash.startswith("0x") and len(state_hash) == 18):
        fail(errors, f"{name}: expected.state_hash must look like 0x<16 hex digits>")
    if expected.get("boundary") not in {"SEALED", "BREACHED", "HOLDING"}:
        fail(errors, f"{name}: expected.boundary must be SEALED|BREACHED|HOLDING")
    if expected.get("status") not in {"victory", "defeat", "running"}:
        fail(errors, f"{name}: expected.status must be victory|defeat|running")

    actions = data.get("actions")
    if not isinstance(actions, list):
        fail(errors, f"{name}: actions must be a list")
        return
    last_tick = 0
    for index, action in enumerate(actions):
        where = f"{name}: actions[{index}]"
        if not isinstance(action, dict):
            fail(errors, f"{where} must be an object")
            continue
        tick = action.get("tick")
        if not isinstance(tick, int) or tick < 1:
            fail(errors, f"{where}.tick must be an integer >= 1")
            continue
        if tick <= last_tick:
            fail(errors, f"{where}.tick must be strictly increasing")
        if isinstance(final_tick, int) and tick > final_tick:
            fail(errors, f"{where}.tick is past expected.final_tick")
        lane = action.get("lane")
        if not isinstance(lane, int) or not 0 <= lane <= 4:
            fail(errors, f"{where}.lane must be 0..4")
        if action.get("action") not in ACTIONS:
            fail(errors, f"{where}.action must be one of {sorted(ACTIONS)}")
        last_tick = tick


def validate_checksums(paths: list[Path], errors: list[str]) -> None:
    checks = VECTORS / "checksums.txt"
    if not checks.exists():
        fail(errors, "vectors/checksums.txt is missing")
        return
    listed: dict[str, str] = {}
    for line in checks.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            digest, name = line.split(None, 1)
        except ValueError:
            fail(errors, f"checksums.txt: malformed line {line!r}")
            continue
        listed[name.strip()] = digest.lower()
    for path in paths:
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        recorded = listed.pop(path.name, None)
        if recorded is None:
            fail(errors, f"checksums.txt: missing entry for {path.name}")
        elif recorded != actual:
            fail(errors, f"checksums.txt: digest mismatch for {path.name}")
    for orphan in listed:
        fail(errors, f"checksums.txt: lists {orphan}, but the file is absent")


def main() -> int:
    vectors = sorted(VECTORS.glob("*.json"))
    errors: list[str] = []
    if not vectors:
        errors.append("no replay vectors found in vectors/")
    for path in vectors:
        validate_vector(path, errors)
    validate_checksums(vectors, errors)

    if errors:
        print("Replay vector validation FAILED:")
        for error in errors:
            print(f"  - {error}")
        return 1
    print(f"Replay vectors OK ({len(vectors)} file(s), schema {SCHEMA}).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
=======
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
VECTOR = ROOT / "vectors" / "replay-v2.0.0.json"
data = json.loads(VECTOR.read_text(encoding="utf-8"))

assert data["schema"] == "neural-boundary-replay-v2.0.0"
assert data["seed"] == 1707
assert data["config"]["difficulty"] in (0, 1, 2)

allowed = {
    "idle",
    "move_up",
    "move_down",
    "validate",
    "convert",
    "quarantine",
    "consent_gate",
    "evidence_gate",
    "release",
    "restart",
}

last = 0
for item in data["actions"]:
    assert item["tick"] >= last
    assert item["action"] in allowed
    last = item["tick"]

print(f"validated {len(data['actions'])} replay actions for neural-boundary-replay-v2.0.0")
>>>>>>> origin/main
