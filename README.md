<!-- SPDX-FileCopyrightText: 2026 Denis Yermakou
SPDX-FileContributor: AxonOS
SPDX-License-Identifier: CC-BY-NC-ND-4.0 -->

# AxonOS Boundary Console v7.3.0 RC1

### Cognitive Sovereignty Runtime for neural boundary systems.

[![CI](https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/ci.yml/badge.svg)](https://github.com/AxonOS-BCI/neural-boundary-game/actions)
[![Release](https://img.shields.io/badge/release-v7.3.0--rc.1-black)](https://github.com/AxonOS-BCI/neural-boundary-game/releases)
[![License](https://img.shields.io/badge/license-AGPL--3.0--only%20OR%20AxonOS%20Commercial-blue)](LICENSE)

**Run the console:** https://axonos-bci.github.io/neural-boundary-game/

---

AxonOS Boundary Console is a deterministic Rust/WASM simulator for cognitive sovereignty and neural privacy boundaries.

It demonstrates the operating principle of AxonOS: raw neural signal remains private, applications receive typed intent only, and the boundary runtime remains deterministic, replayable, and evidence-driven.

This repository is a public release-candidate surface for the Boundary Console. The canonical AxonOS kernel line remains the hard real-time Rust microkernel work under AxonOS.

## Release posture

- **Version:** v7.3.0 RC1
- **Runtime:** Rust core + WASM boundary ABI
- **Verification:** replay vectors, source manifest, evidence report, Foundation CI gates
- **Status:** public release candidate, not a medical device, not a clinical system
- **Commercial posture:** AGPL-3.0-only OR AxonOS Commercial license

## Technical surface

- deterministic cognitive sovereignty engine
- flat WASM ABI v3 with 83 exports
- 16 canonical replay vectors
- replay/conformance CLI
- Grande Style Elite web console
- PWA/fail-closed fallback surface
- evidence report and source manifest
- 37 Foundation CI gates

---

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

45 kB Rust/WASM core. Flat WASM ABI v3 (83 exports), deterministic Rust core, no telemetry.
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

# Verify all 16 canonical replay vectors
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

*© 2026 Denis Yermakou / AxonOS. Neural Boundary Game™ v7.3.0 — Cognitive Sovereignty.*
*Software: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial.*
*AxonOS™ and Neural Boundary Game™ are claimed trademarks.*
