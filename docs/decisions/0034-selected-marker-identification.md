# 0034 — Selected-marker identification

Date: 2026-10-16
Status: accepted (bounded M3 contract; implemented in the opt-in wide view; zoom visibility and tuning amended by [0035](0035-label-zoom-visibility.md))

## Decision
Add one compact identifier beside the selected visible marker in
[0033's wide comparison](0033-wide-radar-comparison-contract.md), following
[0032's identification direction](0032-wide-radar-and-selected-satellite-glyphs.md).
The user approved right-side placement, left-side fallback, vertical clamping
and transparent overlaps before implementation. This amends 0033's deferral
of labels, not its approved radar/arrow behavior or the default presentation.

## Why
Identification makes the selected diamond useful without introducing new
physical data or designing the entire glyph strip. A deterministic placement
rule keeps this first comparison bounded. Review actual crowding before adding
collision handling; implementation is not a claim of user visual approval.

## Consequences
- Label only the selected visible marker: bare decimal NORAD ID, with the sole
  familiar-name exception `25544 -> ISS`. Names do not determine the exception.
  Support the full existing ID type rather than truncating at five/six digits.
  Keep full catalogue names and IDs in the unchanged diagnostic presentation.
- Use the existing 1-bit 6×10 font, vertically centered beside the diamond with
  a small gap. Prefer the right; flip left if the complete label would cross the
  drawable right edge. Clamp vertically to keep the complete text inside the
  drawable radar. No leader line or separate strip identifier.
- Draw transparent text over the scene. Overlaps with Earth, the aim point,
  north label, other markers or arrows are allowed. Do not erase background
  pixels, search for collision-free space, suppress labels or move scene glyphs.
  This may reduce legibility in crowded frames; that is a visual-review concern,
  not permission to silently change arrow policy.
- Follow existing visible-only selection immediately. No label for arrows or
  absent selection; clear stale text on every redraw. No new ranking,
  persistence, transition animation or selection control.
- Keep the square/diagnostic default, other comparison modes, projection,
  warning strip and all arrow geometry/multiplicity unchanged. No new CLI flag.
- Verify identity formatting, edge/corner fit, transparent compositing,
  selection/visibility lifecycle, baseline isolation and deterministic real-data
  CLI snapshots. Synthetic identity/placement cases are tests, not physical
  evidence or production catalogue choices.
- Defer category/data glyph mappings and transitions, final normal/debug access,
  refined empty/incomplete-state treatment, prediction, fetching, slowdown,
  capacity/cadence policy, firmware and frozen-M2 changes.
