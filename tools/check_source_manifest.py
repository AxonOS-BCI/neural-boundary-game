#!/usr/bin/env python3
"""Verify that the deterministic source manifest exactly matches the source tree."""
from __future__ import annotations

import hashlib
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "SOURCE_MANIFEST.sha256"
SKIP_DIRS = {".git", "target", "dist", "node_modules", "__pycache__", "release-assets"}

if not MANIFEST.is_file():
    print("FAIL: SOURCE_MANIFEST.sha256 is missing")
    sys.exit(1)

expected: dict[str, str] = {}
errors: list[str] = []
for line_number, raw in enumerate(MANIFEST.read_text(encoding="utf-8").splitlines(), start=1):
    if not raw.strip():
        continue
    try:
        digest, relative = raw.split("  ", 1)
    except ValueError:
        errors.append(f"manifest line {line_number}: malformed")
        continue
    if len(digest) != 64 or any(char not in "0123456789abcdef" for char in digest):
        errors.append(f"manifest line {line_number}: invalid SHA-256")
    if relative in expected:
        errors.append(f"manifest line {line_number}: duplicate path {relative}")
    expected[relative] = digest

actual_paths = sorted(
    path
    for path in ROOT.rglob("*")
    if path.is_file()
    and path != MANIFEST
    and not any(part in SKIP_DIRS for part in path.relative_to(ROOT).parts)
)
actual_names = {path.relative_to(ROOT).as_posix() for path in actual_paths}
expected_names = set(expected)

for missing in sorted(actual_names - expected_names):
    errors.append(f"unmanifested source file: {missing}")
for stale in sorted(expected_names - actual_names):
    errors.append(f"manifest references missing file: {stale}")
for path in actual_paths:
    relative = path.relative_to(ROOT).as_posix()
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if expected.get(relative) != digest:
        errors.append(f"digest mismatch: {relative}")

if errors:
    print("FAIL: source manifest")
    for error in errors:
        print(f"  - {error}")
    sys.exit(1)
print(f"PASS: source manifest covers and verifies {len(actual_paths)} files")
