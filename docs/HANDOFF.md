# Overhead — Handoff

_Last updated: 2026-10-15 UTC. Next session: scope selected-marker identification._

## Current state / agreed direction
[0033's opt-in `--wide` comparison](decisions/0033-wide-radar-comparison-contract.md)
is implemented and visually approved by the user: 352×240, vertical FOV preserving
pixel scale, one hollow arrow per eligible off-screen satellite, overlaps allowed.
No suppression, grouping or displacement; no invalid-depth arrows. Selection
stays visible-marker-only. Minimal missing-frame/empty-marker/tracking/ingestion
notices remain explicit, including warnings alongside valid markers.
The default remains the square radar/diagnostic panel. Marker-adjacent IDs,
category/data glyphs, transitions and the full normal/debug view are NOT implemented.

## This session
Changed `app/src/{lib,projection}.rs`, `render/src/lib.rs`, simulator config/reporting
and `sim/tests/snapshot.rs`; recorded 0033 and updated design/plan/usage.
[Comparison commands and counts](../sim/README.md#wide-radar-comparison) reproduce
real-data snapshots without this chat or `/tmp` assets. No engine, dependency,
lockfile, fixture, firmware or frozen-M2 changes.

## Next step / boundaries
User deferred further implementation to next session. Scope one bounded
selected-marker identification slice: bare NORAD ID and `25544 -> ISS` under
[0032 / DESIGN](DESIGN.md#selected-satellite-presentation). Agree edge placement
and label-collision behavior before coding; keep approved radar/arrows unchanged.
Glyph mappings/transitions, final debug access and refined empty/incomplete-state
treatment remain open. No prediction integration, panning, slowdown, fetching,
new selection ranking or production limits.

## Verification / risks
Wrap-up reran `cargo fmt --all -- --check`, `cargo check --workspace` and
`cargo test --workspace`: all pass, including deterministic headless CLI tests
at start/+30 seconds against independent geometry. Earlier this session generated
square/wide weather/GPS and wide demo PNGs; selected PNGs visually inspected.
No agent interactive replay, hardware, physical-display or performance validation.
An earlier concurrent `cargo run` relink caused CLI spawn ENOENT; serial reruns
pass. Do not rebuild the executable while its CLI tests run. M2 budgets/scheduling,
whole-device RAM/safe stack, Sharp bring-up and RWX remain open; frozen evidence
still uses [0023's pinned revision](decisions/0023-frozen-m2-revision-isolation.md).
