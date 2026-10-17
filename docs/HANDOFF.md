# Overhead — Handoff

_Last updated: 2026-10-17 UTC. Next: scope one bounded category contract._

## Current state / agreed direction
[0044's nominal period](decisions/0044-static-period-glyph-contract.md) is
implemented and user visually accepted: fixed logarithmic 90-minute/2½-turn to
24-hour/one-turn spiral, `Per` label and immediate four-field lifecycle in opt-in
wide view; executable acceptance was separate from approval of the mocks/scale.
[0043's speed](decisions/0043-static-speed-glyph-contract.md),
[0042's altitude](decisions/0042-static-altitude-glyph-contract.md),
[0041's elevation](decisions/0041-circular-elevation-only-contract.md) and
[0040's notices](decisions/0040-white-backed-bottom-left-notice-contract.md) remain
unchanged and accepted. Square diagnostics remain default; counts, selection
and ID hysteresis are unchanged. Category remains absent; [0038](decisions/0038-labelled-borderless-glyph-direction.md)
is full-stack direction, not a complete implementation contract.

## Session changes
Added 0044 and `docs/mockups/0044-*.png`; [DESIGN](DESIGN.md#static-period-contract)
owns illustrative provenance (three turns were exploration, not the final scale).
`app/src/lib.rs` reads existing metadata without extra propagation or storage;
`render/src/{glyphs,lib}.rs` draws it. Extended app/render and `sim/tests/snapshot.rs` tests; updated usage/docs.
No dependencies, core propagation, fixtures, lockfiles, firmware or frozen-evidence changes.

## Next step / boundaries
Recommend a **category contract-design session**: settle assignment, ambiguous/
unknown membership and one bounded raster/lifecycle before coding. No category
implementation is authorized. Full-stack transitions and debug access remain
open; no animation, selection-policy, collision-search, prediction, fetching or hardware work.
[Period usage](../sim/README.md#static-period-glyph) links self-contained review recipes; no /tmp files needed.

## Verification / risks
Wrap-up freshly passed serial workspace check/tests, fmt, app/render no-default-
feature checks and strict all-target app/render/sim Clippy. Links/anchors, retained
PNGs and whitespace checked. Earlier 15-case comparisons preserved square bytes,
all wide pixels outside period and diagnostics; synthetic 90-minute parity and
headless recipes passed (0044). User accepted executable output; faults are synthetic.
No interactive/display/target validation; the 4097-point raster is not a measured
20 Hz budget. M2 budgets/scheduling, RAM/safe stack, Sharp bring-up and RWX remain
open. Keep checks serial; preserve [0023's frozen revision](decisions/0023-frozen-m2-revision-isolation.md).
