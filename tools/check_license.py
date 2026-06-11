#!/usr/bin/env python3
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
license_file = ROOT / "LICENSE"
mit_file = ROOT / "LICENSE-MIT"

for path in [license_file, mit_file]:
    if not path.exists():
        print(f"missing license file: {path.name}", file=sys.stderr)
        raise SystemExit(1)

license_text = license_file.read_text(encoding="utf-8")
mit_text = mit_file.read_text(encoding="utf-8")

if "MIT License" not in license_text:
    raise SystemExit("root LICENSE does not look like MIT")
if "MIT License" not in mit_text:
    raise SystemExit("LICENSE-MIT does not look like MIT")
if "Apache License" in license_text:
    raise SystemExit("root LICENSE should be MIT-only for clean GitHub detection")

print("license checks passed")
