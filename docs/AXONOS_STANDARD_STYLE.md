# AxonOS Standard — Foundation Grande Style

The visual system of the v2.1.2 stage. It must be calm, dark, precise,
premium, readable, grid-aligned, free of skewed text and decorative noise,
and understandable in ten seconds.

## Palette (canonical)

| Token | Hex | Use |
|---|---|---|
| `bg` | `#03070B` | stage background |
| `panel` | `#071018` | cards, rail cells, overlays |
| `panel2` | `#0B1520` | chips, zone strip, nested surfaces |
| `border` | `#143140` | every hairline |
| `cyan` | `#20D6FF` | primary accent, membrane, passed gates |
| `cyan-soft` | `#8EEBFF` | secondary accent, artifact labels |
| `green` | `#7CFF4F` | typed intent, integrity, victory |
| `red` | `#FF4D4D` | hazards, risk, failure |
| `violet` | `#9B7CFF` | unknown packets |
| `amber` | `#FF9F1C` | claims, cooldown, release |
| `text` | `#EDF5FF` | primary text |
| `muted` | `#8EA0B6` | secondary text |
| `faint` | `#617186` | labels, keys |

## Type and layout

- System fonts only: a sans stack for headings, a mono stack
  (`Consolas / SF Mono / Menlo`) for telemetry, chips and labels.
- Stage: fixed 1280×720, 24 px padding, 12 px vertical rhythm; scaled to fit
  the viewport as one unit (no reflow).
- Rows: header 72 · status rail 54 · playfield 372 + event strip 34 ·
  action bar 54 · footer 30. Right panel 300 wide.
- Letter-spacing carries hierarchy; weight stays in the 400–800 range.

## Rules

- no rotated labels;
- no crowded microcopy;
- no random glow — light only where state changes (consent dot, passed gates);
- no cheap neon, no fake complexity;
- every panel needs a reason; every line must support hierarchy;
- color is semantic before it is decorative: cyan = artifact, green = typed,
  red = hazard, amber = claim, violet = unknown — identical in canvas,
  legend, preview and docs.

The footer line is part of the standard:

```text
v2.1.2 • AxonOS Standard Foundation Grande Style • deterministic Rust core
```
