# Contributing

Contributions must preserve deterministic behavior, claim discipline, privacy boundaries, and the single-authority architecture.

## Before opening a change

1. Read `docs/ARCHITECTURE.md`, `docs/GAME_SPEC.md`, and `docs/REPLAY_SPEC.md`.
2. Keep policy and simulation decisions inside `neural-boundary-core`.
3. Do not add a JavaScript fallback engine.
4. Do not add analytics, remote fonts, trackers, advertising, wallet code, or network-dependent game state.
5. Do not describe roadmap work as implemented capability.

## Required checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace --locked
cargo run --locked -p neural-boundary-cli -- verify-all
python3 tools/check_version_consistency.py
python3 tools/check_hygiene.py
python3 tools/check_links.py
python3 tools/validate_replay.py
python3 tools/release_check.py
node --test web/tests/*.test.mjs
```

Changes to state transition rules, RNG consumption, spawn schedules, terminal conditions, serialization, or hashing are protocol changes. Update canonical vectors and document compatibility impact.

## Commit discipline

Use focused commits. A typical sequence is:

```text
core: implement deterministic boundary invariant
web: render evidence gate state
test: add revoked-consent replay vector
docs: specify replay compatibility behavior
```

Do not use force-push on protected release branches. Do not tag from a working tree with uncommitted changes.
