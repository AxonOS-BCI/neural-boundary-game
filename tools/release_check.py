#!/usr/bin/env python3
"""Release gate for Neural Boundary Game v2.1.2.

Verifies that the working tree is internally consistent for the v2.1.2
Foundation Grande release:
  * version 2.1.2 is present in every place that states a version
  * no stray previous-release version strings outside CHANGELOG history
  * every file required by the release layout exists
  * preview.png is a 1280x720 PNG
  * the replay schema in shipped vectors matches the release
"""

from __future__ import annotations

import json
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION = "2.1.2"
PREVIOUS = "1.0.3"

REQUIRED_FILES = [
    "Cargo.toml",
    "Cargo.lock",
    "Trunk.toml",
    "index.html",
    "preview.png",
    "rust-toolchain.toml",
    "LICENSE",
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "README.md",
    "CHANGELOG.md",
    "RELEASE_NOTES.md",
    "CONTRIBUTING.md",
    "SECURITY.md",
    ".gitignore",
    ".github/CODEOWNERS",
    ".github/workflows/ci.yml",
    ".github/workflows/pages.yml",
    ".github/ISSUE_TEMPLATE/bug_report.md",
    ".github/ISSUE_TEMPLATE/feature_request.md",
    ".github/ISSUE_TEMPLATE/docs.md",
    ".github/PULL_REQUEST_TEMPLATE.md",
    "crates/neural-boundary-core/Cargo.toml",
    "crates/neural-boundary-core/src/lib.rs",
    "crates/neural-boundary-cli/Cargo.toml",
    "crates/neural-boundary-cli/src/main.rs",
    "crates/neural-boundary-cli/src/bot.rs",
    "crates/neural-boundary-cli/tests/vectors.rs",
    "crates/neural-boundary-web/Cargo.toml",
    "crates/neural-boundary-web/src/lib.rs",
    "vectors/replay-v2.1.2.json",
    "vectors/replay-breach-demo-v2.1.2.json",
    "vectors/checksums.txt",
    "docs/GAME_SPEC.md",
    "docs/REPLAY_SPEC.md",
    "docs/AXONOS_STANDARD_STYLE.md",
    "docs/BCI_BOUNDARY.md",
    "docs/NO_RAW_NEURAL_DATA.md",
    "docs/CLAIM_HYGIENE.md",
    "docs/COMMERCIAL_SERVICES.md",
    "docs/LIMITATIONS.md",
    "docs/ROADMAP.md",
    "docs/RELEASE_CHECKLIST.md",
    "docs/GITHUB_SETUP.md",
    "scripts/termux_push.sh",
    "scripts/termux_find_unpack_push.sh",
    "scripts/create_release_tag.sh",
    "scripts/smoke_check.sh",
    "tools/validate_replay.py",
    "tools/check_hygiene.py",
    "tools/release_check.py",
    "tools/generate_preview.py",
]

VERSION_SITES = [
    ("Cargo.toml", f'version = "{VERSION}"'),
    ("README.md", f"v{VERSION}"),
    ("CHANGELOG.md", f"## [{VERSION}]"),
    ("RELEASE_NOTES.md", f"v{VERSION}"),
    ("index.html", f"v{VERSION}"),
    ("scripts/create_release_tag.sh", f"v{VERSION}"),
    ("docs/GAME_SPEC.md", VERSION),
    ("docs/REPLAY_SPEC.md", f"neural-boundary-replay-v{VERSION}"),
]

# CHANGELOG legitimately keeps history of previous releases.
PREVIOUS_ALLOWED = {"CHANGELOG.md"}
PREVIOUS_SCAN = [
    "Cargo.toml",
    "README.md",
    "RELEASE_NOTES.md",
    "index.html",
    "Trunk.toml",
    "docs/GAME_SPEC.md",
    "docs/REPLAY_SPEC.md",
    "scripts/create_release_tag.sh",
    "scripts/termux_push.sh",
    "scripts/termux_find_unpack_push.sh",
    "scripts/smoke_check.sh",
]


def png_size(path: Path) -> tuple[int, int] | None:
    data = path.read_bytes()
    if len(data) < 24 or data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
        return None
    width, height = struct.unpack(">II", data[16:24])
    return width, height


def main() -> int:
    errors: list[str] = []

    for rel in REQUIRED_FILES:
        if not (ROOT / rel).exists():
            errors.append(f"missing required file: {rel}")

    for rel, needle in VERSION_SITES:
        path = ROOT / rel
        if path.exists() and needle not in path.read_text(encoding="utf-8"):
            errors.append(f"{rel}: expected to contain {needle!r}")

    for rel in PREVIOUS_SCAN:
        path = ROOT / rel
        if rel in PREVIOUS_ALLOWED or not path.exists():
            continue
        if PREVIOUS in path.read_text(encoding="utf-8"):
            errors.append(f"{rel}: stray previous version string {PREVIOUS}")

    preview = ROOT / "preview.png"
    if preview.exists():
        size = png_size(preview)
        if size is None:
            errors.append("preview.png: not a valid PNG")
        elif size != (1280, 720):
            errors.append(f"preview.png: expected 1280x720, got {size[0]}x{size[1]}")

    for rel in ("vectors/replay-v2.1.2.json", "vectors/replay-breach-demo-v2.1.2.json"):
        path = ROOT / rel
        if not path.exists():
            continue
        try:
            schema = json.loads(path.read_text(encoding="utf-8")).get("schema")
        except json.JSONDecodeError:
            schema = None
        if schema != f"neural-boundary-replay-v{VERSION}":
            errors.append(f"{rel}: schema does not match release version")

    if errors:
        print(f"Release check FAILED for v{VERSION}:")
        for error in errors:
            print(f"  - {error}")
        return 1
    print(f"Release check OK: v{VERSION} is consistent across the tree.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
