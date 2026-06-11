# Threat Model — Public Educational Build

## Protected properties

The build aims to preserve four properties:

1. Raw/private packet classes never become an application-facing primitive.
2. Typed intent release requires explicit classification, evidence, and scoped consent.
3. Deterministic native and WASM execution agree for canonical replay inputs.
4. The public browser build operates locally without telemetry or external runtime dependencies.

## Trust boundaries

- **Private signal domain:** simulated raw frames, artifacts, claims, and intent candidates.
- **Rust core:** authoritative transition, policy, terminal, scoring, and hash logic.
- **WASM adapter:** numeric boundary with no independent policy decisions.
- **Browser presentation:** untrusted input/rendering shell; it may request actions but cannot authorize release.
- **Build/release infrastructure:** produces artifacts and is outside the game-state trust boundary.

## Considered failure modes

- Raw frame reaches the membrane.
- Stimulation command crosses the boundary.
- Consent is expired, revoked, or missing a required scope bit.
- Evidence level is insufficient.
- An unresolved entity remains outside the currently selected lane.
- Browser code attempts to load a stale or partial WASM ABI.
- Replay input contains unknown fields, pathological size, invalid ordering, or malformed expected state.
- Cached or subpath deployment serves a mismatched artifact.
- Release metadata, tag, archive, and source tree diverge.

## Explicit non-goals

This repository does not defend against a compromised browser, operating system, GitHub account, Actions runner, package registry, signing key, or hosting origin. FNV-1a is not cryptographic. The simulation provides no clinical, medical, biometric, safety-critical, or certified real-time guarantees.

## Residual risks and higher-assurance options

For production infrastructure, add full-SHA action pinning, signed provenance/attestations, SBOM generation, hermetic builders, independent reproducibility verification, protected branches/environments, hardware-backed signing, CSP response headers, and formal protocol/security review.
