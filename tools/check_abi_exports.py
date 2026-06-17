#!/usr/bin/env python3
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
candidates = [
    ROOT / "web/neural_boundary_web.wasm",
    ROOT / "dist/neural_boundary_web.wasm",
    ROOT / "target/wasm32-unknown-unknown/release/neural_boundary_web.wasm",
]
for p in candidates:
    if p.exists() and p.stat().st_size > 1000:
        print(f"ABI export gate OK: wasm present at {p.relative_to(ROOT)} ({p.stat().st_size} bytes)")
        sys.exit(0)
print("ABI export gate FAILED: neural_boundary_web.wasm not found")
sys.exit(1)
