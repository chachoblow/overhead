# Overhead — Handoff

_Last updated: 2026-10-12 UTC. Next session: revisit M3 panel design; no approved mockup._

## Current state / user direction
Implemented: 240×240 borderless square, provisional 20° FOV, fixed 75,000 km camera
under [0028](decisions/0028-borderless-square-viewport.md). Keep breathing room.
Agreed direction only: [0029](decisions/0029-normal-and-debug-presentation-direction.md)
separates normal and objective debug presentations and cuts routine diagnostic
fields from normal. View/sky information stays with a new presentation; range
is still undecided. No panel layout, graphical treatment or debug access chosen.
The user wants **alive and active, but not busy**; supplied Are.na references
inform DESIGN. Both normal-panel mockups remain unapproved. The user explicitly
deferred further visual iteration to a different session; stop UI work here.

## Implementation / next step
`app/src/projection.rs`, `render/src/lib.rs`, `sim/src/config.rs` and associated
tests implement viewport comparisons and square defaults. Physical tracking,
true horizon eligibility and the original diagnostic panel are unchanged.
Normal/debug splitting, `v`/launch controls and graphical readouts are NOT
implemented. `/tmp/overhead-panel-mockups/` holds disposable exploration PNGs/
scripts only, not source assets or an accepted specification. Illustrated pass
profiles/carets were invented; no prediction integration was approved or added.
Next session: read DESIGN/0029, then scope panel exploration or a small split/
field-reduction slice with the user. Do not implement the last mock by default.
Stay in M3; no slowdown, fetching or production population/cadence decision.
[Simulator usage](../sim/README.md) owns current run/shape-comparison commands.

## Verification / risks
Fresh staged-source Git export, empty target: offline locked workspace check,
default/all-feature tests, core/app/render no-default checks, strict Clippy/fmt
and simulator build pass. Headless snapshots from an unrelated directory with
SDL unavailable reproduce counts; default 20° matches the reviewed square PNG.
Two-second SDL dummy-loop smoke passes. No ignored artifacts or mockups needed;
cached Rust tools/dependencies and Homebrew SDL2 remain prerequisites. No agent
interactive review, hardware access, firmware build or toolchain changes.
M2 budgets/scheduling, whole-device RAM/safe stack, Sharp bring-up and RWX remain
open. Frozen evidence uses [0023's pinned revision](decisions/0023-frozen-m2-revision-isolation.md),
not this workspace; no evidence renewed. Host throughput is not target capacity.
