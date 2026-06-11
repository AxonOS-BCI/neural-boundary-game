# Claim Hygiene

Scoped language is a release gate, not a style preference. CI runs
`tools/check_hygiene.py` over every tracked text file.

## Preferred wording

- BCI-adjacent software boundary;
- educational technical demo;
- deterministic simulation;
- typed intent model;
- reviewer-safe public artifact.

## Forbidden phrases

The checker fails the build on any of these, anywhere in the tree:

```text
clinical-grade        fda-ready           guaranteed safe
real brain control    mind control        regulatory compliant
certified medical     reads thoughts      production bci
medical device
```

Two escapes exist, both deliberate:

1. **Negation in the same sentence fragment** — “this is *not* a medical
   device” is exactly the kind of statement this repository should make.
2. The literal marker `claims-ok` on a line, reserved for documentation that
   must name the phrases themselves (this file is excluded by path instead).

If you need a third escape, the claim is probably wrong.
