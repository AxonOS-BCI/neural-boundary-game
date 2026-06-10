# Game Spec

Neural Boundary Game is a deterministic Rust/WASM game about BCI-adjacent software boundaries.

Core rule:

```text
Do not ship raw signal.
Ship typed intent.
```

The player defends the boundary between:

```text
Signal layer
Boundary layer
Application layer
```

A run succeeds when evidence gates are passed, risk is low, trust is high, and no raw leak reaches the application boundary.
