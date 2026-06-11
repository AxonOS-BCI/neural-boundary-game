# BCI Boundary

Neural Boundary Game represents an architectural boundary concept. It does not implement a BCI transport, acquisition driver, neural decoder, classifier trained on biological data, implant protocol, or stimulation path.

In an actual cognitive system, a comparable boundary would need separate layers for acquisition, signal quality, artifact rejection, feature extraction, model inference, intent typing, consent/capability enforcement, provenance, revocation, audit, application policy, safety monitoring, and hardware isolation.

The game deliberately collapses those concerns into symbolic entities so the user can reason about one principle: **raw signal is not an application API**.

No game outcome should be interpreted as evidence of clinical efficacy, neural-decoding accuracy, regulatory compliance, hardware safety, or production readiness.
