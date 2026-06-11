# Game Specification — v3.0.0

## Objective

Protect the sovereign boundary. Keep private and unsafe entities in the signal domain. Transform eligible intent into an application-safe typed form. Seal only after a complete review.

## Field

The logical field is `1200 × 640` with five lanes. The membrane is positioned at logical X `840`. Entities spawn at X `16` and move toward the membrane on fixed simulation ticks.

Rendering must preserve aspect ratio. Circles remain circles at every viewport; unused space becomes letterbox padding rather than geometric distortion.

## Entity policy

| Entity | Required action | Result |
|---|---|---|
| Unknown packet | Validate | Resolves deterministically to validated intent or artifact |
| Validated intent | Convert | Becomes typed intent only with active consent and at least L1 evidence |
| Typed intent | None | May cross safely and contributes to trust/score |
| Raw frame | Quarantine | Establishes privacy gate; crossing counts as a leak |
| Artifact | Quarantine | Removes noise or false intent |
| Consent token | Consent | Activates an expiring capability with explicit conversion and release scope bits |
| Revoked consent | Consent | Revokes immediately |
| Evidence | Evidence | Raises evidence to L1 |
| Checksum | Evidence | Raises evidence to L2 and supports determinism gate |
| CI test | Evidence | Raises evidence to L3 and supports determinism gate |
| Unsupported/untraceable/roadmap-as-fact claim | Quarantine | Enforces claim hygiene |
| Stimulation command | Quarantine | Crossing terminates fail-closed |

## Actions

`Validate`, `Convert`, `Quarantine`, `Consent`, and `Evidence` operate on the foremost active entity in the selected lane. An incorrect action modifies risk/integrity and resets streak. Conversion requires the conversion scope bit. `Release` is global, requires the release scope bit, and checks all invariants.

## Metrics

- **Trust:** confidence earned through correct handling and safe typed-intent release.
- **Risk:** accumulated policy exposure; `100` is terminal.
- **Integrity:** health of boundary enforcement; `0` is terminal.
- **Evidence:** L0–L3 review maturity.
- **Score:** deterministic performance metric.
- **Streak:** consecutive correct actions.
- **Raw leaks:** private-frame crossings; mode/config limit is terminal.

Metrics are game abstractions, not medical or security certification scores.

## Review gates

1. Type safety — intent classification and conversion path demonstrated.
2. Consent — valid scoped capability active.
3. Evidence — proof maturity established.
4. Privacy — raw/private content quarantined without leak.
5. Determinism — checksum/CI evidence registered.

A seal attempt is additionally blocked while any entity slot remains active. Clearing only the visible lane is insufficient; the full fixed-capacity pool is authoritative.

## Terminal states

- `Sealed / Released`
- `Breached / RawLeakLimit`
- `Unsafe / StimulationCrossed`
- `Unsafe / IntegrityCollapse`
- `Unsafe / RiskOverflow`
- `Degraded / TimeExpired`
- `Unsafe / InvariantViolation`

## Mode profiles

Guided prioritizes comprehension. Standard is the canonical balance. Audit increases adversarial claims and evidence pressure. Grand extends the review interval and sequence density. Daily derives the seed from the UTC date so all clients receive the same deterministic challenge without a server.
