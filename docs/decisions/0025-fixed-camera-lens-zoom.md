# 0025 — Fixed-camera lens zoom

Date: 2026-10-12
Status: accepted (direction; settings refined and implemented under [0026](0026-fixed-camera-perspective-slice.md))

The original direction-only decision and session boundary are recorded below.
0026 owns the subsequently accepted numeric settings and implemented slice.

## Decision
Use a fixed camera above the configured observer, looking down toward that
location with north up; zoom changes the lens field of view, not camera position.
This supersedes only the moving-camera/fixed-field-of-view choice in
[0024](0024-perspective-orbital-view-direction.md), retaining its actual-3D
perspective direction and physical-state contracts.

## Why
The moving-through-space description was an initial intuition, not a requirement.
Discussion exposed that inward camera travel can pass below eligible satellites,
including directly overhead ones, making them disappear behind the camera.
Lens zoom better matches inspecting a smaller area around the observer: magnify
the same scene rather than explore different orbital heights. Actual satellite
altitude and placement outside Earth's silhouette do not require camera travel.

Camera movement and behind-camera indicators were considered, but neither is
needed for this first lens-zoom slice. A fixed viewpoint loses travel-induced
changes in depth relationships; a very distant camera also weakens perspective.
Choose the fixed distance deliberately during implementation refinement rather
than treating an arbitrarily distant camera as equivalent to the intended view.

## Consequences
- Project actual 3D positions, with Earth and satellites sharing one perspective
  camera. Keep north up and the observer at the aim point. Zoom narrows/widens
  field of view without translating or automatically retargeting the camera.
- Zoom alone cannot move a satellite behind the camera. Objects can still be
  outside the viewport or behind the fixed camera; retain explicit clipping.
  Do not promise that every eligible orbit is in front of it before the camera
  distance and supported visual workload have been reviewed.
- Retain true observer-horizon eligibility (elevation > 0), true-position readouts,
  verified engine outputs, explicit failures and the shared app/renderer split.
- Refine the fixed camera distance and its reference/units, camera frame, FOV
  default/range, near-plane/viewport clipping and consistent Earth depiction.
  No numeric camera package from the discussion was accepted. CLI migration and
  zoom-readout semantics remain open; do not silently reuse map-radius semantics
  or introduce a camera-distance control as the zoom control.
- Test perspective/altitude placement, markers outside Earth's silhouette,
  fixed-viewpoint magnification, clipping and unchanged physical/horizon outputs.
  Review replacement ground-map tests and snapshots rather than blindly updating
  expected values. Use the weather/GPS example as a visual reference, not a
  catalogue-size/default or capacity decision.
- This step is documentation only. Current code still implements
  [0022's ground map](0022-minimal-ground-map-simulator.md); no perspective or lens
  zoom implementation, behind-camera indicators, prediction integration,
  slow-down, coastlines or firmware work is included.
