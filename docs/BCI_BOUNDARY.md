# BCI Boundary

The game separates the system into three zones:

```text
Signal layer      raw frames, noise, artifacts
Boundary layer    consent, confidence, classifier output, evidence checks
Application layer typed intent only
```

The boundary is a software design principle: applications should receive typed intent and consent state, not raw signal.
