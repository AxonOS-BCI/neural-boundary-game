<!-- SPDX-FileCopyrightText: 2026 Denis Yermakou
SPDX-FileContributor: AxonOS
SPDX-License-Identifier: CC-BY-NC-ND-4.0 -->

# Release Notes — Neural Boundary Game v7.3.0

**Neural Boundary Game v7.3.0 — Production Grand AxonOS Standard Foundation**

**Production Grand AxonOS Standard Foundation — Cognitive Sovereignty**

## Summary

Neural Boundary Game v7.3.0 is the Production Grand release. The deterministic
Rust/WASM core is complete: 19 entity kinds, 7 review gates, Neural Permissions
epoch model, Privacy Vault FSM, WCET timing budget, and 7 run modes including
the Grand Run (four phases) and Kernel Trial (deadline pressure).

The WASM ABI is flat (41 named exports, no wasm-bindgen). The JavaScript UI
uses ES modules with a fixed-step RAF loop. Daily seed computation mirrors the
Rust core in JavaScript for offline verification.

The release includes 8 canonical replay vectors verified by SHA-256 and
deterministic re-execution. Every vector is covered by 18 integration tests.

## How to verify

```bash
cargo run -p neural-boundary-cli --release -- verify-all
python3 tools/validate_replay.py
```

## Known limitations

- Browser QA (Playwright) requires network access; BLOCKED in the Termux environment.
  QA specs are in `qa/` for CI execution.
- AGPL licence text files require the full GNU GPL text; stubs are present.
- Commercial DOGE payments disabled pending Singapore MAS legal review.

## Acceptance checklist (Denis Yermakou)

- [ ] Test DOGE transaction confirming wallet control (DMwHAhqVNWf7dyEznukxCufNS5rjuP5MTp)
- [ ] Replace AGPL/CC licence stubs with full texts from gnu.org / creativecommons.org
- [ ] Singapore legal review of DOGE commercial flow (MAS Payment Services Act)
- [ ] Tag signature: `git tag -s v7.3.0 -m "..."`
- [ ] Enable GitHub branch protection on main
- [ ] Upload preview.png as GitHub repository social preview
- [ ] Set Pages source to GitHub Actions (pages.yml)
- [ ] Publish Article #39 on Medium (AxonOS–SYM.BOT collaboration)
