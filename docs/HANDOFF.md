# Overhead — Handoff

_Last updated: 2026-10-17 UTC. Next: propose one bounded altitude-glyph contract._

## Current state / agreed direction
[0041's circular elevation](decisions/0041-circular-elevation-only-contract.md)
is implemented and user visually accepted: static true-elevation cutout, `Elev`
label and padded separator in the opt-in wide view. Without selection the label/
glyph clear, retaining the separator. [0040's notices](decisions/0040-white-backed-bottom-left-notice-contract.md)
are unchanged and visually accepted. Square diagnostics remain default; physical
state, counts, selection and ID hysteresis are unchanged. Other glyphs remain
absent; [0038](decisions/0038-labelled-borderless-glyph-direction.md) is full-stack
direction, not a completed implementation contract.

## Session changes
Added 0041 and `docs/mockups/0041-*.png`; [DESIGN](DESIGN.md#circular-elevation-only-contract)
owns illustrative-reference provenance. Replaced the capsule/fill in
`render/src/glyphs.rs` and `render/src/lib.rs`; updated raster/lifecycle and
`sim/tests/snapshot.rs` coverage. Updated `sim/README.md`, PLAN and decision
cross-references. No dependencies, core/app runtime, fixtures, lockfiles or firmware changes.

## Next step / boundaries
Recommend a **contract-design session**, not implementation, for 0038's static
concentric-circle altitude glyph using true WGS-84 altitude. Agree scale/range,
rounding/saturation, exact raster and invalid/no-selection behavior first; the
old diamond/bar logarithmic mapping is not approved for circles. This next slice
is a recommendation, not authorization. Category rules, speed/period semantics,
full-stack lifecycle, transitions and final debug access remain open. No new
animation, selection policy, collision search, prediction, fetching or hardware.

## Verification / risks
Wrap-up freshly passed serial workspace check/tests, fmt, app/render no-default-
feature checks and strict all-target app/render/sim Clippy; links/anchors, PNGs
and whitespace checked. Earlier 12-case pre/post captures preserved all square
PNG bytes and wide radar/notice pixels; counts/diagnostics unchanged (0041 records
details). [Elevation](../sim/README.md#static-elevation-glyph) and
[notice](../sim/README.md#notice-overlay-review) recipes are self-contained;
fault controls are synthetic. No interactive/display/performance validation.
Keep checks serial (prior relink ENOENT). M2 budgets/scheduling, RAM/safe stack,
Sharp bring-up and RWX remain open; preserve [0023's frozen revision](decisions/0023-frozen-m2-revision-isolation.md).
