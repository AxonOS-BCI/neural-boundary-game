#!/usr/bin/env python3
"""Structural validation of replay vectors (schema neural-boundary-replay-v3.0.1).

Deterministic re-execution is the CLI's job (`verify-all`); this gate checks
structure, enums, ordering, checksums, and — as a cross-language conformance
check — recomputes the Daily Seed in Python from the documented algorithm.
Expected identity values come from release.toml.
"""

from __future__ import annotations

import hashlib
import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VECTORS = ROOT / "vectors"

MODES = {"guided", "standard", "audit", "grand", "daily"}
DIFFICULTIES = {"calm", "standard", "intense"}
ACTIONS = {"Validate", "Convert", "Quarantine", "ConsentGate", "EvidenceGate", "Release"}
STATUSES = {"sealed", "breached", "expired"}
REASONS = {
    "sealed",
    "horizon_reached",
    "integrity_collapse",
    "risk_overflow",
    "raw_leak_threshold",
    "stimulation_crossed",
}
GRADES = {"Sovereign", "Sealed", "Reviewable", "Degraded", "Breached", "Unsafe"}
REQUIRED_VECTORS = [
    "01-clean-sealed.json",
    "02-idle-breach.json",
    "03-revoked-consent.json",
    "04-raw-leak.json",
    "05-stimulation-fail-closed.json",
    "06-audit-sealed.json",
    "07-grand-run-sealed.json",
    "08-daily-seed-sealed.json",
]

EXPECTED_FIELDS = {
    "terminal_tick": int,
    "status": str,
    "terminal_reason": str,
    "boundary": str,
    "grade": str,
    "trust": int,
    "risk": int,
    "integrity": int,
    "evidence_level": str,
    "evidence_points": int,
    "gates_passed": int,
    "raw_leaks": int,
    "delivered": int,
    "score": int,
    "best_streak": int,
    "revocations": int,
    "state_hash": str,
}


def manifest() -> dict:
    with open(ROOT / "release.toml", "rb") as handle:
        return tomllib.load(handle)


