#!/usr/bin/env python3
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
required_docs = [
    "README.md",
    "CHANGELOG.md",
    "RELEASE_NOTES.md",
    "SECURITY.md",
    "CONTRIBUTING.md",
    "docs/GITHUB_SETUP.md",
    "docs/GAME_SPEC.md",
    "docs/AXONOS_STANDARD_STYLE.md",
    "docs/BCI_BOUNDARY.md",
    "docs/NO_RAW_NEURAL_DATA.md",
    "docs/CLAIM_HYGIENE.md",
    "docs/COMMERCIAL_SERVICES.md",
    "docs/ROADMAP.md",
    "docs/REPLAY_SPEC.md",
    "docs/RELEASE_CHECKLIST.md",
    "docs/LIMITATIONS.md",
    "docs/API_SURFACE.md",
    "assets/quick-launch-banner.svg",
    "docs/.nojekyll",
    "docs/index.html",
]

missing = [name for name in required_docs if not (ROOT / name).exists()]
if missing:
    for name in missing:
        print(f"missing doc: {name}", file=sys.stderr)
    raise SystemExit(1)

readme = (ROOT / "README.md").read_text(encoding="utf-8")
for needle in ["Do not ship raw signal", "Architecture", "Controls", "Limitations", "Commercial services"]:
    if needle not in readme:
        print(f"README missing section/content: {needle}", file=sys.stderr)
        raise SystemExit(1)

print("documentation checks passed")
