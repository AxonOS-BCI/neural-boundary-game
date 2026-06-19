#!/usr/bin/env python3
# Copyright (c) 2026 Denis Yermakou / AxonOS
# SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
# Part of Neural Boundary Game — Cognitive Sovereignty Console (v8.2.1).
"""Validate the replay vector suite against schema neural-boundary-replay-v4
(§19.2), check SHA-256 integrity, and cross-check the daily seed (§19.3 #16).
Structural validation only — deterministic replay equality is enforced by the
Rust CLI `verify-all`."""
from __future__ import annotations
import hashlib, json, re, sys, tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VECTORS = ROOT / "vectors"
MASK = 0xFFFFFFFFFFFFFFFF

ACTIONS = {"NONE","AUDIT","AUTHORIZE","REVOKE","QUARANTINE","SEAL_VAULT","THROTTLE","RELEASE"}
GRADES  = {"PENDING","SEALED","REVIEWABLE","BREACHED","UNSAFE"}
REASONS = {"NONE","SUCCESS_RELEASE","BOUNDARY_COLLAPSE","RAW_LEAK_LIMIT",
           "UNSAFE_STIMULATION_ESCAPE","CONSENT_COLLAPSE","VAULT_FAILURE","TIMEOUT"}
REQUIRED = [
    "01-clean-boundary-sealed","02-clean-boundary-timeout-reviewable",
    "03-raw-leak-breached","04-raw-leak-sealed","05-consent-drift-sealed",
    "06-consent-collapse","07-unsafe-stimulation","08-stimulation-throttled-sealed",
    "09-latency-storm-reviewable","10-adversarial-probe-breached",
    "11-adversarial-probe-sealed","12-vault-siege-breached",
    "13-silent-escalation-breached","14-silent-escalation-reviewable",
    "15-grand-trial-sealed","16-daily-seed-crosscheck",
]

def manifest():
    with open(ROOT / "release.toml", "rb") as f:
        return tomllib.load(f)

def _fnv(b: bytes) -> int:
    h = 0xCBF29CE484222325
    for x in b:
        h = ((h ^ x) * 0x100000001B3) & MASK
    return h

def _xss(state: int) -> int:
    x = state & MASK
    x ^= x >> 12
    x ^= (x << 25) & MASK
    x ^= x >> 27
    return (x * 0x2545F4914F6CDD1D) & MASK

def daily_seed(y: int, m: int, d: int) -> int:
    s = f"NBG|8.2.1|{y:04d}-{m:02d}-{d:02d}|DAILY".encode()
    h = _fnv(s)
    seed = h or 0x7300
    return _xss(seed)

def validate_vector(path: Path, idn: dict, errors: list[str]) -> None:
    name = path.name
    try:
        d = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        errors.append(f"{name}: JSON error: {e}"); return
    for field, exp in (("schema", idn["replay_schema"]), ("version", idn["version"])):
        if d.get(field) != exp:
            errors.append(f"{name}: {field} must be {exp!r} (got {d.get(field)!r})")
    if d.get("tick_rate") != idn["tick_rate_hz"]:
        errors.append(f"{name}: tick_rate must be {idn['tick_rate_hz']}")
    sid = d.get("scenario_id")
    if not isinstance(sid, int) or not (1 <= sid <= 16):
        errors.append(f"{name}: scenario_id must be int 1..16")
    seed = d.get("seed", "")
    if not (isinstance(seed, str) and re.fullmatch(r"[0-9a-f]{16}", seed)):
        errors.append(f"{name}: seed must be 16 lowercase hex digits")
    exp = d.get("expected")
    if not isinstance(exp, dict):
        errors.append(f"{name}: expected block missing"); return
    if exp.get("grade") not in GRADES:
        errors.append(f"{name}: expected.grade invalid ({exp.get('grade')!r})")
    if exp.get("reason") not in REASONS:
        errors.append(f"{name}: expected.reason invalid ({exp.get('reason')!r})")
    if not isinstance(exp.get("score_min"), int):
        errors.append(f"{name}: expected.score_min must be int")
    h = exp.get("state_hash", "")
    if not (isinstance(h, str) and re.fullmatch(r"0x[0-9a-f]{16}", h)):
        errors.append(f"{name}: expected.state_hash must be 0x+16 hex")
    acts = d.get("actions")
    if not isinstance(acts, list):
        errors.append(f"{name}: actions must be a list"); return
    last = 0
    for i, a in enumerate(acts):
        if not isinstance(a, dict):
            errors.append(f"{name}: actions[{i}] not an object"); continue
        t = a.get("tick")
        if not isinstance(t, int) or t < 1 or t <= last:
            errors.append(f"{name}: actions[{i}].tick not strictly increasing")
        else:
            last = t
        if a.get("action") not in ACTIONS:
            errors.append(f"{name}: actions[{i}].action {a.get('action')!r} invalid")
    # Daily-seed cross-check for the daily vector (§19.3 #16).
    m = re.match(r"daily (\d{4})-(\d{2})-(\d{2})", d.get("title", ""))
    if m and isinstance(seed, str) and re.fullmatch(r"[0-9a-f]{16}", seed):
        es = daily_seed(int(m[1]), int(m[2]), int(m[3]))
        if es != int(seed, 16):
            errors.append(f"{name}: daily seed mismatch py={es:016x} file={seed}")

def validate_checksums(paths: list[Path], errors: list[str]) -> None:
    checks = VECTORS / "checksums.sha256"
    if not checks.exists():
        errors.append("vectors/checksums.sha256 missing"); return
    listed: dict[str, str] = {}
    for line in checks.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        parts = line.split(None, 1)
        if len(parts) != 2:
            errors.append(f"checksums.sha256: malformed {line!r}"); continue
        listed[parts[1].strip()] = parts[0].lower()
    for p in paths:
        digest = hashlib.sha256(p.read_bytes()).hexdigest()
        rec = listed.pop(p.name, None)
        if rec is None:
            errors.append(f"checksums.sha256: {p.name} not listed")
        elif rec != digest:
            errors.append(f"checksums.sha256: digest mismatch for {p.name}")
    for orphan in listed:
        errors.append(f"checksums.sha256: {orphan} listed but absent")

def main() -> int:
    idn = manifest()
    vectors = sorted(VECTORS.glob("*.json"))
    errors: list[str] = []
    names = {p.name for p in vectors}
    for req in REQUIRED:
        if f"{req}.json" not in names:
            errors.append(f"required vector missing: {req}.json")
    for p in vectors:
        validate_vector(p, idn, errors)
    validate_checksums(vectors, errors)
    if errors:
        print("Replay vector validation FAILED:")
        for e in errors:
            print(f"  - {e}")
        return 1
    print(f"Replay vectors OK ({len(vectors)} files, schema {idn['replay_schema']}, "
          f"daily-seed cross-check passed).")
    return 0

if __name__ == "__main__":
    sys.exit(main())
