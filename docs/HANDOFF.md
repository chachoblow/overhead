# Overhead — Handoff

_Last updated: 2026-10-16 UTC. Next: review label zoom visibility._

## Current state / agreed direction
[0033's wide radar/arrows](decisions/0033-wide-radar-comparison-contract.md) remain
user-visually-approved; square diagnostics remain default. [0034](decisions/0034-selected-marker-identification.md)
adds selected-marker NORAD ID / ISS, right-side placement with left fallback,
vertical clamping and transparent overlaps. User review flagged overview crowding;
[0035](decisions/0035-label-zoom-visibility.md) implements the approved refinement:
hide text at ≥70° vertical FOV, restore at ≤60°, with configurable hysteresis.
Selected diamonds stay visible. Dither mockups were exploration only; no fade or
animation was implemented. Glyphs and final normal/debug access remain open.

## Session changes
`app/src/{identification,lib,projection}.rs`: clockless label policy/state.
`render/src/lib.rs`: labels; `sim/src/{config,main}.rs`: configurable thresholds.
App/render and `sim/tests/snapshot.rs` test both contracts; docs record 0034/0035.
No core, dependency, lockfile, source-fixture, firmware or frozen-M2 changes.

## Next step / boundaries
Review [zoom comparisons](../sim/README.md#label-zoom-visibility) at 70° and the
60°–70° zoom round trip before further UI work; [ID examples](../sim/README.md#identification-review)
cover edges, ISS and six-digit labels without this chat or temporary assets.
Defaults are provisional, not collision guarantees. Startup shows below the hide
threshold; frame/selection changes preserve history. Keep approved arrows,
selection and warning notices unchanged. Glyph mappings/transitions need separate
scope; no prediction, panning, slowdown, fetching or production limits/cadences.

## Verification / risks
Wrap-up reran fmt check, `cargo check --workspace`, `cargo test --workspace`,
app/render `--no-default-features` checks and strict all-target Clippy for
app/render/sim: all pass. Local Markdown links/anchors and diff whitespace checked.
Earlier headless visual/pixel comparisons: square baseline unchanged; hiding only
removes label ink, configurable overrides restore it. Tests cover threshold/history,
identity/placement, retained diamonds/warnings and independent geometry/determinism.
No interactive/physical-display/performance validation; user zoom review pending.
Build and CLI tests serially (prior relink ENOENT). M2 budgets/scheduling,
RAM/safe stack, Sharp bring-up and RWX remain open; frozen evidence uses
[0023's pinned revision](decisions/0023-frozen-m2-revision-isolation.md).
