# Replay Protocol

## Identity

- Schema: `neural-boundary-replay-v3.0.0`
- Product/core version: `3.0.0`
- Hash algorithm: `fnv1a64-v1`
- Tick rate: `60`

## Purpose

A replay is a deterministic conformance artifact, not a screen recording. It contains initial conditions, an ordered input stream, and expected terminal state. Native and WASM implementations of the same core must resolve it identically.

## Required envelope

```json
{
  "schema": "neural-boundary-replay-v3.0.0",
  "product_version": "3.0.0",
  "core_version": "3.0.0",
  "hash_algorithm": "fnv1a64-v1",
  "tick_rate": 60,
  "name": "clean-sealed",
  "mode": "guided",
  "difficulty": "standard",
  "seed": 58,
  "initial_config": {
    "max_ticks": 3600,
    "raw_leak_limit": 3
  },
  "events": [
    {"tick": 100, "lane": 2, "action": "validate"}
  ],
  "expected": {
    "terminal_tick": 3250,
    "status": "sealed",
    "reason": "released",
    "state_hash": "39654c8608a5cc3f"
  }
}
```

Canonical vectors include additional expected metrics. Consumers must reject incompatible schema, core version, hash algorithm, tick rate, unknown envelope/config/event/expected fields, duplicate or missing required fields, files over 1 MiB, more than 10,000 events, configurations over 100,000 ticks, summaries over 4,096 UTF-8 bytes, unsorted events, out-of-range ticks, invalid lanes, unknown actions, malformed lowercase hashes, and mismatched expected state.

## Event ordering

Events are ordered by nondecreasing tick. Multiple events at the same tick retain JSON array order. The verifier advances the simulation to each event tick, applies the event, then continues to terminal or expected terminal tick.

## Hash scope

The canonical hash includes all state needed to detect behavioral divergence: configuration, tick, RNG state, metrics, consent, evidence, gates, terminal state, feedback, spawn counters, selection, and every fixed entity slot in stable order.

The hash is a deterministic integrity identifier, not a cryptographic commitment. Do not use FNV-1a for authentication, signatures, adversarial collision resistance, or secret protection.

## Compatibility

Patch releases may remain schema-compatible only when replay output and strict parser semantics are unchanged. Any intentional change to RNG consumption, spawn schedule, transition rules, terminal checks, state serialization, or hash scope requires explicit compatibility review and normally a new replay schema identifier.
