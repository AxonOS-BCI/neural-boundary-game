# Security Policy

## Supported version

Security fixes are accepted for the current `3.x` release line. Historical snapshots may remain available for reproducibility but are not maintained as active products.

## Reporting

Do not disclose a suspected vulnerability in a public issue before coordinated review. Send a concise report to `security@axonos.org` with:

- affected commit or tag;
- reproducible steps;
- expected and observed behavior;
- security impact;
- proof-of-concept material that does not contain third-party sensitive data.

Never send raw neural data, credentials, private keys, production tokens, or personal medical information.

## Security model

The open-source game is a local educational simulation. Its security controls focus on:

- deterministic state transitions;
- no JavaScript policy fallback;
- same-origin runtime assets;
- no analytics or telemetry;
- explicit consent and revocation state;
- bounded storage and fixed-capacity entity state;
- fail-closed terminal conditions;
- reproducible replay verification.

## Out of scope

The repository does not claim to secure an actual BCI, medical device, neural implant, stimulation controller, or production cognitive-data pipeline. Hardware integrations and customer deployments require a dedicated threat model, cryptographic key architecture, safety case, regulatory analysis, and independent testing.

## Dependencies and supply chain

The core has no third-party runtime dependency. The CLI uses `serde` and `serde_json`; the web adapter depends only on the core. CI must use locked dependencies and reviewed action versions. Release artifacts must be produced from a clean tagged commit.


## Supply-chain controls

- Browser runtime assets are same-origin and dependency-free.
- Rust dependencies are locked in `Cargo.lock`; CI uses `--locked`.
- Dependabot monitors Cargo and GitHub Actions metadata.
- Workflows use minimal job permissions and explicit timeouts.
- Source archives are normalized and reproducible under a fixed `SOURCE_DATE_EPOCH`.
- Release publication requires an annotated version tag whose commit is reachable from `origin/main`.

These controls reduce accidental drift; they do not by themselves prove action immutability or artifact provenance. A higher-assurance deployment should pin third-party actions by full commit SHA, publish signed attestations/SBOMs, and enforce protected environments.

## Browser policy boundary

The HTML meta CSP is defense in depth. Directives such as `frame-ancestors` must be delivered as HTTP response headers by the hosting layer. The public Pages build contains no telemetry, analytics, remote fonts, wallet integration, or external runtime fetch.
