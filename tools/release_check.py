#!/usr/bin/env python3
"""Structural, semantic, and test-surface release gate for v3.0.0."""
from __future__ import annotations

import json
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REQUIRED_FILES = (
    "VERSION", "release.toml", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml",
    "package.json", "README.md", "CHANGELOG.md", "RELEASE_NOTES.md", "SECURITY.md",
    "CONTRIBUTING.md", "LICENSE", "LICENSE-MIT", "LICENSE-APACHE", "index.html",
    "preview.png", "SOURCE_MANIFEST.sha256", ".gitattributes",
    ".github/dependabot.yml",
    "crates/neural-boundary-core/Cargo.toml", "crates/neural-boundary-core/src/lib.rs",
    "crates/neural-boundary-cli/Cargo.toml", "crates/neural-boundary-cli/src/main.rs",
    "crates/neural-boundary-web/Cargo.toml", "crates/neural-boundary-web/src/lib.rs",
    "web/app.js", "web/styles.css", "web/favicon.svg",
    "web/tests/app.test.mjs", "web/tests/wasm-smoke.mjs",
    "tools/check_versions.py", "tools/check_version_consistency.py",
    "tools/check_hygiene.py", "tools/check_links.py", "tools/validate_replays.py",
    "tools/validate_replay.py", "tools/reference_model.py", "tools/generate_vectors.py",
    "tools/generate_source_manifest.py", "tools/check_source_manifest.py",
    "tools/deep_audit.py", "tools/package_release.py",
    "docs/GAME_SPEC.md", "docs/REPLAY_SPEC.md", "docs/ARCHITECTURE.md",
    "docs/BCI_BOUNDARY.md", "docs/NO_RAW_NEURAL_DATA.md", "docs/LIMITATIONS.md",
    "docs/CLAIM_HYGIENE.md", "docs/COMMERCIAL_SERVICES.md", "docs/GITHUB_SETUP.md",
    "docs/RELEASE_PROCESS.md", "docs/UX_STANDARD.md",
    "docs/ABI_CONTRACT.md", "docs/THREAT_MODEL.md",
    "scripts/build_web.sh", "scripts/verify_release.sh", "scripts/http_smoke.sh",
    "scripts/serve_dist.sh", "scripts/smoke_check.sh", "scripts/termux_release.sh",
    ".github/workflows/ci.yml", ".github/workflows/pages.yml",
    ".github/workflows/release.yml", ".github/pull_request_template.md",
    ".github/CODEOWNERS",
)

errors = [f"missing {item}" for item in REQUIRED_FILES if not (ROOT / item).is_file()]

release_path = ROOT / "release.toml"
release = {}
if release_path.is_file():
    release = tomllib.loads(release_path.read_text(encoding="utf-8"))
    expected_release = {
        "product": "Neural Boundary Game",
        "version": "3.0.0",
        "display_version": "v3.0.0",
        "git_tag": "v3.0.0",
        "replay_schema": "neural-boundary-replay-v3.0.0",
        "storage_namespace": "axonos_nbg_v300_",
        "license": "MIT OR Apache-2.0",
    }
    for key, expected in expected_release.items():
        if release.get(key) != expected:
            errors.append(f"release.toml {key!r}: expected {expected!r}, got {release.get(key)!r}")

readme_path = ROOT / "README.md"
readme = readme_path.read_text(encoding="utf-8") if readme_path.is_file() else ""
for needle in (
    "v3.0.0", "RUN THE GAME", "neural-boundary-replay-v3.0.0",
    "Educational technical simulation", "Commercial deployment", "MIT OR Apache-2.0",
    "scripts/verify_release.sh",
):
    if needle not in readme:
        errors.append(f"README missing {needle!r}")

package_path = ROOT / "package.json"
if package_path.is_file():
    package = json.loads(package_path.read_text(encoding="utf-8"))
    if package.get("version") != "3.0.0":
        errors.append("package.json version must be 3.0.0")
    scripts = package.get("scripts", {})
    for script in ("check:js", "test:web", "verify"):
        if script not in scripts:
            errors.append(f"package.json missing script {script!r}")

vectors = sorted((ROOT / "vectors").glob("*.json"))
if len(vectors) != 8:
    errors.append(f"expected exactly 8 canonical replay vectors, found {len(vectors)}")
for vector in vectors:
    try:
        data = json.loads(vector.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        errors.append(f"{vector.relative_to(ROOT)}: invalid JSON: {exc}")
        continue
    if data.get("schema") != "neural-boundary-replay-v3.0.0":
        errors.append(f"{vector.relative_to(ROOT)}: wrong replay schema")
    if "expected" not in data:
        errors.append(f"{vector.relative_to(ROOT)}: missing expected outcome")

workflow_text = "\n".join(
    path.read_text(encoding="utf-8")
    for path in sorted((ROOT / ".github/workflows").glob("*.yml"))
)
# Compiler and protocol commands are intentionally centralized in one canonical gate.
workflow_text += "\n" + (ROOT / "scripts/verify_release.sh").read_text(encoding="utf-8")
for needle in (
    "cargo fmt", "cargo clippy", "cargo test", "wasm32-unknown-unknown",
    "validate_replay.py", "deep_audit.py", "check_source_manifest.py", "http_smoke.sh",
    "package_release.py", "git merge-base --is-ancestor",
):
    if needle not in workflow_text:
        errors.append(f"workflow surface missing {needle!r}")

if errors:
    print("FAIL: release structure")
    for item in errors:
        print(f"  - {item}")
    sys.exit(1)
print("PASS: v3.0.0 release surface, canonical vectors, tests, and workflows are complete")
