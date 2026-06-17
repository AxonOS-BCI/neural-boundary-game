#!/usr/bin/env python3
from pathlib import Path
import json
import sys

ROOT = Path(__file__).resolve().parents[1]
errors = []

manifest = ROOT / "web/manifest.json"
sw = ROOT / "web/service-worker.js"
index = ROOT / "web/index.html"

for p in [manifest, sw, index]:
    if not p.exists():
        errors.append(f"{p.relative_to(ROOT)} missing")

if manifest.exists():
    try:
        json.loads(manifest.read_text(encoding="utf-8"))
    except Exception as e:
        errors.append(f"web/manifest.json invalid JSON: {e}")

for p in [sw, index]:
    if p.exists():
        text = p.read_text(encoding="utf-8", errors="ignore")
        if "http://" in text or "https://" in text:
            errors.append(f"{p.relative_to(ROOT)} contains remote URL")

if errors:
    print("PWA cache gate FAILED:")
    for e in errors:
        print("  -", e)
    sys.exit(1)

print("PWA cache gate OK")
