# Neural Boundary Game v2.1.2 — Foundation Grande AxonOS Standard Edition

**Do not ship raw signal. Ship typed intent.**

v2.1.2 turns the skeleton into the flagship: a complete deterministic
review-conveyor game in `no_std` Rust, a strict replay verifier, pinned
conformance vectors, and the full Foundation Grande stage in the browser.

**Play:** <https://axonos-bci.github.io/neural-boundary-game/>

## Highlights

- **Complete boundary mechanic.** Five lanes, 14 entity kinds, six actions,
  five review gates, evidence levels, 25-second consent windows, and exact
  win/lose conditions. Unsupported claims move faster than evidence
  (`CLAIM_SPEED_BONUS`) — containment is a real-time discipline.
- **Deterministic to the hash.** The core is `#![no_std]`,
  `#![forbid(unsafe_code)]`, zero-allocation, fixed-step 60 Hz, with a 64-bit
  FNV-1a hash over the entire simulation state.
- **Seed 58, told twice.** The canonical clean run seals the boundary at tick
  1862 (trust 92, risk 12, integrity 88, 5/5 gates, 0 leaks). The same seed
  left idle breaches at tick 948 on the third raw leak. Same world — the only
  difference is boundary discipline.
- **CLI toolkit.** `verify`, `record` (clean/idle), `search`, `trace`;
  integration tests run both shipped vectors through the release binary.
- **Foundation Grande stage.** Fixed 1280×720 review console: status rail,
  canvas membrane with gate window, boundary-principle panel, action bar,
  menu/help/pause/end overlays — system fonts, canonical palette,
  scale-to-fit.
- **Release gates.** Replay/schema validation, negation-aware claim-hygiene
  scan, full-tree version consistency check; CI caches via
  `Swatinem/rust-cache@v2`, Pages builds with `jetli/trunk-action@v0.5.0`.

## Verify it yourself

```bash
cargo run -p neural-boundary-cli --release -- verify
```

```text
Replay OK
Final trust: 92
Final risk: 12
Final integrity: 88
Boundary status: SEALED
```

## Documentation

`docs/GAME_SPEC.md` · `docs/REPLAY_SPEC.md` · `docs/BCI_BOUNDARY.md` ·
`docs/AXONOS_STANDARD_STYLE.md` · `docs/LIMITATIONS.md` ·
`docs/CLAIM_HYGIENE.md` · `docs/COMMERCIAL_SERVICES.md`

This release is an educational technical demo of a software boundary
principle; it does not process real signal data and does not control
hardware.