def fnv1a64(data: bytes) -> int:
    value = 0xCBF29CE484222325
    for byte in data:
        value = ((value ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return value


def splitmix(value: int) -> int:
    value &= 0xFFFFFFFFFFFFFFFF
    value ^= value >> 30
    value = (value * 0xBF58476D1CE4E5B9) & 0xFFFFFFFFFFFFFFFF
    value ^= value >> 27
    value = (value * 0x94D049BB133111EB) & 0xFFFFFFFFFFFFFFFF
    value ^= value >> 31
    return value or 0x3001


def daily_seed(schema: str, year: int, month: int, day: int) -> int:
    payload = schema.encode() + year.to_bytes(2, "little") + bytes([month, day])
    return splitmix(fnv1a64(payload))


def validate_vector(path: Path, identity: dict, errors: list[str]) -> None:
    name = path.name
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        errors.append(f"{name}: invalid JSON: {exc}")
        return

    for field, expected in (
        ("schema", identity["replay_schema"]),
        ("product_version", identity["version"]),
        ("core_version", identity["version"]),
        ("hash_algorithm", identity["hash_algorithm"]),
    ):
        if data.get(field) != expected:
            errors.append(f"{name}: {field} must be {expected!r}")

    if data.get("tick_rate") != 60:
        errors.append(f"{name}: tick_rate must be 60")
    if data.get("mode") not in MODES:
        errors.append(f"{name}: mode must be one of {sorted(MODES)}")
    if data.get("difficulty") not in DIFFICULTIES:
        errors.append(f"{name}: difficulty must be one of {sorted(DIFFICULTIES)}")
    seed = data.get("seed")
    if not isinstance(seed, int) or seed <= 0:
        errors.append(f"{name}: seed must be a positive integer")
    for key in ("title", "generated_by"):
        if not isinstance(data.get(key), str) or not data[key].strip():
            errors.append(f"{name}: {key} must be a non-empty string")

    if data.get("mode") == "daily":
        date = data.get("date")
        if not isinstance(date, str) or not re.fullmatch(r"\d{4}-\d{2}-\d{2}", date):
            errors.append(f"{name}: daily vector requires date YYYY-MM-DD")
        elif isinstance(seed, int):
            year, month, day = (int(part) for part in date.split("-"))
            expected_seed = daily_seed(identity["replay_schema"], year, month, day)
            if expected_seed != seed:
                errors.append(
                    f"{name}: seed {seed} != daily_seed({date}) = {expected_seed} (cross-language check)"
                )

    expected = data.get("expected")
    if not isinstance(expected, dict):
        errors.append(f"{name}: expected block missing")
        return
    for field, kind in EXPECTED_FIELDS.items():
        if not isinstance(expected.get(field), kind):
            errors.append(f"{name}: expected.{field} must be {kind.__name__}")
    if expected.get("status") not in STATUSES:
        errors.append(f"{name}: expected.status must be one of {sorted(STATUSES)}")
    if expected.get("terminal_reason") not in REASONS:
        errors.append(f"{name}: expected.terminal_reason invalid")
    if expected.get("grade") not in GRADES:
        errors.append(f"{name}: expected.grade invalid")
    if expected.get("boundary") not in {"SEALED", "BREACHED", "HOLDING"}:
        errors.append(f"{name}: expected.boundary invalid")
    state_hash = expected.get("state_hash", "")
    if not (isinstance(state_hash, str) and re.fullmatch(r"0x[0-9a-f]{16}", state_hash)):
        errors.append(f"{name}: expected.state_hash must be 0x + 16 lowercase hex digits")

    inputs = data.get("inputs")
    if not isinstance(inputs, list):
        errors.append(f"{name}: inputs must be a list")
        return
    last_tick = 0
    terminal_tick = expected.get("terminal_tick", 0)
    for index, item in enumerate(inputs):
        where = f"{name}: inputs[{index}]"
        if not isinstance(item, dict):
            errors.append(f"{where} must be an object")
            continue
        tick = item.get("tick")
        if not isinstance(tick, int) or tick < 1 or tick <= last_tick:
            errors.append(f"{where}.tick must be strictly increasing and >= 1")
        elif isinstance(terminal_tick, int) and tick > terminal_tick:
            errors.append(f"{where}.tick is past terminal_tick")
        lane = item.get("lane")
        if not isinstance(lane, int) or not 0 <= lane <= 4:
            errors.append(f"{where}.lane must be 0..4")
        if item.get("action") not in ACTIONS:
            errors.append(f"{where}.action invalid")
        if isinstance(tick, int):
            last_tick = tick


def validate_checksums(paths: list[Path], errors: list[str]) -> None:
    checks = VECTORS / "checksums.sha256"
    if not checks.exists():
        errors.append("vectors/checksums.sha256 is missing")
        return
    listed: dict[str, str] = {}
    for line in checks.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            digest, name = line.split(None, 1)
        except ValueError:
            errors.append(f"checksums.sha256: malformed line {line!r}")
            continue
        listed[name.strip()] = digest.lower()
    for path in paths:
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        recorded = listed.pop(path.name, None)
        if recorded is None:
            errors.append(f"checksums.sha256: missing entry for {path.name}")
        elif recorded != digest:
            errors.append(f"checksums.sha256: digest mismatch for {path.name}")
    for orphan in listed:
        errors.append(f"checksums.sha256: lists {orphan}, but the file is absent")


def main() -> int:
    identity = manifest()
    vectors = sorted(VECTORS.glob("*.json"))
    errors: list[str] = []
    names = {path.name for path in vectors}
    for required in REQUIRED_VECTORS:
        if required not in names:
            errors.append(f"required vector missing: {required}")
    for path in vectors:
        validate_vector(path, identity, errors)
    validate_checksums(vectors, errors)

    if errors:
        print("Replay vector validation FAILED:")
        for error in errors:
            print(f"  - {error}")
        return 1
    print(
        f"Replay vectors OK ({len(vectors)} file(s), schema {identity['replay_schema']}, "
        "daily seed cross-checked in Python)."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
