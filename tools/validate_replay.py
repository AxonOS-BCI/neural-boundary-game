#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VECTORS = ROOT / "vectors"

def load_manifest():
    with open(ROOT / "release.toml", "rb") as f:
        return tomllib.load(f)

def main() -> int:
    m = load_manifest()
    errors: list[str] = []
    vectors = sorted(p for p in VECTORS.glob("*.json"))

    if len(vectors) < 16:
        errors.append(f"expected at least 16 canonical vectors, found {len(vectors)}")

    for path in vectors:
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except Exception as e:
            errors.append(f"{path.name}: JSON parse failed: {e}")
            continue

        for field, expected in [
            ("schema", m["replay_schema"]),
            ("product_version", m["version"]),
            ("core_version", m["version"]),
            ("hash_algorithm", m["state_hash_algorithm"]),
            ("rng_algorithm", m["rng_algorithm"]),
            ("tick_rate_hz", m["tick_rate_hz"]),
            ("abi_version", m["abi_version"]),
        ]:
            if data.get(field) != expected:
                errors.append(f"{path.name}: {field} must be {expected!r}, got {data.get(field)!r}")

        seed = data.get("seed")
        if not isinstance(seed, str) or not re.fullmatch(r"[0-9a-f]{16}", seed):
            errors.append(f"{path.name}: seed must be 16 lowercase hex digits")

        exp = data.get("expected")
        if not isinstance(exp, dict):
            errors.append(f"{path.name}: missing expected object")
        else:
            if not isinstance(exp.get("state_hash"), str) or not re.fullmatch(r"0x[0-9a-f]{16}", exp.get("state_hash", "")):
                errors.append(f"{path.name}: expected.state_hash must be 0x + 16 lowercase hex digits")
            for k in ["terminal_tick", "score", "trust", "risk", "integrity"]:
                if k in exp and not isinstance(exp[k], int):
                    errors.append(f"{path.name}: expected.{k} must be int")

        inputs = data.get("inputs")
        if not isinstance(inputs, list):
            errors.append(f"{path.name}: inputs must be list")
        else:
            last = -1
            for i, item in enumerate(inputs):
                if not isinstance(item, dict):
                    errors.append(f"{path.name}: inputs[{i}] must be object")
                    continue
                tick = item.get("tick")
                if not isinstance(tick, int) or tick <= last:
                    errors.append(f"{path.name}: inputs[{i}].tick not strictly increasing")
                if isinstance(tick, int):
                    last = tick

    checks = VECTORS / "checksums.sha256"
    if checks.exists():
        for line in checks.read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            sha, name = line.split(None, 1)
            target = VECTORS / name.strip()
            if target.exists():
                got = hashlib.sha256(target.read_bytes()).hexdigest()
                if got != sha:
                    errors.append(f"vectors/{target.name}: checksum mismatch")

    if errors:
        print("Replay validation FAILED:")
        for e in errors:
            print("  -", e)
        return 1

    print(f"Replay validation OK: {len(vectors)} vectors")
    return 0

if __name__ == "__main__":
    sys.exit(main())
