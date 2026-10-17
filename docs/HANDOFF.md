# Overhead — Handoff

_Last updated: 2026-10-17 UTC. Next: user review of executable notice snapshots._

## Current state / agreed direction
Implemented: [0037's elevation capsule](decisions/0037-static-elevation-glyph-contract.md)
plus [0040's bottom-left white-backed notices](decisions/0040-white-backed-bottom-left-notice-contract.md)
in the opt-in wide view; square diagnostics remain default. The user approved
bottom-left placement, condition-driven clearing and local visual occlusion of
crossing graphics. Physical state, counts, selection and ID hysteresis are unchanged.
Executable visual acceptance is pending; implementation approval is not that review.
[0038's labelled borderless stack](decisions/0038-labelled-borderless-glyph-direction.md),
circular data glyphs, separator and 10px padding remain agreed but unimplemented.

## Session changes
Added 0039/0040 and `docs/mockups/0039-bottom-left-*.png`; [DESIGN](DESIGN.md#bottom-left-notice-reference)
owns mock provenance. Relocated notices in `render/src/lib.rs`; added renderer
and `sim/tests/snapshot.rs` coverage. Old strip notices are gone; capsule unchanged.
Updated DESIGN/PLAN, decision cross-references and `sim/README.md`; no dependencies,
core/app runtime, fixtures, lockfiles, firmware or frozen-M2 evidence changed.

## Next step / boundaries
Regenerate and review the four [notice overlay examples](../sim/README.md#notice-overlay-review):
selected/empty plus warnings, and an ISS marker/ID crossing with/without a notice.
The self-contained recipe needs no prior temporary files; its faults are synthetic.
After review, propose one bounded glyph contract rather than coding the full stack.
Glyph scales/raster rules, speed/period semantics, category assignment and full-stack
lifecycle remain open. No-selection clearing/retained separator and weather/unknown
icons are proposals; the old diamond/bar altitude mapping is not approved for circles.
No new animation, selection policy, displacement/collision search, prediction,
panning, slowdown, fetching, production policy or hardware scope is approved.

## Verification / risks
Wrap-up passed serial workspace check/tests, fmt, app/render no-default-feature
checks and strict all-target app/render/sim Clippy; local links/anchors and whitespace
checked. The README recipe freshly reproduced all four earlier reviewed PNGs exactly.
Earlier pre/post healthy square and wide 20°/100° PNGs were byte-identical; empty-view
radar changed only within the notice backing. No interactive/display/performance
validation. Keep build/CLI checks serial (prior relink ENOENT). M2 budgets/scheduling,
RAM/safe stack, Sharp bring-up and RWX remain open; preserve [0023's frozen revision](decisions/0023-frozen-m2-revision-isolation.md).
