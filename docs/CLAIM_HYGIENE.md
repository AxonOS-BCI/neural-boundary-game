<!-- SPDX-FileCopyrightText: 2026 Denis Yermakou
SPDX-FileContributor: AxonOS
SPDX-License-Identifier: CC-BY-NC-ND-4.0 -->

# Claim Hygiene

All public statements about Neural Boundary Game and AxonOS must be
accurate and substantiated.

## Forbidden claims

- "Clinical-grade" or "clinically proven"
- "FDA approved" or "FDA ready"
- "Regulatory compliant" (without specifying which regulation and evidence)
- "Guaranteed safe" or "production BCI"
- "Reads thoughts" or "mind control"
- Any investment, equity or financial-return implication

## Permitted claims

- "Educational technical simulation"
- "Demonstrates boundary enforcement concepts"
- "Deterministic Rust/WASM core"
- "Open-source under AGPL-3.0-only"
- "Local-only, no telemetry"

The `tools/check_hygiene.py` gate enforces these rules in CI.
