#!/usr/bin/env python3
"""Fail when any active v3.0.0 release identity diverges from release.toml."""
from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
release = tomllib.loads((ROOT / "release.toml").read_text(encoding="utf-8"))
version = release["version"]
display = release["display_version"]
schema = release["replay_schema"]
namespace = release["storage_namespace"]
errors: list[str] = []


def require(condition: bool, message: str) -> None:
    if not condition:
        errors.append(message)


require((ROOT / "VERSION").read_text(encoding="utf-8").strip() == version, "VERSION does not match release.toml")

cargo = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
require(cargo["workspace"]["package"]["version"] == version, "workspace Cargo version mismatch")

package = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
require(package["version"] == version, "package.json version mismatch")

checks = {
    "README.md": [display, schema],
    "RELEASE_NOTES.md": [display, schema],
    "index.html": [display],
    "web/app.js": [f'VERSION = "{version}"', namespace],
    "crates/neural-boundary-core/src/lib.rs": [f'CORE_VERSION: &str = "{version}"', schema],
}
for relative, needles in checks.items():
    path = ROOT / relative
    require(path.exists(), f"missing {relative}")
    if not path.exists():
        continue
    text = path.read_text(encoding="utf-8")
    for needle in needles:
        require(needle in text, f"{relative} is missing canonical value {needle!r}")

for path in sorted((ROOT / "vectors").glob("*.json")):
    data = json.loads(path.read_text(encoding="utf-8"))
    require(data.get("schema") == schema, f"{path.name}: replay schema mismatch")
    require(data.get("product_version") == version, f"{path.name}: product version mismatch")
    require(data.get("core_version") == version, f"{path.name}: core version mismatch")

active_paths = [
    ROOT / "README.md",
    ROOT / "RELEASE_NOTES.md",
    ROOT / "index.html",
    ROOT / "web/app.js",
    ROOT / "release.toml",
    ROOT / "VERSION",
]
old_pattern = re.compile(r"\bv(?:1|2)\.\d+\.\d+\b")
for path in active_paths:
    if path.exists():
        match = old_pattern.search(path.read_text(encoding="utf-8"))
        require(match is None, f"{path.relative_to(ROOT)} contains stale active version {match.group(0) if match else ''}")

if errors:
    print("FAIL: version consistency")
    for item in errors:
        print(f"  - {item}")
    sys.exit(1)
print(f"PASS: all active release identities resolve to {display}")
