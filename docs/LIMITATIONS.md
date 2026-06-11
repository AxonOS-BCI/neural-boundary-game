# Limitations

This repository is an educational technical demo.

This repository is, explicitly:

- not a medical device;
- not a clinical system;
- not a safety-certified system;
- not a real neural decoder;
- not a stimulation controller;
- not a security audit;
- not a regulatory review;
- not a production firmware system.

It does not process real signal data.

It does not control stimulation hardware.

It demonstrates software boundary concepts through a deterministic game.

Scope notes:

- determinism is guaranteed for the simulation core and the replay vectors;
  wall-clock frame pacing in a browser is best-effort (fixed-step with a
  bounded catch-up of 6 steps per frame);
- the difficulty model is a teaching curve, not a model of any real
  signal-processing workload;
- the boundary rule shown here is one architectural pattern, presented as a
  game — adopting it in a real system requires the engineering work this
  demo deliberately does not claim to replace.
