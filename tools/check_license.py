#!/usr/bin/env python3
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
mit = ROOT / "LICENSE-MIT"
apache = ROOT / "LICENSE-APACHE"
root_license = ROOT / "LICENSE"

for path in [mit, apache, root_license]:
    if not path.exists():
        print(f"missing license file: {path.name}", file=sys.stderr)
        raise SystemExit(1)

if "MIT License" not in mit.read_text(encoding="utf-8"):
    raise SystemExit("LICENSE-MIT does not look like MIT")
if "Apache License" not in apache.read_text(encoding="utf-8"):
    raise SystemExit("LICENSE-APACHE does not look like Apache-2.0")
if "MIT OR Apache-2.0" not in root_license.read_text(encoding="utf-8"):
    raise SystemExit("root LICENSE missing SPDX expression")

print("license checks passed")
