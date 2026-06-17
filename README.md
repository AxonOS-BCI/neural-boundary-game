<!-- SPDX-FileCopyrightText: 2026 Denis Yermakou
SPDX-FileContributor: AxonOS
SPDX-License-Identifier: CC-BY-NC-ND-4.0 -->

# Neural Boundary Game v7.3.0 — Cognitive Sovereignty Console

[![CI](https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/ci.yml/badge.svg)](https://github.com/AxonOS-BCI/neural-boundary-game/actions)
[![License: AGPL-3.0-only OR AxonOS Commercial](https://img.shields.io/badge/license-AGPL--3.0--only%20OR%20AxonOS%20Commercial-blue)](LICENSE)
[![Style: AxonOS Standard Foundation Grande Elite](https://img.shields.io/badge/style-AxonOS%20Grande%20Elite-C8A96A)](docs/UX_STANDARD.md)

[![Play Neural Boundary Game v7.3.0](preview.png)](https://axonos-bci.github.io/neural-boundary-game/)

**[▶ RUN BOUNDARY](https://axonos-bci.github.io/neural-boundary-game/)**

**Elite AxonOS Standard Foundation Grande Style.**
> Do not ship raw signal. Ship typed intent.

---

A deterministic Rust/WASM game about defending **cognitive sovereignty** at the
brain–computer boundary. Raw signal stays inside the device; applications
receive typed intent only. The Rust core is authoritative; the browser is never
trusted and fails closed.

## 30-second walkthrough

You defend the Boundary Field against a timed schedule of neural events. Each
tick you may take **one** action (20 Hz core clock):

| Key | Action | Use it to |
|---|---|---|
| **D** | Audit | raise audit confidence; required before authorising escalations |
| **A** | Authorize | accept a safe, consented request |
| **R** | Revoke | cut a stale or unjustified permission immediately |
| **Q** | Quarantine | contain an exposure, probe, or latency spike |
| **S** | Seal Vault | seal the Privacy Vault before raw signal can leak |
| **T** | Throttle | clamp stimulation before it turns unsafe |
| **Enter** | Release | release sovereignty once every gate is satisfied |
| **Space** | Pause | freeze the field (presentation only) |

Mobile: tap the bottom action bar. Reduced-motion and mute toggles are honoured.

## What this demonstrates

- **Raw-signal containment** — `RAW_SIGNAL_EXPOSURE` must be vaulted or quarantined before Raw Leak Risk saturates.
- **Privacy Vault** — seal before authorising raw access; the vault fails the run if it collapses while raw is exposed.
- **Consent coherence** — sensitive permissions must be revoked promptly or consent collapses.
- **Stimulation fail-closed** — unsafe stimulation authorised blind ends the run; throttle first.
- **Deterministic replay** — same seed + same actions ⇒ identical 64-bit state hash, verified offline.

Eight live metrics (0–100): Boundary Integrity, Consent Coherence, Vault
Integrity, Cognitive Flow, Raw Leak Risk, Stimulation Risk, Latency Pressure,
Audit Confidence. Nine scenarios. Final grade: **Sealed · Reviewable ·
Breached · Unsafe**.

## Architecture

50 kB Rust/WASM core. Flat C ABI v3 (**83 named exports, no `wasm-bindgen`**).
Vanilla ES-module UI. No runtime dependencies. No CDN. No telemetry.

```
crates/
  neural-boundary-core   #![no_std] #![forbid(unsafe_code)] deterministic engine
  neural-boundary-cli    replay verifier + bot, 16 canonical vectors, SHA-256
  neural-boundary-web     flat WASM ABI v3 (cdylib) → web/*.js UI
web/                       13-file Grande Style Elite UI + Boundary Field + PWA
vectors/                   16 replay vectors + checksums (neural-boundary-replay-v3)
```

Identity: ABI v3 · product version packed `0x070300` · replay schema
`neural-boundary-replay-v3` · hash `fnv1a64-v1` · RNG `xorshift64star-v1` ·
20 Hz tick.

## Build and verify

```bash
rustup target add wasm32-unknown-unknown

# Full gate (fmt, clippy -D warnings, tests, wasm build, verify-all, Python gates)
bash scripts/verify_release.sh

# Verify all 16 canonical replay vectors
cargo run -p neural-boundary-cli --release -- verify-all

# Build the web bundle
bash scripts/build_web.sh
```

## Replay proof

Every run produces a deterministic 64-bit state hash. Pin a world offline:

```bash
cargo run -p neural-boundary-cli --release -- record \
  --scenario 1 --seed 0000000000000001 --policy clean --out my-run.json
cargo run -p neural-boundary-cli --release -- run-vector my-run.json
```

## Intellectual Property and Licensing

Developed by AxonOS under a dual-licensing model:

- **AGPL-3.0-only** for qualifying open-source use; or
- a separate **AxonOS Commercial License** for proprietary and commercial use.

Documentation is CC-BY-NC-ND-4.0. AxonOS names, logos, product identity and
designated commercial materials are not licensed under the AGPL and remain
protected intellectual property.

See: `LICENSE` · `COMMERCIAL_LICENSE.md` · `IP_NOTICE.md` · `TRADEMARKS.md` · `THIRD_PARTY_NOTICES.md`

## Support Development

Dogecoin: `DMwHAhqVNWf7dyEznukxCufNS5rjuP5MTp`

Verify the complete address before sending. Dogecoin contributions are voluntary
and do not represent an investment, equity interest, token allocation, security,
governance right or promise of financial return. AxonOS will never request your
wallet seed phrase or private key.

## Commercial deployment

Build a private boundary demo or deterministic safety review: `connect@axonos.org`

---

*© 2026 Denis Yermakou / AxonOS. Neural Boundary Game™ v7.3.0 — Cognitive Sovereignty Console.*
*Software: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial.*
*AxonOS™ and Neural Boundary Game™ are claimed trademarks.*

## Quick Launch

**Run the console:** https://axonos-bci.github.io/neural-boundary-game/

```bash
git clone https://github.com/AxonOS-BCI/neural-boundary-game.git
cd neural-boundary-game
bash scripts/build_web.sh
python3 -m http.server -d dist 8080
```

Then open `http://127.0.0.1:8080/`.

Verification:

```bash
bash scripts/verify_release.sh
cargo run -p neural-boundary-cli --release -- verify-all
```
