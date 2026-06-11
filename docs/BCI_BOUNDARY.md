# BCI Boundary

<<<<<<< HEAD
The game separates the system into three zones:

```text
Signal layer       raw frames, noise, artifacts, unknown packets
Boundary layer     validation, consent, evidence, quarantine
Application layer  typed intent only
```

The boundary is a software design principle: applications should receive
typed intent and consent state, never raw signal. The playfield is that
principle drawn to scale — the membrane sits at `BOUNDARY_X = 680` of a
1000-unit field, and the 320-unit gate window in front of it is the only
place where review actions work.

Mapping to the mechanics in `docs/GAME_SPEC.md`:

| Zone | In the game |
|---|---|
| Signal layer | everything left of the membrane; `RAW`, `STIM`, `?PKT` belong here and must never leave |
| Boundary layer | the gate window: Validate, Consent, Evidence, Quarantine, and finally Release |
| Application layer | right of the membrane; only `TYPED` cards may exist here |

Every losing condition is a boundary statement: raw frames crossing
(`raw_leaks`), stimulation commands crossing (`direct_stim`), or unscoped
claims eroding integrity until nothing the interface says can be trusted.
=======
Applications should receive typed intent, consent state, confidence, and evidence metadata.

Applications should not receive raw signal frames.
>>>>>>> origin/main
