# Overhead — Handoff

_Last updated: 2026-10-17 UTC. Next: propose one bounded static period contract._

## Current state / agreed direction
[0043's speed](decisions/0043-static-speed-glyph-contract.md) is implemented and
user visually accepted: true TEME velocity magnitude, fixed linear 0–12 km/s
clockwise arc, `Spd` label and immediate three-field lifecycle in opt-in wide view.
[0042's altitude](decisions/0042-static-altitude-glyph-contract.md),
[0041's elevation](decisions/0041-circular-elevation-only-contract.md) and
[0040's notices](decisions/0040-white-backed-bottom-left-notice-contract.md) remain
unchanged and accepted. Square diagnostics remain default; counts, selection
and ID hysteresis are unchanged. Category/period remain absent; [0038](decisions/0038-labelled-borderless-glyph-direction.md)
is full-stack direction, not a complete implementation contract.

## Session changes
Added 0043 and `docs/mockups/0043-*.png`; [DESIGN](DESIGN.md#static-speed-contract)
owns illustrative provenance. `app/src/lib.rs` retains one true-speed scalar
from existing propagation; `render/src/{glyphs,lib}.rs` draws the static field.
Extended app/render and independent `sim/tests/snapshot.rs` tests; updated usage/docs.
No dependencies, core propagation, fixtures, lockfiles, firmware or frozen-evidence changes.

## Next step / boundaries
Recommend a **static period-spiral contract-design session**: settle period
meaning/input, fixed scale, raster and lifecycle before coding. No period metric
or next implementation is authorized. Category semantics, full-stack lifecycle,
transitions and debug access remain open; no animation, selection-policy,
collision-search, prediction, fetching or hardware work. Self-contained recipes:
[speed](../sim/README.md#static-speed-glyph), [mixed orbits](../sim/README.md#static-altitude-glyph),
and [notices](../sim/README.md#notice-overlay-review); no temporary files needed.

## Verification / risks
Wrap-up freshly passed serial workspace check/tests, fmt, app/render no-default-
feature checks and strict all-target app/render/sim Clippy. Links/anchors, retained
PNGs and whitespace checked. Earlier 15-case comparisons preserved square bytes
and all wide pixels outside speed; counts/diagnostics unchanged (0043). Recipes
ran headlessly; user accepted executable output; faults are synthetic. No interactive,
physical-display or target-performance validation. Keep checks serial (prior relink
ENOENT). M2 budgets/scheduling, RAM/safe stack, Sharp bring-up and RWX remain open;
the added scalar is not new RAM evidence. Preserve [0023's frozen revision](decisions/0023-frozen-m2-revision-isolation.md).
