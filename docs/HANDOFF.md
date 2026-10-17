# Overhead — Handoff

_Last updated: 2026-10-17 UTC. Next: propose one bounded static speed-arc contract._

## Current state / agreed direction
[0042's altitude](decisions/0042-static-altitude-glyph-contract.md) is implemented
and user visually accepted: fixed compressed true WGS-84 altitude, concentric
circles, `Alt` label and immediate two-field lifecycle in the opt-in wide view.
[0041's elevation](decisions/0041-circular-elevation-only-contract.md) and
[0040's notices](decisions/0040-white-backed-bottom-left-notice-contract.md)
are unchanged and accepted. Square diagnostics remain default; physical state,
counts, selection and ID hysteresis are unchanged. Category, speed and period
remain absent; [0038](decisions/0038-labelled-borderless-glyph-direction.md) is
full-stack direction, not a complete implementation contract.

## Session changes
Added 0042 and `docs/mockups/0042-*.png`; [DESIGN](DESIGN.md#static-altitude-contract)
owns illustrative provenance. Implemented altitude in `render/src/{glyphs,lib}.rs`;
extended raster/mapping/lifecycle and independent `sim/tests/snapshot.rs` coverage.
Updated `sim/README.md`, PLAN and decision cross-references. No dependencies,
core/app runtime, fixtures, lockfiles, firmware or frozen-evidence changes.

## Next step / boundaries
Recommend a **static speed-arc contract-design session**: settle reference frame/
input, scale, raster and lifecycle before coding. No speed metric or next
implementation is authorized. Category/period semantics, full-stack lifecycle,
transitions and debug access remain open; no animation, selection-policy,
collision-search, prediction, fetching or hardware work. Self-contained recipes:
[altitude](../sim/README.md#static-altitude-glyph), [elevation](../sim/README.md#static-elevation-glyph),
and [notices](../sim/README.md#notice-overlay-review); no temporary files needed.

## Verification / risks
Wrap-up freshly passed serial workspace check/tests, fmt, app/render no-default-
feature checks and strict all-target app/render/sim Clippy. Links/anchors, retained
PNGs and whitespace checked. Earlier 12-case comparisons preserved square PNG
bytes and every wide pixel outside altitude; counts/diagnostics unchanged (0042).
User accepted fresh executable output; faults are synthetic. No interactive,
physical-display or target-performance validation. Keep checks serial (prior
relink ENOENT). M2 budgets/scheduling, RAM/safe stack, Sharp bring-up and RWX remain
open; preserve [0023's frozen revision](decisions/0023-frozen-m2-revision-isolation.md).
