<!-- SPDX-FileCopyrightText: 2026 Denis Yermakou
SPDX-FileContributor: AxonOS
SPDX-License-Identifier: CC-BY-NC-ND-4.0 -->

# UX Standard — Neural Boundary Game v7.3.0

## Design tokens

| Token | Value |
|-------|-------|
| --bg-0 | #030507 |
| --bg-1 | #080d12 |
| --sf1 | #0d141b |
| --ac (cyan) | #79def5 |
| --gold | #d6b96b |
| --safe | #78e6ad |
| --danger | #ff7186 |
| --purple | #a993ff |

## Accessibility

WCAG 2.2 AA. Skip link. Landmarks. Live regions (aria-live=polite) for
boundary events. No hover-only information. Escape closes modals (except
terminal result). Focus trapped in open modal. Reduced motion respected.

## Responsive breakpoints

- ≥760 px: 10-column HUD, 6-action deck
- <760 px: 5-column HUD, 3+3 deck
- Landscape <480 px tall: HUD hidden, compact layout

## Target sizes

- Primary CTA: 56 px min-height
- Action deck buttons: 60 px min-height (52 px mobile landscape)
- Lane selection: 44 px min-height

## State communication

State is never communicated by colour alone. Entity symbols (§9) are
visible at all scales. Gate status uses letter codes (P/T/C/E/D/V/W)
alongside colour.

## Honesty line

Every landing screen must display:
> Educational technical simulation. No sensor, no real neural data,
> no stimulation hardware. Local-only.
