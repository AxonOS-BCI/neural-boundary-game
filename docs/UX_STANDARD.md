# UX Standard

## Aesthetic

The product uses restrained digital luxury: dark mineral surfaces, precise spacing, high-contrast typography, fine rules, minimal chromatic noise, and deliberate motion. Decoration must reinforce hierarchy, not imitate a generic sci-fi dashboard.

## Field geometry

- Logical field remains `1200 × 640`.
- Canvas scales with preserved aspect ratio.
- Circular entities remain circular.
- The membrane position and lane spacing remain visually stable.
- No content may collapse into the upper-left corner during resize or orientation changes.

## Mobile

- Primary controls remain inside safe-area insets.
- Tap targets are at least 44 CSS pixels in their smallest dimension.
- The control deck does not require hover.
- Portrait and landscape both expose metrics, field, lane selection, and all six actions.
- Text remains readable without browser zoom.

## Motion and feedback

- Motion is functional and bounded.
- Reduced-motion preference removes nonessential transitions.
- Correct, blocked, incorrect, breach, and seal outcomes use distinct visual and textual feedback.
- Audio and haptics are optional, local, and muted by default.

## Accessibility

- Every action is keyboard reachable.
- Focus indicators are visible.
- Canvas state has a semantic DOM mirror.
- Important state changes are announced through a polite live region.
- Color is not the only signal for state.
- Dialogs have labels, descriptions, explicit return actions, and predictable cancel behavior.
- Opening protocol help pauses an active run; closing it resumes only the run that the dialog paused.
- Mode selection implements arrow, Home, and End navigation with a single roving tab stop.
- Repeated feedback after an intervening neutral state is announced again rather than being suppressed as a duplicate.

Manual review is required with representative Android and desktop browsers and at least one screen reader before publishing a formal conformance claim.
