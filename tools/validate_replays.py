#!/usr/bin/env python3
"""Validate the exact replay schema, checksums, ordering, and reference outcomes."""
from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

sys.dont_write_bytecode = True

from reference_model import (
    CORE_VERSION,
    HASH_ALGORITHM,
    REPLAY_SCHEMA,
    TICK_RATE,
    Difficulty,
    RunMode,
    Simulation,
    SimulationConfig,
)

ROOT = Path(__file__).resolve().parents[1]
VECTOR_DIR = ROOT / "vectors"
errors: list[str] = []

mode_map = {"guided": RunMode.GUIDED, "standard": RunMode.STANDARD, "audit": RunMode.AUDIT, "grand": RunMode.GRAND, "daily": RunMode.DAILY}
difficulty_map = {"assisted": Difficulty.ASSISTED, "standard": Difficulty.STANDARD, "expert": Difficulty.EXPERT}
actions = {"validate", "convert", "quarantine", "consent", "evidence", "release"}
statuses = {"open", "sealed", "degraded", "breached", "unsafe"}
reasons = {"none", "released", "raw-leak-limit", "stimulation-crossed", "integrity-collapse", "risk-overflow", "time-expired", "invariant-violation"}
required_top = {"schema", "product_version", "core_version", "hash_algorithm", "seed", "mode", "difficulty", "tick_rate", "initial_config", "events", "expected", "summary"}
required_config = {"max_ticks", "raw_leak_limit"}
required_event = {"tick", "lane", "action"}
required_expected = {"terminal_tick", "status", "reason", "state_hash", "trust", "risk", "integrity", "evidence", "review_gates", "raw_leaks", "score"}
hex64 = re.compile(r"^[0-9a-f]{16}$")
sha256 = re.compile(r"^[0-9a-f]{64}$")


def exact_keys(name: str, value: object, required: set[str]) -> bool:
    if not isinstance(value, dict):
        errors.append(f"{name}: expected object")
        return False
    actual = set(value)
    if actual != required:
        errors.append(f"{name}: fields differ; missing={sorted(required-actual)} extra={sorted(actual-required)}")
        return False
    return True


def integer(name: str, value: object, minimum: int, maximum: int) -> bool:
    if type(value) is not int or not minimum <= value <= maximum:
        errors.append(f"{name}: expected integer in {minimum}..={maximum}, got {value!r}")
        return False
    return True

checksum_path = VECTOR_DIR / "checksums.sha256"
checksums: dict[str, str] = {}
if checksum_path.is_file():
    for line_number, line in enumerate(checksum_path.read_text(encoding="utf-8").splitlines(), start=1):
        if not line:
            continue
        match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9._-]+\.json)", line)
        if not match:
            errors.append(f"checksums.sha256:{line_number}: malformed line")
            continue
        digest, name = match.groups()
        if name in checksums:
            errors.append(f"checksums.sha256:{line_number}: duplicate {name}")
        checksums[name] = digest
else:
    errors.append("missing vectors/checksums.sha256")

paths = sorted(VECTOR_DIR.glob("*.json"))
if len(paths) != 8:
    errors.append(f"expected exactly 8 canonical replay vectors, found {len(paths)}")

