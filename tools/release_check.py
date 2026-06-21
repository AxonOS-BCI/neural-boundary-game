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
    "LICENSES/AGPL-3.0-only.txt",
    "LICENSES/CC-BY-NC-ND-4.0.txt",
    "README.md",
    "CHANGELOG.md",
    "RELEASE_NOTES.md",
    "CONTRIBUTING.md",
    "SECURITY.md",
    ".gitignore",
    ".github/CODEOWNERS",
    ".github/workflows/ci.yml",
    ".github/workflows/release.yml",
    "crates/neural-boundary-core/src/lib.rs",
    "crates/neural-boundary-cli/src/main.rs",
    "crates/neural-boundary-cli/src/bot.rs",
    "crates/neural-boundary-cli/tests/vectors.rs",
    "crates/neural-boundary-web/src/lib.rs",
    "web/main.js",
    "web/wasm-loader.js",
    "web/renderer.js",
    "web/hud.js",
    "web/accessibility.js",
    "web/scenarios.js",
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

    REQUIRED_MINUS_OLD = [n for n in REQUIRED_FILES
                          if n not in ("LICENSE-MIT","LICENSE-APACHE")]
    missing = [name for name in REQUIRED_MINUS_OLD if not (ROOT / name).exists()]
    rows.append(("layout", not missing, "all release files present" if not missing else f"missing: {', '.join(missing[:6])}{' …' if len(missing) > 6 else ''}"))

    shell_ok = (ROOT/"web/index.html").exists() and (ROOT/"web/neural_boundary_web.wasm").exists()
    rows.append(("web-build", shell_ok, "web/index.html + prebuilt WASM present"))

    gitignore = (ROOT / ".gitignore").read_text(encoding="utf-8")
    ignored = all(token in gitignore for token in ("target", "dist", "node_modules"))
    rows.append((".gitignore", ignored, "build output ignored (target, dist, node_modules)"))

    license_root = (ROOT / "LICENSE").read_text(encoding="utf-8") if (ROOT/"LICENSE").exists() else ""
    sw_lic = identity.get("software_license","")
    license_ok = "AGPL" in license_root and "Commercial" in license_root and "AGPL" in sw_lic
    rows.append(("license", license_ok, f"dual {sw_lic} licence surface in LICENSE"))

    preview = png_size(ROOT / "preview.png") if (ROOT / "preview.png").exists() else None
    rows.append(("preview", preview == (1280, 720), f"preview.png is 1280x720 PNG (got {preview})"))

    vector_names = sorted(path.name for path in (ROOT / "vectors").glob("*.json"))
    rows.append(("vector-set", len(vector_names) == 16, f"16 canonical vectors present ({len(vector_names)} found)"))

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
