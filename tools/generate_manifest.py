#!/usr/bin/env python3
# Copyright (c) 2026 Denis Yermakou / AxonOS
# SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
# Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.3.0).
"""Write SOURCE_MANIFEST.sha256: a sorted SHA-256 manifest of the source tree
(§25/§26). Excludes VCS, build output, vendored deps, binaries, and generated
artifacts so the manifest is reproducible."""
from __future__ import annotations
import hashlib, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "SOURCE_MANIFEST.sha256"
SKIP_DIRS = {".git", "target", "dist", "node_modules"}
SKIP_NAMES = {"SOURCE_MANIFEST.sha256", "EVIDENCE_REPORT.md", ".DS_Store"}
SKIP_SUFFIX = {".wasm", ".png", ".jpg", ".jpeg", ".gif", ".pdf", ".mp4", ".bundle"}

def included(p: Path) -> bool:
    rel = p.relative_to(ROOT)
    if any(part in SKIP_DIRS for part in rel.parts):
        return False
    if p.name in SKIP_NAMES or p.suffix.lower() in SKIP_SUFFIX:
        return False
    return p.is_file()

def main() -> int:
    files = sorted(p for p in ROOT.rglob("*") if included(p))
    lines = []
    for p in files:
        digest = hashlib.sha256(p.read_bytes()).hexdigest()
        lines.append(f"{digest}  {p.relative_to(ROOT).as_posix()}")
    body = "\n".join(lines) + "\n"
    OUT.write_text(body, encoding="utf-8")
    aggregate = hashlib.sha256(body.encode()).hexdigest()
    print(f"SOURCE_MANIFEST.sha256: {len(files)} files, aggregate {aggregate[:16]}…")
    return 0

if __name__ == "__main__":
    sys.exit(main())
