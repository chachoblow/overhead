# Overhead — Handoff

_Last updated: 2026-10-17 UTC. Next: scope the next glyph contract; altitude recommended._

## Current state / agreed direction
[0037's static elevation slice](decisions/0037-static-elevation-glyph-contract.md)
is implemented in the opt-in wide view and user visually accepted: true elevation,
short shared capsule, separate warnings, no capsule/glyph without selection.
Keep [0033's radar/arrows](decisions/0033-wide-radar-comparison-contract.md),
[0034's ID placement](decisions/0034-selected-marker-identification.md) and
[0035's accepted configurable 70°/60° hysteresis](decisions/0035-label-zoom-visibility.md).
Square diagnostics remain default. Other glyphs and transitions are unimplemented.

## Session changes
Added `render/src/glyphs.rs`, integrated it in `render/src/lib.rs`, and extended
renderer tests plus `sim/tests/snapshot.rs` with independent real-data glyph checks.
`render/Cargo.toml` / `Cargo.lock` add a direct edge to existing `libm` for no_std
sine/cosine, with no new package/version. Added 0037; updated DESIGN/PLAN, decision
cross-references and [simulator usage](../sim/README.md#static-elevation-glyph).
App/core/simulator runtime logic, fixtures, firmware and frozen-M2 evidence unchanged.

## Next step / boundaries
Recommendation, not approved scope: agree a static altitude-glyph contract using
existing true WGS-84 altitude. Decide gap mapping/range/saturation and footprint
inside the current capsule before coding; do not infer scales from the mock.
Category mapping, speed/period semantics, full-stack layout, transitions and final
normal/debug access remain open. Preserve warnings and visible-only selection.
Circular/info-brick, noise-field and population-graph alternatives remain exploration.
No prediction, history, panning, slowdown, fetching, production policy or hardware work.

## Verification / risks
Wrap-up reran `cargo check --workspace`, `cargo test --workspace`, fmt check,
app/render `--no-default-features` checks and strict all-target app/render/sim Clippy;
all passed. Markdown links/anchors, final/staged whitespace and scope reviewed.
Earlier headless comparisons at 0.1°/20°/70°/100° preserved square PNG bytes, wide
radar pixels and physical/count reports. User accepted the rendered snapshot set;
reproduce via simulator usage above, not chat or temporary assets. No interactive,
physical-display or performance validation. Run build/CLI checks serially (prior
relink ENOENT). M2 budgets/scheduling, RAM/safe stack, Sharp bring-up and RWX remain
open; frozen evidence uses [0023's pinned revision](decisions/0023-frozen-m2-revision-isolation.md).
