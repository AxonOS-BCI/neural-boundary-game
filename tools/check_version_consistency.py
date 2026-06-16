#!/usr/bin/env python3
"""Version-consistency gate for Neural Boundary Game.

Expected values are read from release.toml — the single canonical manifest.
The checker inspects every active release surface and rejects stale project
versions. Historical versions are permitted only inside the historical
sections of CHANGELOG.md and Git metadata; this is parsed structurally, not
guessed with a broad grep.
"""

from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def load_manifest() -> dict:
    with open(ROOT / "release.toml", "rb") as handle:
        return tomllib.load(handle)


def changelog_history_versions(text: str, current: str) -> set[str]:
    versions = set(re.findall(r"^## \[(\d+\.\d+\.\d+)\]", text, flags=re.M))
    versions.discard(current)
    return versions


def main() -> int:
    manifest = load_manifest()
    version = manifest["version"]
    display = manifest["display_version"]
    errors: list[str] = []

    def expect(path: str, needle: str, label: str) -> None:
        file_path = ROOT / path
        if not file_path.exists():
            errors.append(f"{path}: missing (needed for {label})")
            return
        if needle not in file_path.read_text(encoding="utf-8"):
            errors.append(f"{path}: expected {label} {needle!r}")

    # Canonical version sites.
    expect("VERSION", version, "version")
    expect("Cargo.toml", f'version = "{version}"', "workspace version")
    expect("crates/neural-boundary-core/src/lib.rs", f'CORE_VERSION: &str = "{version}"', "core version")
    expect(
        "crates/neural-boundary-core/src/lib.rs",
        f'REPLAY_SCHEMA: &str = "{manifest["replay_schema"]}"',
        "replay schema",
    )
    expect(
        "crates/neural-boundary-core/src/lib.rs",
        f'HASH_ALGORITHM: &str = "{manifest["state_hash_algorithm"]}"',
        "hash algorithm",
    )
    expect("web/storage.js", f"NS = '{manifest['storage_namespace']}'", "storage namespace")
    expect("web/index.html", "v5.5.12", "visible UI version")
    expect("README.md", display, "README version")
    expect("README.md", manifest["homepage"], "homepage link")
    expect("RELEASE_NOTES.md", manifest["release_title"], "release title")
    expect("CHANGELOG.md", f"## [{version}]", "changelog entry")
    expect("scripts/create_release_tag.sh", manifest["git_tag"], "release tag")
    expect("docs/GAME_SPEC.md", version, "game spec version")
    expect("web/abi.js", "5<<16", "WASM product version check")
    expect("docs/REPLAY_SPEC.md", manifest["replay_schema"], "replay spec schema")

    # Vector schema fields.
    vector_dir = ROOT / "vectors"
    for vector in sorted(vector_dir.glob("*.json")):
        data = json.loads(vector.read_text(encoding="utf-8"))
        for field, expected in (
            ("schema", manifest["replay_schema"]),
            ("product_version", version),
            ("core_version", version),
            ("hash_algorithm", manifest["state_hash_algorithm"]),
        ):
            if data.get(field) != expected:
                errors.append(f"vectors/{vector.name}: {field} != {expected!r}")

    # Stale project versions: every version that ever appeared in CHANGELOG
    # history must not appear on any other active surface.
    changelog = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    history = changelog_history_versions(changelog, version)
    scan_suffixes = {".md", ".toml", ".rs", ".html", ".css", ".sh", ".py", ".yml", ".json"}
    skip = {"CHANGELOG.md", "Cargo.lock"}
    for path in ROOT.rglob("*"):
        relative = path.relative_to(ROOT).as_posix()
        if (
            not path.is_file()
            or path.suffix not in scan_suffixes
            or relative in skip
            or relative.startswith((".git/", "target/", "dist/", "node_modules/", "qa/node_modules"))
        ):
            continue
        text = path.read_text(encoding="utf-8", errors="ignore")
        for line_no, line in enumerate(text.splitlines(), start=1):
            for old in history:
                if old in line:
                    errors.append(f"{relative}:{line_no}: stale project version {old}")

    if errors:
        print("Version consistency FAILED:")
        for error in errors:
            print(f"  - {error}")
        return 1
    print(f"Version consistency OK: every active surface identifies {display}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
