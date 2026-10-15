# Overhead — Handoff

_Last updated: 2026-10-15 UTC. Next session: scope the first M3 wide-radar slice._

## Current state / agreed direction
Code still implements [0028's square radar](decisions/0028-borderless-square-viewport.md)
and the original diagnostic panel. Normal/debug separation, wide viewport,
edge indicators, glyphs and glyph transitions are NOT implemented.
[0032](decisions/0032-wide-radar-and-selected-satellite-glyphs.md) records the agreed
future direction; [DESIGN + retained mockup](DESIGN.md#selected-satellite-presentation)
are the starting point. No chat, earlier iterations or `/tmp` files are needed.
Wide borderless radar; bare ID beside selection (ISS exception); right-aligned
category/elevation/altitude/speed/vertical-coil period glyphs. Edge indicators
replace exact population counts; normal FOV text is also dropped. Preserve true
state, horizon eligibility and explicit failure honesty. Exact proportions,
category mapping and non-elevation encodings are not settled.

## This session
Design only: added 0032 and `docs/mockups/0032-glyph-radar.png`; updated DESIGN,
PLAN and decision cross-references, and clarified the distinction from current
[simulator usage](../sim/README.md). Retained only the final schematic, not the
iteration history. The scene/glyph values are invented, not real-data evidence.

## Next step / boundaries
Scope one real-data comparison: wide viewport plus off-screen indicators first.
Before coding, agree rectangular FOV/aspect semantics and minimal invalid-depth/
crowding behavior with the user; don't silently turn mock geometry into policy.
Likely code: `app/src/projection.rs`, `app/src/lib.rs`, `render/src/lib.rs`, and
sim config/tests. Defer glyph animation/mappings to a bounded follow-up as needed.
Empty/incomplete-state treatment and debug access are parked, not waived.
No prediction integration, panning, slowdown, fetching or production limits.

## Verification / risks
Docs/static PNG only: final/staged diff and whitespace, local links/anchors/paths,
image mode/dimensions and instruction consistency checked. No Rust checks rerun:
no application code/build inputs changed. No runtime, interactive or hardware
verification; no physical-display legibility or performance claim.
M2 budgets/scheduling, whole-device RAM/safe stack, Sharp bring-up and RWX remain
open. Frozen evidence uses [0023's pinned revision](decisions/0023-frozen-m2-revision-isolation.md),
not this workspace; no evidence renewed.
