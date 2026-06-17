#!/usr/bin/env python3
from pathlib import Path
import hashlib

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "SOURCE_MANIFEST.sha256"

skip = (".git/", "target/", "dist/", "node_modules/")
rows = []

for p in sorted(ROOT.rglob("*")):
    if not p.is_file():
        continue
    rel = p.relative_to(ROOT).as_posix()
    if rel == "SOURCE_MANIFEST.sha256" or rel.startswith(skip):
        continue
    rows.append(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {rel}")

OUT.write_text("\n".join(rows) + "\n", encoding="utf-8")
print(f"Wrote {OUT.relative_to(ROOT)} with {len(rows)} entries")
