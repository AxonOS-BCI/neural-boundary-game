#!/usr/bin/env python3
# Copyright (c) 2026 Denis Yermakou / AxonOS
# SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
# Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.3.0).
"""Assert the built WASM exports >= 64 nbg_* functions (§17.2) and that the
mandatory health trio is present. Reads the .wasm export section directly."""
from __future__ import annotations
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FLOOR = 64
CANDIDATES = [
    ROOT / "target/wasm32-unknown-unknown/release/neural_boundary_web.wasm",
    ROOT / "web/neural_boundary_web.wasm",
]
REQUIRED = {"nbg_abi_version", "nbg_product_version_packed", "nbg_health_check"}

def leb(data: bytes, p: int) -> tuple[int, int]:
    r = s = 0
    while True:
        b = data[p]; p += 1
        r |= (b & 0x7F) << s; s += 7
        if not b & 0x80:
            return r, p

def exports(data: bytes) -> list[str]:
    assert data[:4] == b"\x00asm", "not a wasm module"
    pos = 8
    while pos < len(data):
        sid = data[pos]; pos += 1
        size, pos = leb(data, pos); end = pos + size
        if sid == 7:  # export section
            n, pos = leb(data, pos); out = []
            for _ in range(n):
                ln, pos = leb(data, pos)
                name = data[pos:pos + ln].decode(); pos += ln
                kind = data[pos]; pos += 1
                _, pos = leb(data, pos)
                if kind == 0 and name.startswith("nbg_"):
                    out.append(name)
            return out
        pos = end
    return []

def main() -> int:
    wasm = next((c for c in CANDIDATES if c.exists()), None)
    if wasm is None:
        print("check_abi_exports FAILED: no built wasm found. Build first:\n"
              "  cargo build -p neural-boundary-web --target wasm32-unknown-unknown --release")
        return 2
    names = exports(wasm.read_bytes())
    missing = REQUIRED - set(names)
    errors = []
    if len(names) < FLOOR:
        errors.append(f"only {len(names)} nbg_ exports (< floor {FLOOR})")
    if missing:
        errors.append(f"missing required exports: {sorted(missing)}")
    if errors:
        print("check_abi_exports FAILED:")
        for e in errors:
            print(f"  - {e}")
        return 1
    print(f"ABI exports OK: {len(names)} nbg_ functions (>= {FLOOR}) in {wasm.relative_to(ROOT)}.")
    return 0

if __name__ == "__main__":
    sys.exit(main())
