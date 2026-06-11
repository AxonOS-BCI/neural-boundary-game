#!/usr/bin/env python3
"""Check local Markdown links without making network requests."""
from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[1]
pattern = re.compile(r"(?<!!)\[[^\]]+\]\(([^)]+)\)")
errors: list[str] = []

for path in ROOT.rglob("*.md"):
    if any(part in {"target", "dist", ".git"} for part in path.parts):
        continue
    text = path.read_text(encoding="utf-8")
    for target in pattern.findall(text):
        target = target.strip().split()[0].strip("<>")
        if target.startswith(("http://", "https://", "mailto:", "#")):
            continue
        clean = unquote(target.split("#", 1)[0])
        if not clean:
            continue
        resolved = (path.parent / clean).resolve()
        try:
            resolved.relative_to(ROOT.resolve())
        except ValueError:
            errors.append(f"{path.relative_to(ROOT)}: link escapes repository: {target}")
            continue
        if not resolved.exists():
            errors.append(f"{path.relative_to(ROOT)}: missing local link target: {target}")

if errors:
    print("FAIL: local links")
    for item in errors:
        print(f"  - {item}")
    sys.exit(1)
print("PASS: local Markdown links resolve")
