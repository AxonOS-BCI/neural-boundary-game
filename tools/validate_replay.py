#!/usr/bin/env python3
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
VECTOR = ROOT / "vectors" / "replay-v1.9.1.json"
data = json.loads(VECTOR.read_text(encoding="utf-8"))

assert data["schema"] == "neural-boundary-replay-v1.9.1"
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

print(f"validated {len(data['actions'])} replay actions for neural-boundary-replay-v1.9.1")
