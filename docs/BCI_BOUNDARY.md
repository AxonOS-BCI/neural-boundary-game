<!-- SPDX-FileCopyrightText: 2026 Denis Yermakou
SPDX-FileContributor: AxonOS
SPDX-License-Identifier: CC-BY-NC-ND-4.0 -->

# BCI Boundary Model

The Neural Boundary Game demonstrates the boundary enforcement concept
central to AxonOS: no raw neural signal leaves the neural processing
domain; only validated, typed, consent-gated output crosses to applications.

## The boundary contract

1. **Raw signal stays private.** RawFrame entities must be quarantined before
   reaching the membrane. Any crossing increments the raw_leaks counter
   and ultimately breaches the boundary.

2. **Intent requires validation.** Candidate intent must be validated (typed)
   before conversion. Conversion without validation is a protocol violation.

3. **Conversion requires consent and evidence.** The consent epoch model and
   evidence chain (L0→L1→L2→L3) ensure that output is authorized and auditable.

4. **Stimulation fails closed.** StimulationCommand entities trigger immediate
   boundary closure if they reach the membrane. There is no override.

5. **Seven gates must all pass before release.** Privacy, Typing, Consent,
   Evidence, Determinism, Vault and WCET gates are independent invariants.

## Not a medical device

This is an educational simulation. No real neural data is processed.
No stimulation hardware is connected or controlled.
