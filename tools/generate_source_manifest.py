#!/usr/bin/env python3
"""Generate deterministic SHA-256 manifest for source-controlled release inputs."""
from __future__ import annotations

import hashlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "SOURCE_MANIFEST.sha256"
SKIP_DIRS = {".git", "target", "dist", "node_modules", "__pycache__", "release-assets"}
SKIP_FILES = {OUTPUT.name}


def source_files() -> list[Path]:
    return sorted(
        path
        for path in ROOT.rglob("*")
        if path.is_file()
        and path.name not in SKIP_FILES
        and not any(part in SKIP_DIRS for part in path.relative_to(ROOT).parts)
    )


lines = []
for path in source_files():
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    lines.append(f"{digest}  {path.relative_to(ROOT).as_posix()}")
OUTPUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
print(f"WROTE: {OUTPUT.relative_to(ROOT)} ({len(lines)} files)")
