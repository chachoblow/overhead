# Overhead — Handoff

_Last updated: 2026-10-12 UTC. Next: refine the M3 fixed-camera lens-zoom slice._

## User direction / session boundary
The user reconsidered camera travel and selected **fixed-camera lens zoom** in
[0025](decisions/0025-fixed-camera-lens-zoom.md), superseding only 0024's
moving-camera/fixed-FOV choice. Keep the north-up perspective view of actual 3D
satellites, aimed at the observer, with shared Earth/satellite geometry and true
observer-horizon eligibility/readouts. Zoom changes FOV, not camera position.
No numeric camera package was accepted; behind-camera indicators are not included.
**This session saves documentation only; the perspective change is not implemented.**

## Current code / next session
Commit `403354e` implements [0022's ground map](decisions/0022-minimal-ground-map-simulator.md).
`app/src/projection.rs` flattens ground positions; `app/src/lib.rs` caches them;
`render/src/lib.rs` draws a scaled Earth circle. `sim/src/config.rs`, snapshot
tests and [sim/README.md](../sim/README.md) still use projected-map radius semantics.
Read PLAN, DESIGN and 0025. Refine fixed camera distance/reference/units and frame,
FOV default/range, near-plane/viewport clipping, Earth depiction and zoom
readout/CLI migration before implementing. Zoom alone cannot put an object behind
the camera, but the fixed camera need not contain every eligible orbit in front.
Replace projection-specific tests with reviewed perspective/lens-zoom cases;
preserve physical outputs, horizon counts and explicit failure behavior.
Weather/GPS is a visual reference, not a default/count decision. Stay in M3.

## Open risks / evidence
M2 operating limits/cadences/overload/scheduling and whole-device RAM/safe stack
remain open. Sharp bring-up and RWX are separate. Frozen M2 checks require
[0023's pinned revision](decisions/0023-frozen-m2-revision-isolation.md), not the
current workspace lockfile; do not renew old evidence to accommodate UI changes.

## Changes / verification
Added 0025; updated 0024/DESIGN/PLAN/PROGRESS/HANDOFF. Fresh export, empty target:
offline locked workspace check/default/all-feature tests, core/app/render
no-default checks, Clippy/fmt,
snapshots (4/22/6 markers) and SDL dummy-loop smoke pass. These verify the current
ground map, not planned lens zoom. Documentation whitespace/local links checked.
Run via [sim usage](../sim/README.md); cached Rust tools/dependencies and Homebrew SDL2 required.
No interactive/hardware/firmware-build review or frozen-evidence rerun/change.
