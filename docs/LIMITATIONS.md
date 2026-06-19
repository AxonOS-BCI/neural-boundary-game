<!-- SPDX-FileCopyrightText: 2026 Denis Yermakou
SPDX-FileContributor: AxonOS
SPDX-License-Identifier: CC-BY-NC-ND-4.0 -->

# Limitations

## What this game does not demonstrate

- Real neural signal processing pipelines
- Hardware latency or timing accuracy
- Clinical or full BCI pipeline deployment
- Regulatory compliance (FDA, MDR, etc.)
- Formal verification of boundary properties

## Technical limitations

- WASM is 45 kB; the full deterministic core, not a subset.
- JavaScript and browser timing are not part of the trust model.
- The 60 Hz tick rate is nominal; browser frame timing is approximate.
- Local storage can be cleared by the browser at any time.
- No offline service worker; the game requires an initial network load.

## Browser support

Any browser with WebAssembly and ES module support.
BigInt is required for u64 score values.
