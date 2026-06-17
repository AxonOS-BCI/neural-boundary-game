#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def load_manifest():
    with open(ROOT / "release.toml", "rb") as f:
        return tomllib.load(f)

def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8", errors="ignore")

def main() -> int:
    m = load_manifest()
    version = m["version"]
    display = m["display_version"]
    errors: list[str] = []

    checks = [
        ("VERSION", version),
        ("Cargo.toml", f'version = "{version}"'),
        ("README.md", display),
        ("README.md", m["homepage"]),
        ("RELEASE_NOTES.md", display),
        ("SECURITY.md", display),
        ("PRIVACY_NOTICE.md", display),
        ("release.toml", f'version = "{version}"'),
        ("web/index.html", display),
        ("web/manifest.json", display),
    ]

    for path, needle in checks:
        p = ROOT / path
        if not p.exists():
            errors.append(f"{path}: missing")
            continue
        if needle not in read(path):
            errors.append(f"{path}: missing {needle!r}")

    core = ROOT / "crates/neural-boundary-core/src/lib.rs"
    if core.exists():
        text = core.read_text(encoding="utf-8", errors="ignore")
        for needle in [
            f'CORE_VERSION: &str = "{version}"',
            f'REPLAY_SCHEMA: &str = "{m["replay_schema"]}"',
            f'HASH_ALGORITHM: &str = "{m["state_hash_algorithm"]}"',
        ]:
            if needle not in text:
                errors.append(f"{core.relative_to(ROOT)}: missing {needle!r}")

    vector_dir = ROOT / "vectors"
    vectors = sorted(vector_dir.glob("*.json"))
    if len(vectors) < 16:
        errors.append(f"vectors: expected at least 16 canonical vectors, found {len(vectors)}")

    for vector in vectors:
        data = json.loads(vector.read_text(encoding="utf-8"))
        expected = {
            "schema": m["replay_schema"],
            "product_version": version,
            "core_version": version,
            "hash_algorithm": m["state_hash_algorithm"],
        }
        for k, v in expected.items():
            if data.get(k) != v:
                errors.append(f"vectors/{vector.name}: {k} != {v!r}")

    stale_patterns = ["5.5.12", "v5.5.12", "3.0.1", "v3.0.1", "3.0.0", "v3.0.0"]
    allowed = {"CHANGELOG.md", "Cargo.lock"}
    scan_suffixes = {".md", ".toml", ".rs", ".html", ".css", ".js", ".sh", ".py", ".yml", ".yaml", ".json"}

    for path in ROOT.rglob("*"):
        rel = path.relative_to(ROOT).as_posix()
        if (
            not path.is_file()
            or path.suffix not in scan_suffixes
            or rel in allowed
            or rel.startswith((".git/", "target/", "dist/", "node_modules/"))
        ):
            continue
        text = path.read_text(encoding="utf-8", errors="ignore")
        for i, line in enumerate(text.splitlines(), start=1):
            for stale in stale_patterns:
                if stale in line:
                    errors.append(f"{rel}:{i}: stale version {stale}")

    if errors:
        print("Version consistency FAILED:")
        for e in errors:
            print("  -", e)
        return 1

    print(f"Version consistency OK: {display}")
    return 0

if __name__ == "__main__":
    sys.exit(main())
