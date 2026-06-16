#!/usr/bin/env python3
"""Release gate aggregator: runs every repository invariant and prints one
concise PASS/FAIL table suitable for CI logs. Expected identity values come
from release.toml; this script never maintains its own magic strings.
"""

from __future__ import annotations

import json
import struct
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

REQUIRED_FILES = [
    "VERSION",
    "release.toml",
    "Cargo.toml",
    "Cargo.lock",
    "Trunk.toml",
    "rust-toolchain.toml",
    "web/index.html",
    "web/styles.css",
    "preview.png",
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
    ".github/workflows/release.yml",
    "crates/neural-boundary-core/src/lib.rs",
    "crates/neural-boundary-cli/src/main.rs",
    "crates/neural-boundary-cli/src/bot.rs",
    "crates/neural-boundary-cli/tests/vectors.rs",
    "crates/neural-boundary-web/src/lib.rs",
    "web/app.js",
    "web/abi.js",
    "web/render.js",
    "web/hud.js",
    "web/storage.js",
    "web/a11y.js",
    "crates/neural-boundary-web/src/app.rs",
    "crates/neural-boundary-web/src/bridge.rs",
    "crates/neural-boundary-web/src/input.rs",
    "crates/neural-boundary-web/src/render.rs",
    "crates/neural-boundary-web/src/hud.rs",
    "crates/neural-boundary-web/src/storage.rs",
    "crates/neural-boundary-web/src/accessibility.rs",
    "vectors/checksums.sha256",
    "docs/GAME_SPEC.md",
    "docs/REPLAY_SPEC.md",
    "docs/ARCHITECTURE.md",
    "docs/BCI_BOUNDARY.md",
    "docs/NO_RAW_NEURAL_DATA.md",
    "docs/LIMITATIONS.md",
    "docs/CLAIM_HYGIENE.md",
    "docs/COMMERCIAL_SERVICES.md",
    "docs/GITHUB_SETUP.md",
    "docs/RELEASE_PROCESS.md",
    "docs/UX_STANDARD.md",
    "scripts/verify_release.sh",
    "scripts/create_release_tag.sh",
    "scripts/termux_push.sh",
    "scripts/termux_find_unpack_push.sh",
    "tools/validate_replay.py",
    "tools/check_hygiene.py",
    "tools/check_version_consistency.py",
    "tools/check_links.py",
    "tools/release_check.py",
    "tools/generate_preview.py",
    "qa/package.json",
    "qa/playwright.config.ts",
    "qa/tests/smoke.spec.ts",
]

SUBCHECKS = [
    ("vectors", [sys.executable, "tools/validate_replay.py"]),
    ("hygiene", [sys.executable, "tools/check_hygiene.py"]),
    ("versions", [sys.executable, "tools/check_version_consistency.py"]),
    ("links", [sys.executable, "tools/check_links.py"]),
]


def manifest() -> dict:
    with open(ROOT / "release.toml", "rb") as handle:
        return tomllib.load(handle)


def png_size(path: Path) -> tuple[int, int] | None:
    data = path.read_bytes()
    if len(data) < 24 or data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
        return None
    return struct.unpack(">II", data[16:24])


def main() -> int:
    identity = manifest()
    rows: list[tuple[str, bool, str]] = []

    missing = [name for name in REQUIRED_FILES if not (ROOT / name).exists()]
    rows.append(("layout", not missing, "all release files present" if not missing else f"missing: {', '.join(missing[:6])}{' …' if len(missing) > 6 else ''}"))

    trunk = (ROOT / "Trunk.toml").read_text(encoding="utf-8")
    rows.append(("trunk", 'target = "web/index.html"' in trunk, "single Pages build path via web/index.html"))

    gitignore = (ROOT / ".gitignore").read_text(encoding="utf-8")
    ignored = all(token in gitignore for token in ("target", "dist", "node_modules"))
    rows.append((".gitignore", ignored, "build output ignored (target, dist, node_modules)"))

    license_root = (ROOT / "LICENSE").read_text(encoding="utf-8")
    license_ok = "MIT" in license_root and "Apache" in license_root and identity["license"] == "MIT OR Apache-2.0"
    rows.append(("license", license_ok, "dual MIT OR Apache-2.0 surface is unambiguous"))

    preview = png_size(ROOT / "preview.png") if (ROOT / "preview.png").exists() else None
    rows.append(("preview", preview == (1280, 720), f"preview.png is 1280x720 PNG (got {preview})"))

    vector_names = sorted(path.name for path in (ROOT / "vectors").glob("*.json"))
    rows.append(("vector-set", len(vector_names) == 8, f"8 canonical vectors present ({len(vector_names)} found)"))

    csp = (ROOT / "web/index.html").read_text(encoding="utf-8")
    rows.append(("csp", "Content-Security-Policy" in csp and "object-src 'none'" in csp, "restrictive CSP meta present"))

    for name, command in SUBCHECKS:
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
        ok = result.returncode == 0
        detail = (result.stdout or result.stderr).strip().splitlines()
        rows.append((name, ok, detail[0] if ok else "; ".join(detail[:4])))
        if not ok:
            for line in detail:
                print(f"    {line}")

    width = max(len(name) for name, _, _ in rows)
    failed = False
    for name, ok, detail in rows:
        status = "PASS" if ok else "FAIL"
        if not ok:
            failed = True
        print(f"{status}  {name.ljust(width)}  {detail}")

    if failed:
        print(f"\nRelease check FAILED for v{identity['version']}.")
        return 1
    print(f"\nRelease check OK: {identity['release_title']}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
