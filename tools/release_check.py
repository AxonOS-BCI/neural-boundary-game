#!/usr/bin/env python3
import hashlib
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
VECTOR_DIR = ROOT / "vectors"
CHECKSUMS = VECTOR_DIR / "checksums.txt"

checks = {}
for line in CHECKSUMS.read_text(encoding="utf-8").splitlines():
    if line.strip():
        digest, name = line.split(None, 1)
        checks[name] = digest

for name, expected in checks.items():
    path = VECTOR_DIR / name
    if not path.exists():
        print(f"missing vector artifact: {name}", file=sys.stderr)
        raise SystemExit(1)
    actual = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual != expected:
        print(f"checksum mismatch for {name}", file=sys.stderr)
        raise SystemExit(1)

required = [
    "README.md",
    "CHANGELOG.md",
    "RELEASE_NOTES.md",
    "docs/GITHUB_SETUP.md",
    "docs/AXONOS_STANDARD_STYLE.md",
    "preview.png",
    ".github/workflows/ci.yml",
    ".github/workflows/pages.yml",
]
for name in required:
    if not (ROOT / name).exists():
        print(f"missing required release file: {name}", file=sys.stderr)
        raise SystemExit(1)

print(f"release checks passed for {len(checks)} vector artifact(s)")