for path in paths:
    if path.stat().st_size > 1_048_576:
        errors.append(f"{path.name}: exceeds 1 MiB")
        continue
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        errors.append(f"{path.name}: malformed JSON: {exc}")
        continue
    if not exact_keys(path.name, data, required_top):
        continue
    if data["schema"] != REPLAY_SCHEMA: errors.append(f"{path.name}: schema mismatch")
    if data["product_version"] != CORE_VERSION or data["core_version"] != CORE_VERSION: errors.append(f"{path.name}: version mismatch")
    if data["hash_algorithm"] != HASH_ALGORITHM: errors.append(f"{path.name}: hash algorithm mismatch")
    if data["tick_rate"] != TICK_RATE: errors.append(f"{path.name}: tick rate mismatch")
    if type(data["seed"]) is not int or not 0 <= data["seed"] <= 2**64 - 1: errors.append(f"{path.name}: seed must be u64")
    if data["mode"] not in mode_map or data["difficulty"] not in difficulty_map:
        errors.append(f"{path.name}: unknown mode/difficulty")
        continue
    if not isinstance(data["summary"], str) or not data["summary"].strip() or len(data["summary"].encode("utf-8")) > 4096: errors.append(f"{path.name}: summary must contain 1..=4096 UTF-8 bytes")
    if not exact_keys(f"{path.name}.initial_config", data["initial_config"], required_config): continue
    config = data["initial_config"]
    integer(f"{path.name}.max_ticks", config["max_ticks"], 1, 100_000)
    integer(f"{path.name}.raw_leak_limit", config["raw_leak_limit"], 1, 255)
    if not isinstance(data["events"], list) or len(data["events"]) > 10_000:
        errors.append(f"{path.name}: events must be a list of at most 10000 entries")
        continue
    previous = -1
    valid_events = True
    for index, event in enumerate(data["events"]):
        prefix = f"{path.name}.events[{index}]"
        if not exact_keys(prefix, event, required_event): valid_events = False; continue
        if not integer(f"{prefix}.tick", event["tick"], 0, config["max_ticks"]): valid_events = False
        if event["tick"] < previous: errors.append(f"{prefix}: tick is out of order"); valid_events = False
        previous = event["tick"]
        if not integer(f"{prefix}.lane", event["lane"], 0, 4): valid_events = False
        if event["action"] not in actions: errors.append(f"{prefix}: invalid action {event['action']!r}"); valid_events = False
    if not exact_keys(f"{path.name}.expected", data["expected"], required_expected): continue
    expected = data["expected"]
    terminal_tick_valid = integer(f"{path.name}.terminal_tick", expected["terminal_tick"], 0, config["max_ticks"])
    if terminal_tick_valid and data["events"] and data["events"][-1].get("tick", 0) > expected["terminal_tick"]:
        errors.append(f"{path.name}: event occurs after expected terminal_tick")
    for field in ("trust", "risk", "integrity"): integer(f"{path.name}.{field}", expected[field], 0, 100)
    integer(f"{path.name}.evidence", expected["evidence"], 0, 3)
    integer(f"{path.name}.review_gates", expected["review_gates"], 0, 31)
    integer(f"{path.name}.raw_leaks", expected["raw_leaks"], 0, 255)
    integer(f"{path.name}.score", expected["score"], 0, 2**32 - 1)
    if expected["status"] not in statuses: errors.append(f"{path.name}: invalid status")
    if expected["reason"] not in reasons: errors.append(f"{path.name}: invalid reason")
    if not isinstance(expected["state_hash"], str) or not hex64.fullmatch(expected["state_hash"]): errors.append(f"{path.name}: state_hash must be 16 lowercase hex characters")
    status_for_reason = {
        "none": "open",
        "released": "sealed",
        "raw-leak-limit": "breached",
        "stimulation-crossed": "unsafe",
        "integrity-collapse": "unsafe",
        "risk-overflow": "unsafe",
        "time-expired": "degraded",
        "invariant-violation": "unsafe",
    }
    if expected["reason"] in status_for_reason and expected["status"] != status_for_reason[expected["reason"]]:
        errors.append(f"{path.name}: status/reason mismatch")

    if valid_events:
        simulation = Simulation(SimulationConfig(int(data["seed"]), mode_map[data["mode"]], difficulty_map[data["difficulty"]], int(config["max_ticks"]), int(config["raw_leak_limit"])))
        for event in data["events"]: simulation.apply_event(event)
        expected_tick = int(expected["terminal_tick"])
        if simulation.tick_value < expected_tick: simulation.tick_many(expected_tick - simulation.tick_value)
        actual = simulation.expected()
        if actual != expected: errors.append(f"{path.name}: reference outcome mismatch\n    expected={expected}\n    actual={actual}")

    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if checksums.get(path.name) != digest: errors.append(f"{path.name}: SHA-256 checksum mismatch")

path_names = {path.name for path in paths}
if set(checksums) != path_names:
    errors.append(f"checksums coverage differs; missing={sorted(path_names-set(checksums))} extra={sorted(set(checksums)-path_names)}")

if errors:
    print("FAIL: replay validation")
    for item in errors: print(f"  - {item}")
    sys.exit(1)
print(f"PASS: exactly {len(paths)} replay vectors passed strict schema, outcome, and SHA-256 validation")
