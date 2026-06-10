# CI Repair Notes for v1.9.7

The v1.5 CI failure was caused by Rust formatting only.

GitHub stable rustfmt rewrote:

```rust
matches!(snapshot.evidence_level, EvidenceLevel::L2 | EvidenceLevel::L3)
```

into:

```rust
matches!(
    snapshot.evidence_level,
    EvidenceLevel::L2 | EvidenceLevel::L3
)
```

v1.9.7 ships the rustfmt-compatible form directly.
