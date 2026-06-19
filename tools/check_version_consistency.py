#!/usr/bin/env python3
# Copyright (c) 2026 Denis Yermakou / AxonOS
# SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
# Part of Neural Boundary Game — Cognitive Sovereignty Console (v8.0.1).
"""Version-consistency gate. Canonical code/config surfaces are checked
strictly against release.toml. A repo-wide stale-version scan (5.5.12 / 7.0.3)
errors on code surfaces; hits in the legacy doc set are reported as WARNINGS
(pending the P0 docs retarget) unless --strict is given. License stubs are
also flagged as warnings (replace before release)."""
from __future__ import annotations
import json, re, sys, tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
STALE = ("5.5.12", "7.0.3")
LEGACY = ("README.md", "SECURITY.md", "PRIVACY_NOTICE.md", "RELEASE_NOTES.md",
          "THIRD_PARTY_NOTICES.md", "CHANGELOG.md", "docs/", "qa/")
SCAN_SUFFIX = {".md", ".toml", ".rs", ".html", ".css", ".js", ".sh", ".py", ".json", ".yml", ".yaml"}
SKIP_DIRS = (".git/", "target/", "dist/", "node_modules/", "qa/node_modules/")
SKIP_NAMES = {"CHANGELOG.md", "Cargo.lock", "SOURCE_MANIFEST.sha256"}

def manifest():
    with open(ROOT / "release.toml", "rb") as f:
        return tomllib.load(f)

def main() -> int:
    strict = "--strict" in sys.argv
    m = manifest()
    v, disp = m["version"], m["display_version"]
    errors: list[str] = []
    warnings: list[str] = []

    def need(path: str, needle: str, label: str):
        fp = ROOT / path
        if not fp.exists():
            errors.append(f"{path}: missing (needed for {label})")
        elif needle not in fp.read_text(encoding="utf-8", errors="ignore"):
            errors.append(f"{path}: expected {label} {needle!r}")

    # Canonical surfaces (strict).
    need("VERSION", v, "version")
    need("Cargo.toml", f'version = "{v}"', "workspace version")
    need("crates/neural-boundary-core/src/lib.rs", f'CORE_VERSION: &str = "{v}"', "core version")
    need("crates/neural-boundary-core/src/lib.rs", f'REPLAY_SCHEMA: &str = "{m["replay_schema"]}"', "replay schema")
    need("crates/neural-boundary-core/src/lib.rs", f'ABI_VERSION: u32 = {m["abi_version"]}', "abi version")
    need("crates/neural-boundary-core/src/hash.rs", f'HASH_ALGORITHM: &str = "{m["state_hash_algorithm"]}"', "hash algorithm")
    need("crates/neural-boundary-core/src/hash.rs", f'RNG_ALGORITHM: &str = "{m["rng_algorithm"]}"', "rng algorithm")
    need("web/index.html", disp, "visible UI version")
    need("web/service-worker.js", disp, "service-worker cache version")
    need("web/wasm-loader.js", "0x080001", "wasm packed-version guard")
    need("web/wasm-loader.js", f'ABI_VERSION = {m["abi_version"]}', "wasm abi guard")

    # DOGE config (§21.4).
    doge = ROOT / "DOGE.json"
    if not doge.exists():
        errors.append("DOGE.json: missing (§21.4)")
    else:
        d = json.loads(doge.read_text())
        if d.get("commercial_doge_enabled") is not False:
            errors.append("DOGE.json: commercial_doge_enabled must be false")
        if not d.get("doge_address"):
            errors.append("DOGE.json: doge_address missing")

    # Vector identity (light; deep checks in validate_replay.py).
    for vec in sorted((ROOT / "vectors").glob("*.json")):
        data = json.loads(vec.read_text())
        if data.get("schema") != m["replay_schema"]:
            errors.append(f"vectors/{vec.name}: schema != {m['replay_schema']!r}")
        if data.get("version") != v:
            errors.append(f"vectors/{vec.name}: version != {v!r}")

    # License-stub guard (P0): full texts must replace the placeholders.
    for lic in ("LICENSES/AGPL-3.0-only.txt", "LICENSES/CC-BY-NC-ND-4.0.txt"):
        fp = ROOT / lic
        if fp.exists():
            t = fp.read_text(encoding="utf-8", errors="ignore")
            if "BEFORE-RELEASE" in t or "replace with" in t.lower() or len(t) < 1500:
                warnings.append(f"{lic}: appears to be a STUB — drop in the full license text")

    # Repo-wide stale-version scan. Migrated code surfaces must be clean
    # (fatal); the rest of the repo is mid-migration (warnings, pending P0-3).
    CANON_PREFIX = ("crates/", "web/", "vectors/")
    CANON_FILE = {"release.toml", "Cargo.toml", "VERSION", "DOGE.json"}
    SELF = "tools/check_version_consistency.py"  # contains the patterns as literals

    def is_canon(rel: str) -> bool:
        return rel in CANON_FILE or any(rel.startswith(p) for p in CANON_PREFIX)

    for p in ROOT.rglob("*"):
        rel = p.relative_to(ROOT).as_posix()
        if (not p.is_file() or p.suffix not in SCAN_SUFFIX or p.name in SKIP_NAMES
                or rel == SELF or any(rel.startswith(s) for s in SKIP_DIRS)):
            continue
        text = p.read_text(encoding="utf-8", errors="ignore")
        for ln, line in enumerate(text.splitlines(), 1):
            for old in STALE:
                if old in line:
                    msg = f"{rel}:{ln}: stale version {old}"
                    (errors if is_canon(rel) else warnings).append(msg)

    if strict:
        errors += warnings
        warnings = []

    for w in warnings:
        print(f"  warn: {w}")
    if errors:
        print("Version consistency FAILED:")
        for e in errors:
            print(f"  - {e}")
        return 1
    extra = f" ({len(warnings)} warning(s) — legacy docs/licenses pending)" if warnings else ""
    print(f"Version consistency OK: canonical surfaces identify {disp}{extra}.")
    return 0

if __name__ == "__main__":
    sys.exit(main())
