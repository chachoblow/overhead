# Overhead — Handoff

_Last updated: 2026-10-12 UTC. Next: refine the M3 perspective-camera slice._

## User direction / session boundary
The user tried the simulator and liked the busier weather/GPS view as a first
pass, but expected actual 3D satellites rather than footprints inside Earth.
[0024](decisions/0024-perspective-orbital-view-direction.md) accepts a north-up
camera above the observer looking down toward that location, fixed field of view,
and zoom by moving the camera closer/farther. Altitude affects marker placement;
Earth and satellites share perspective, so markers may lie outside its silhouette.
Retain true observer-horizon eligibility and true-position readouts.
**The user explicitly requested documentation only; do not assume this is implemented.**

## Current code / next session
Commit `403354e` implements the earlier orthographic ground map under
[0022](decisions/0022-minimal-ground-map-simulator.md). Code/tests/CLI are unchanged.
`app/src/projection.rs` flattens ground positions; `app/src/lib.rs` caches them;
`render/src/lib.rs` draws a scaled Earth circle. `sim/src/config.rs`, snapshot
tests and [sim/README.md](../sim/README.md) still use projected-map radius semantics.
Read PLAN, DESIGN and 0024. Refine the camera frame, fixed FOV value, distance
range/defaults/units, near-plane/viewport clipping and consistent Earth depiction
before implementing. Replace projection-specific tests with reviewed perspective
cases; preserve engine outputs, horizon counts and explicit failure behavior.
Use weather/GPS as the visual reference, not a default/count decision; no numeric
camera package settled. Stay in M3: no coastlines, slow-down, prediction or firmware.

## Open risks / evidence
M2 operating limits/cadences/overload/scheduling and whole-device RAM/safe stack
remain open. Sharp bring-up and RWX are separate. Frozen M2 checks require
[0023's pinned revision](decisions/0023-frozen-m2-revision-isolation.md), not the
current workspace lockfile; do not renew old evidence to accommodate UI changes.

## Verification / hardware
This follow-up changed only docs (0024, amendment to 0022, DESIGN/PLAN/PROGRESS
and this handoff); whitespace/local file links checked. No code tests rerun.
Prior `403354e` fresh-source workspace/default/all-feature/no-default/Clippy/fmt
and simulator smoke gates passed; historical pinned gates/52 Python tests passed.
Those results cover the ground-map implementation, not the planned perspective.
No hardware/build/toolchain access; board unchanged, no monitor. Cached tools/dependencies and Homebrew SDL2 required.
