#!/usr/bin/env python3
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
mit = ROOT / "LICENSE-MIT"
apache = ROOT / "LICENSE-APACHE"
copyright_file = ROOT / "COPYRIGHT"

for path in [mit, apache, copyright_file]:
    if not path.exists():
        print(f"missing license metadata file: {path.name}", file=sys.stderr)
        raise SystemExit(1)

if "MIT License" not in mit.read_text(encoding="utf-8"):
    raise SystemExit("LICENSE-MIT does not look like MIT")
if "Apache License" not in apache.read_text(encoding="utf-8"):
    raise SystemExit("LICENSE-APACHE does not look like Apache-2.0")
if "MIT OR Apache-2.0" not in copyright_file.read_text(encoding="utf-8"):
    raise SystemExit("COPYRIGHT missing SPDX expression")

print("license checks passed")
