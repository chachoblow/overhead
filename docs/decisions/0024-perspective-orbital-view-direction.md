# 0024 — Perspective orbital-view direction

Date: 2026-10-12
Status: accepted (perspective direction; moving-camera/fixed-FOV choice superseded by [0025](0025-fixed-camera-lens-zoom.md); fixed-camera implementation under [0026](0026-fixed-camera-perspective-slice.md))

The original decision below records camera travel and its documentation-only
session boundary. Follow 0025 for fixed-camera lens zoom and 0026 for accepted
settings and implementation; the remaining 3D and physical-state contracts here
still apply.

## Decision
Replace the ground-track projection with a north-up perspective view of satellites'
actual 3D positions: a virtual camera above the configured observer looks down
toward that location, and zoom moves the camera closer or farther away at a fixed
field of view. This supersedes only the ground-map/projection/zoom direction in
[0022](0022-minimal-ground-map-simulator.md); the existing code is not changed yet.

## Why
After trying the simulator, the user said the busier weather/GPS view looked good
as a first pass, but noticed every marker remained inside Earth's outline. The
intended view is like a camera floating in space and moving backwards as one
zooms out, not a map of satellite footprints on Earth's surface. Ground tracks
and orthographic scale changes discard the altitude and perspective cues needed
for that view; simply allowing ground markers outside the old circle would not
represent the intended geometry.

## Consequences
- Project true 3D satellite positions, including altitude, rather than their
  geodetic ground positions. Earth and satellites must share the camera geometry.
  Satellites may appear outside Earth's silhouette; they are not pushed outward
  artificially, and directly overhead objects can still project near the center.
- Keep the camera aimed at the observer with north at the top. Zoom translates
  the camera, not the lens or physical state. Objects outside the camera view or
  behind it are clipped naturally; pulling back can reveal them.
- Retain the existing true observer-horizon eligibility rule (elevation > 0),
  independent of camera position/zoom. This remains the configured observer's
  sky, not every satellite a spacecraft camera might see. Camera clipping is
  additional presentation filtering, not a redefinition of physical passes.
- Preserve the verified engine, explicit time/location, typed failures, and true
  readouts. Keep the shared app/renderer architecture, local replay and basic
  identification. No orbital slowdown, prediction integration, networking or
  firmware work is authorized by this projection change.
- The user prefers the busier weather/GPS example as an initial visual reference.
  This does not fix catalogue size, panel proportions, production capacity, or
  change the default demo catalogue. Reassess density with the new projection.
- Refine the implementation slice in the next session: camera-frame construction,
  fixed field-of-view value, camera-distance defaults/range/units, near-plane and
  viewport clipping, and consistent Earth-limb/visibility treatment. No numeric
  camera package or CLI migration is settled here; do not reinterpret the old
  map-radius label/argument silently.
- Plan regressions for altitude-dependent placement, markers outside Earth's
  silhouette, zoom/perspective consistency, behind-camera/edge cases, and unchanged
  physical outputs/horizon counts. Existing ground-map-specific tests and snapshot
  expectations will need reviewed replacements, not blind expected-value updates.
- At the user's explicit request this session is documentation-only. Current code,
  commands and tests still implement 0022's orthographic ground tracks. Starting
  a fresh session should begin with [HANDOFF](../HANDOFF.md) and refinement, not
  assume the perspective change has already been implemented or tested.

M2 budgets/scheduling remain open under [0021](0021-radar-first-iteration-on-verified-engine.md).
Historical evidence stays isolated under [0023](0023-frozen-m2-revision-isolation.md).
