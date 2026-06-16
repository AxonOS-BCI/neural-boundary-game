#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Denis Yermakou
# SPDX-FileContributor: AxonOS
# SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
"""Structural validation of v5.5.12 replay vectors + Python daily-seed cross-check."""
from __future__ import annotations
import hashlib, json, re, sys, tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VECTORS = ROOT / "vectors"
MODES = {"GUIDED","STANDARD","AUDIT","GRAND","DAILY","PRIVACY_VAULT","KERNEL_TRIAL"}
ACTIONS = {"VALIDATE","CONVERT","QUARANTINE","CONSENT","EVIDENCE","RELEASE"}
STATUSES = {"SEALED","BREACHED","UNSAFE","ABORTED","FATAL_RUNTIME","RUNNING"}
REASONS = {
    "SUCCESS_RELEASE","TIMEOUT_UNSEALED","RISK_OVERFLOW","INTEGRITY_COLLAPSE",
    "RAW_LEAK_LIMIT","UNSAFE_STIMULATION_ESCAPE","DEADLINE_BREACH",
    "DETERMINISM_MISMATCH","REPLAY_SCHEMA_ERROR","WASM_INIT_FAILURE","USER_ABORT",
}
GRADES = {"SOVEREIGN","SEALED","REVIEWABLE","DEGRADED","BREACHED","UNSAFE"}
EV_LEVELS = {"L0","L1","L2","L3"}
REQUIRED_VECTORS = [
    "01-standard-clean-sealed.json","02-standard-idle-risk-overflow.json",
    "03-standard-lapse-revocations-sealed.json","04-standard-idle-raw-leak-limit.json",
    "05-standard-idle-stimulation.json","06-audit-clean-sealed.json",
    "07-grand-clean-sealed.json","08-daily-2026-06-14-sealed.json",
]
EXPECTED_TYPES: dict[str, type] = {
    "terminal_tick":int,"status":str,"terminal_reason":str,"grade":str,
    "trust":int,"risk":int,"integrity":int,"evidence_level":str,"evidence_bits":int,
    "gate_mask":int,"gates_passed":int,"raw_leaks":int,"typed_intents":int,
    "quarantined":int,"wrong_actions":int,"score":int,"best_combo":int,
    "revocations":int,"state_hash":str,
}

def load_manifest() -> dict:
    with open(ROOT / "release.toml", "rb") as f:
        return tomllib.load(f)

def _fnv1a64(data: bytes) -> int:
    h, prime = 0xCBF29CE484222325, 0x100000001B3
    for b in data:
        h = ((h ^ b) * prime) & 0xFFFFFFFFFFFFFFFF
    return h

def _xorshift64star(x: int) -> int:
    x &= 0xFFFFFFFFFFFFFFFF
    x ^= x >> 12
    x ^= (x << 25) & 0xFFFFFFFFFFFFFFFF
    x ^= x >> 27
    return (x * 0x2545F4914F6CDD1D) & 0xFFFFFFFFFFFFFFFF

def daily_seed_py(year: int, month: int, day: int) -> int:
    s = f"NBG|5.5.12|{year:04d}-{month:02d}-{day:02d}|DAILY"
    h = _fnv1a64(s.encode())
    seed = h or 0x3001
    result = _xorshift64star(seed)
    return result or 0x3001

