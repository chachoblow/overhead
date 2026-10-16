# 0033 — Wide-radar comparison contract

Date: 2026-10-15
Status: accepted (bounded M3 comparison; implemented as opt-in and visually approved)

## Decision
Implement the first real-data comparison of [0032](0032-wide-radar-and-selected-satellite-glyphs.md):
a 352×240 borderless radar, vertical FOV preserving the current pixel scale, and
one hollow outward arrow per eligible off-screen satellite. Keep the square
radar/diagnostic panel as the default baseline. This is not the complete normal
presentation or a finished glyph-strip layout.

## Why
Establish spatial coverage and real-data edge density before implementing the
glyph strip. The user explicitly prefers an arrow per satellite: allow overlaps
for this comparison, rather than suppressing arrows, merging groups, or moving
them away from their proper directions. Review real crowding before choosing
collision avoidance. After reviewing the implemented comparison, the user
approved the wide layout and overlaps-allowed arrows. Retain these for the next
slice; this does not change the executable's default or approve new collision
handling. Selected-marker identification was deferred to the next session.

Vertical FOV preserves the existing scene magnification while adding horizontal
space. It isolates the layout change from camera/zoom changes and does not
stretch Earth to fill the rectangle.

## Consequences
- Recenter the aim point within the 352×240 allocation. Keep the 75,000 km fixed
  camera, 20° default vertical FOV, 0.1°–100° vertical range, lens-only zoom and
  shared WGS-84 Earth/satellite perspective. Derive horizontal FOV from drawable
  half-extents, including the existing inset, not nominal allocation dimensions.
  Horizontal FOV can exceed 100° at the widest vertical setting.
- Only successful samples with true elevation > 0 and finite projection at
  forward depth >= 1 km can produce markers or arrows. Marker centers on the
  viewport boundary are in view; arrows are only for projections outside it.
  Behind-camera/near-plane clipping is not a tracking failure, and gets no arrow.
- Each arrow follows the center-to-projection ray to just inside the perimeter;
  inset enough to fit the complete hollow glyph. Its direction is toward the
  object's projected location, not its velocity, physical bearing, or distance.
  Do not stretch its pointing direction with the rectangle's aspect ratio.
- Emit one arrow per eligible off-screen object, even for coincident positions.
  No suppression, aggregation, displacement, counts on arrows, or collision
  avoidance. Overlapping arrows may not be separately distinguishable. Arrows
  become ordinary markers when the true projection enters the view.
- Keep the existing visible-only selection behavior: retain selection while
  visible, otherwise choose the first visible NORAD ID. Arrows are not selectable
  markers and do not inflate visible counts. No new ranking or persistence policy.
- Reserve the 48-pixel strip; do not implement glyphs or marker-adjacent IDs yet.
  Show minimal `TRACK ERROR` and `INGEST WARN` notices there, including alongside
  valid markers. `NO MARKERS` describes only the viewport, not a complete empty
  sky; `NO FRAME` identifies absent valid-time state. These are provisional
  comparison notices, not the final empty/incomplete-state design.
- Expose the comparison through an explicit simulator launch option, not a new
  device control or a runtime debug toggle. Retain full diagnostics in the
  baseline presentation and host reports. [Simulator usage](../../sim/README.md#wide-radar-comparison)
  owns commands, option incompatibilities, and observed snapshot counts.
- Test aspect/scale, shared Earth clipping, arrow directions and boundary entry,
  invalid depth, horizon eligibility, per-object multiplicity despite overlaps,
  selection/count independence, failure visibility and deterministic real-data
  snapshots against independent physical-look-angle references.
- Defer glyphs/mappings/transitions, labels/collisions, final normal/debug access
  and refined empty/incomplete-state treatment. No prediction integration,
  slowdown, panning, fetching, production limits/cadences, dependencies, firmware
  work or frozen-M2 evidence renewal. Simulator visuals do not establish device
  performance or physical-display legibility.
