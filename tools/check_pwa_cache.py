#!/usr/bin/env python3
# Copyright (c) 2026 Denis Yermakou / AxonOS
# SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
# Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.3.0).
"""Verify the service worker precache list (§14) is consistent with web/:
every precached path resolves to a real file, and every ES module / stylesheet
shipped in web/ is precached. The wasm is built by CI, so it may be absent."""
from __future__ import annotations
import re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "web"
SW = WEB / "service-worker.js"

def precache_entries(text: str) -> list[str]:
    m = re.search(r"PRECACHE\s*=\s*\[(.*?)\]", text, flags=re.S)
    if not m:
        return []
    return re.findall(r'"([^"]+)"', m.group(1))

def main() -> int:
    if not SW.exists():
        print("check_pwa_cache FAILED: web/service-worker.js missing")
        return 2
    text = SW.read_text(encoding="utf-8")
    entries = precache_entries(text)
    errors, notes = [], []
    if "CACHE_NAME" not in text or "v7.3.0" not in text:
        errors.append("service-worker.js: CACHE_NAME must be versioned (v7.3.0)")
    cached = set()
    for e in entries:
        rel = e.split("?", 1)[0]
        if rel in ("./", ""):
            rel = "index.html"
        cached.add(rel)
        target = WEB / rel
        if not target.exists():
            if rel.endswith(".wasm"):
                notes.append(f"{rel}: not present (built by CI) — OK")
            else:
                errors.append(f"precache entry has no file: web/{rel}")
    # Every shipped JS module / CSS must be precached for offline parity.
    # The service worker itself is never precached (the browser manages it).
    never_cached = {"service-worker.js"}
    for f in sorted(WEB.glob("*.js")) + sorted(WEB.glob("*.css")):
        if f.name in never_cached:
            continue
        if f.name not in cached:
            errors.append(f"web/{f.name} is shipped but not precached")
    for n in notes:
        print(f"  note: {n}")
    if errors:
        print("check_pwa_cache FAILED:")
        for e in errors:
            print(f"  - {e}")
        return 1
    print(f"PWA cache OK: {len(entries)} precache entries consistent with web/.")
    return 0

if __name__ == "__main__":
    sys.exit(main())
