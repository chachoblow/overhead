# 0036 — Shared glyph capsule

Date: 2026-10-17
Status: accepted (visual direction; shortened elevation-only capsule implemented under [0037](0037-static-elevation-glyph-contract.md), full stack unimplemented)

## Decision
Wrap the selected-satellite glyph stack in one shared vertical rounded capsule,
with a 1-pixel outline and one centered break on each long side. Retain black
marks on white and the existing wide-radar layout; this amends the unboxed-strip
brief in [0032's direction](0032-wide-radar-and-selected-satellite-glyphs.md).

## Why
A user-supplied segmented-LCD reference grouped symbols within a long rounded
enclosure. After reviewing continuous and multiply interrupted outlines, then
1px/2px versions with one central opening per side, the user selected the 1px,
single-break treatment for now. This groups the instruments without individual
boxes or internal dividers.

## Consequences
- One enclosure for the whole category/elevation/altitude/speed/period stack,
  not one box per glyph. The radar itself remains borderless; this does not
  replace [0033's approved viewport/arrows](0033-wide-radar-comparison-contract.md)
  or the square/diagnostic executable default.
- [DESIGN's retained mock](../DESIGN.md#glyph-capsule-reference) owns the visual
  starting point and provenance. The 1px stroke and one centered opening on each
  side are chosen; exact padding, corner radius, gap length and glyph footprints
  still need a bounded implementation contract. The mock is not a golden test.
- The glyphs in the mock are illustrative, not new category classifications,
  numerical scales or physically consistent readouts. Non-elevation mappings,
  unknown/multiple category handling, speed/period semantics, transitions and
  no-selection behavior remain to be scoped. Preserve explicit tracking and
  ingestion warnings when allocating space for the capsule.
- Circular-radar/info-brick, noise-field and visible-population graph mockups
  were exploration only. No alternate layout, graph mapping, retained history
  or animation was approved. They do not supersede the wide/glyph direction.
- Scope the next M3 slice before coding. No renderer/app implementation,
  dependency, prediction, scheduler, production-capacity or hardware change is
  implied by this visual approval.
