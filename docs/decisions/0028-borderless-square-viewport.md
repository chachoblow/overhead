# 0028 — Borderless square viewport

Date: 2026-10-12
Status: accepted (future layout direction amended by [0032](0032-wide-radar-and-selected-satellite-glyphs.md); this remains the implemented viewport)

## Decision
Use a borderless square viewport by default, retaining the 240×240 working
allocation and provisional 20° full horizontal/vertical FOV. This supersedes
0026's circular viewport/clipping choice, not its physical or camera contracts.

## Why
After reviewing the 240/200-pixel layouts and circular, borderless-circular and
square snapshots, the user preferred 240×240 and explicitly chose borderless
square. The user likes the 100° overview but kept 20° as the default with room
to zoom in or out. This settles the current view shape, not the complete UI or
what belongs in the right-hand panel.

## Consequences
- Keep 0027's fixed 75,000 km camera and the same lens scale, center and inset.
  Admit the corners rather than rescaling the old circular view; the square's
  diagonal FOV is wider than its horizontal/vertical FOV. No outer border is drawn.
- Marker centers, complete glyphs and the WGS-84 Earth outline use square
  clipping. Visibility counts and selection share that viewport. Earth, the
  observer aim point and N label remain; the panel is unchanged.
- True elevation > 0 eligibility, the 1 km near plane, propagation, true-state
  readouts, explicit failures and lens-only zoom remain unchanged. No auto-fit,
  additional population, slowdown, prediction integration or fetching is added.
- Keep the circular modes as explicit simulator comparison options; see
  [usage](../../sim/README.md#viewport-shape-comparison). They are not new device controls.
- Preserve breathing room rather than adding objects to fill the view. Earlier
  approval of empty outer framing does not require retaining its drawn ring.
- Stay in M3 for panel exploration; no production density, cadence or capacity
  decision follows from these snapshots.