def validate_vector(path: Path, identity: dict, errors: list[str]) -> None:
    name = path.name
    try:
        data = json.loads(path.read_text())
    except json.JSONDecodeError as e:
        errors.append(f"{name}: invalid JSON: {e}")
        return
    for field, expected in (
        ("schema", identity["replay_schema"]),
        ("product_version", identity["version"]),
        ("core_version", identity["version"]),
        ("hash_algorithm", identity["state_hash_algorithm"]),
        ("rng_algorithm", identity["rng_algorithm"]),
    ):
        if data.get(field) != expected:
            errors.append(f"{name}: {field} must be {expected!r}")
    if data.get("abi_version") != 1:
        errors.append(f"{name}: abi_version must be 1")
    if data.get("tick_rate_hz") != 60:
        errors.append(f"{name}: tick_rate_hz must be 60")
    if data.get("mode") not in MODES:
        errors.append(f"{name}: mode {data.get('mode')!r} invalid")
    diff = data.get("difficulty")
    if not isinstance(diff, int) or diff not in (0, 1, 2):
        errors.append(f"{name}: difficulty must be int 0|1|2")
    seed_raw = data.get("seed", "")
    valid_seed = isinstance(seed_raw, str) and bool(re.fullmatch(r"[0-9a-f]{16}", seed_raw))
    if not valid_seed:
        errors.append(f"{name}: seed must be exactly 16 lowercase hex digits (no 0x)")
    elif int(seed_raw, 16) == 0:
        errors.append(f"{name}: seed must be non-zero")
    if data.get("mode") == "DAILY":
        date = data.get("date", "")
        if not (isinstance(date, str) and re.fullmatch(r"\d{4}-\d{2}-\d{2}", date)):
            errors.append(f"{name}: DAILY requires date field YYYY-MM-DD")
        elif valid_seed:
            y, m, d = (int(x) for x in date.split("-"))
            expected_seed = daily_seed_py(y, m, d)
            if expected_seed != int(seed_raw, 16):
                errors.append(
                    f"{name}: seed {seed_raw} != daily_seed_py({date}) = {expected_seed:016x}"
                )
    exp = data.get("expected")
    if not isinstance(exp, dict):
        errors.append(f"{name}: expected block missing")
        return
    for field, kind in EXPECTED_TYPES.items():
        v = exp.get(field)
        if not isinstance(v, kind):
            errors.append(f"{name}: expected.{field} must be {kind.__name__}")
    if exp.get("status") not in STATUSES:
        errors.append(f"{name}: expected.status {exp.get('status')!r} invalid")
    if exp.get("terminal_reason") not in REASONS:
        errors.append(f"{name}: expected.terminal_reason {exp.get('terminal_reason')!r} invalid")
    if exp.get("grade") not in GRADES:
        errors.append(f"{name}: expected.grade {exp.get('grade')!r} invalid")
    if exp.get("evidence_level") not in EV_LEVELS:
        errors.append(f"{name}: expected.evidence_level invalid")
    h = exp.get("state_hash", "")
    if not (isinstance(h, str) and re.fullmatch(r"0x[0-9a-f]{16}", h)):
        errors.append(f"{name}: expected.state_hash must be 0x + 16 lowercase hex digits")
    inputs = data.get("inputs")
    if not isinstance(inputs, list):
        errors.append(f"{name}: inputs must be a list")
        return
    last_tick = 0
    terminal_tick = exp.get("terminal_tick", 0) if isinstance(exp, dict) else 0
    for i, inp in enumerate(inputs):
        if not isinstance(inp, dict):
            errors.append(f"{name}: inputs[{i}] must be object")
            continue
        t = inp.get("tick")
        if not isinstance(t, int) or t < 1 or t <= last_tick:
            errors.append(f"{name}: inputs[{i}].tick not strictly increasing")
        elif isinstance(terminal_tick, int) and t > terminal_tick + 600:
            errors.append(f"{name}: inputs[{i}].tick {t} past terminal_tick+600")
        if not isinstance(inp.get("lane"), int) or not (0 <= inp.get("lane", 99) <= 4):
            errors.append(f"{name}: inputs[{i}].lane must be 0..4")
        if inp.get("action") not in ACTIONS:
            errors.append(f"{name}: inputs[{i}].action {inp.get('action')!r} invalid")
        if isinstance(t, int):
            last_tick = t

def validate_checksums(paths: list[Path], errors: list[str]) -> None:
    checks = VECTORS / "checksums.sha256"
    if not checks.exists():
        errors.append("vectors/checksums.sha256 missing")
        return
    listed: dict[str, str] = {}
    for line in checks.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        parts = line.split(None, 1)
        if len(parts) != 2:
            errors.append(f"checksums.sha256: malformed line")
            continue
        listed[parts[1].strip()] = parts[0].lower()
    for path in paths:
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        rec = listed.pop(path.name, None)
        if rec is None:
            errors.append(f"checksums.sha256: {path.name} not listed")
        elif rec != digest:
            errors.append(f"checksums.sha256: digest mismatch for {path.name}")
    for orphan in listed:
        errors.append(f"checksums.sha256: {orphan} listed but file absent")

def main() -> int:
    identity = load_manifest()
    vectors = sorted(VECTORS.glob("*.json"))
    errors: list[str] = []
    names = {p.name for p in vectors}
    for req in REQUIRED_VECTORS:
        if req not in names:
            errors.append(f"required vector missing: {req}")
    for path in vectors:
        validate_vector(path, identity, errors)
    validate_checksums(vectors, errors)
    if errors:
        print("Replay vector validation FAILED:")
        for e in errors:
            print(f"  - {e}")
        return 1
    print(
        f"Replay vectors OK ({len(vectors)} files, schema {identity['replay_schema']}, "
        "daily seed Python cross-check passed)."
    )
    return 0

if __name__ == "__main__":
    sys.exit(main())
