# Release Checklist — v2.1.2

One command runs every gate:

```bash
bash scripts/smoke_check.sh
```

which is exactly:

```bash
cargo fmt --all --check
cargo test -p neural-boundary-core
cargo test -p neural-boundary-cli
cargo check -p neural-boundary-web
cargo build -p neural-boundary-web --target wasm32-unknown-unknown
python3 tools/validate_replay.py
python3 tools/check_hygiene.py
python3 tools/release_check.py
```

Then:

- regenerate `preview.png` if the stage changed
  (`python3 tools/generate_preview.py`);
- review `docs/LIMITATIONS.md` against the actual diff;
- push and confirm both workflows are green (CI, Pages);
- tag the release:

```bash
git tag -a v2.1.2 -m "Neural Boundary Game v2.1.2 — Foundation Grande AxonOS Standard Edition" && git push origin v2.1.2
```

- create the GitHub release from `RELEASE_NOTES.md` with the title
  `Neural Boundary Game v2.1.2 — Foundation Grande AxonOS Standard Edition`;
- update repository About, topics, website, and the social preview
  (`docs/GITHUB_SETUP.md`).
