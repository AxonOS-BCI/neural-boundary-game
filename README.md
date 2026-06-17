<!-- SPDX-FileCopyrightText: 2026 Denis Yermakou
SPDX-FileContributor: AxonOS
SPDX-License-Identifier: CC-BY-NC-ND-4.0 -->

# Neural Boundary Game v5.5.12

[![CI](https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/ci.yml/badge.svg)](https://github.com/AxonOS-BCI/neural-boundary-game/actions)
[![License: AGPL-3.0-only OR AxonOS Commercial](https://img.shields.io/badge/license-AGPL--3.0--only%20OR%20AxonOS%20Commercial-blue)](LICENSE)

[![Play Neural Boundary Game v5.5.12](preview.png)](https://axonos-bci.github.io/neural-boundary-game/)

**[▶ RUN BOUNDARY](https://axonos-bci.github.io/neural-boundary-game/)**

---

A deterministic Rust/WASM game demonstrating the AxonOS neural boundary protocol.
Raw signal stays private. Applications receive typed intent only.
The Rust core is authoritative. The browser is never trusted.

## 30-second walkthrough

1. Select a lane (↑/↓ or click)
2. **3** QUARANTINE raw frames, artifacts, stim commands and claims immediately
3. **4** CONSENT — gate a token to open the conversion and release scope
4. **5** EVIDENCE — register TRACE → CHECKSUM → CI in strict order
5. **1** VALIDATE candidates and unknowns
6. **2** CONVERT validated intent (requires active consent + L1 evidence)
7. **⏎** RELEASE — all 7 gates must pass and release scope must be active

## What this demonstrates

- Raw-signal containment (PRIVACY gate): zero leaks required
- Consent epoch model (CONSENT gate): revocation is immediate
- Evidence chain L0→L1→L2→L3 (EVIDENCE gate): out-of-order proof is rejected
- Stimulation fail-closed: `StimulationCommand` crossing terminates immediately
- Privacy Vault FSM: raw vault records compromise the vault on escape
- WCET gate: logical timing budget of 618 units enforced per tick
- Determinism contract: same seed + same actions = identical hash, verified offline

## Architecture

45 kB Rust/WASM core. Flat C ABI (41 named exports, no wasm-bindgen).
JavaScript UI. No runtime dependencies. No CDN. No telemetry.

```
neural-boundary-core/  # #![no_std] deterministic simulation, xorshift64star-v1 RNG
neural-boundary-cli/   # replay verifier, 8 canonical vectors, SHA-256 in Rust
neural-boundary-web/   # flat WASM ABI → web/*.js UI
```

## Build and verify

```bash
# Run the full gate
bash scripts/verify_release.sh

# Verify all 8 canonical replay vectors
cargo run -p neural-boundary-cli --release -- verify-all

# Build web to dist/
bash scripts/build_web.sh

# From Android Termux
bash scripts/termux_push.sh
```

## Replay proof

Every run produces a deterministic 64-bit state hash.
Pin a world offline with:

```bash
cargo run -p neural-boundary-cli --release -- record \
  --mode STANDARD --seed 0000000000000001 --difficulty 1 --policy clean \
  --out my-run.json
cargo run -p neural-boundary-cli --release -- verify my-run.json
```

## Intellectual Property and Licensing

Neural Boundary Game is developed by AxonOS.

The original software is available under a dual-licensing model:

- AGPL-3.0-only for qualifying open-source use; or
- a separate AxonOS Commercial License for proprietary and commercial use.

AxonOS names, logos, product identity, visual assets and designated commercial
materials are not licensed under the AGPL and remain protected intellectual property.

See: `LICENSE` · `COMMERCIAL_LICENSE.md` · `IP_NOTICE.md` · `TRADEMARKS.md` · `THIRD_PARTY_NOTICES.md`

## Support Development

Dogecoin: `DMwHAhqVNWf7dyEznukxCufNS5rjuP5MTp`

Verify the complete address before sending.

Dogecoin contributions are voluntary and do not represent an investment,
equity interest, token allocation, security, governance right or promise of financial return.
AxonOS will never request your wallet seed phrase or private key.

## Commercial deployment

Build a private boundary demo or deterministic safety review: `connect@axonos.org`

---

*© 2026 Denis Yermakou / AxonOS. Neural Boundary Game™ v5.5.12 — Cognitive Sovereignty.*
*Software: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial.*
*AxonOS™ and Neural Boundary Game™ are claimed trademarks.*
