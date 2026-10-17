# Overhead — Handoff

_Last updated: 2026-10-17 UTC. Next: review warning placement for the revised glyph stack._

## Current state / agreed direction
Executable unchanged: [0037's elevation-only capsule](decisions/0037-static-elevation-glyph-contract.md)
and separate warnings in the opt-in wide view; square diagnostics remain default.
Keep [0033's radar/arrows](decisions/0033-wide-radar-comparison-contract.md),
[0034's ID placement](decisions/0034-selected-marker-identification.md) and
[0035's configurable 70°/60° hysteresis](decisions/0035-label-zoom-visibility.md).
[0038](decisions/0038-labelled-borderless-glyph-direction.md) accepts the revised
visual direction, not an implementation contract: labelled borderless stack,
padded separator, outlined elevation cutout, concentric altitude circles, speed
arc and period spiral; 1px navigation cross and simple station body/wings (B).
The user accepted equal 10px outer padding. Revised glyphs remain unimplemented.

## Session changes
Retained navigation/station 400×240 1-bit mocks in `docs/mockups/0038-labelled-glyph-*.png`;
[DESIGN](DESIGN.md#selected-satellite-presentation) owns their provenance and brief.
Added 0038; updated DESIGN/PLAN, earlier decision cross-references and PROGRESS.
No runtime, dependencies, fixtures, firmware or frozen-M2 evidence changed.

## Next step / boundaries
Recommendation, not approved scope: mock explicit notices in a reserved radar
corner, including tracking/ingestion warnings alongside valid markers. The new
stack occupies the current warning area; resolve placement before coding.
Glyph scales, speed/period semantics, category assignment and full-stack lifecycle
remain open. Existing no-selection clearing is recommended, not newly specified.
Use a crisp bitmap font before final spacing review. Weather/unknown icons remain
proposals; the earlier diamond/bar altitude scale is not approved for the circles.
No animation, new selection policy, prediction, panning, slowdown, fetching,
production policy or hardware work. Other layout explorations remain unselected.

## Verification / risks
Wrap-up checked local links/paths/anchors, instruction consistency, final/staged
whitespace and scope; PNG checks cover 1-bit 400×240, equal 10px padding, unchanged
radar pixels and station-only substitution. No Rust checks rerun: docs/images only.
No interactive, physical-display or performance validation. Run future build/CLI
checks serially (prior relink ENOENT). M2 budgets/scheduling, RAM/safe stack,
Sharp bring-up and RWX remain open; use [0023's frozen revision](decisions/0023-frozen-m2-revision-isolation.md).
