# 0026 — Fixed-camera perspective simulator slice

Date: 2026-10-12
Status: accepted (camera height superseded by [0027](0027-higher-fixed-camera-overview.md); circular viewport/clipping superseded by [0028](0028-borderless-square-viewport.md); remaining contracts unchanged)

## Decision
Implement [0025's lens-zoom direction](0025-fixed-camera-lens-zoom.md) with a
camera fixed 50,000 km along geodetic local up from the configured observer,
aimed at that observer with north up. Use a 20° default full circular field of
view, adjustable from 0.1° to 100°, and project actual ECEF satellite positions
and the WGS-84 Earth outline through that same camera.

## Why
The user accepted these starting settings after clarifying that lens zoom alone
can make Earth small: at a terrestrial observer it spans roughly 65% of the
radar diameter at 20°, 20% at 60°, and 10% at 100°. Raising the camera is not
necessary for the small-Earth overview and would weaken perspective differences
between Earth and higher satellites. The chosen height puts ordinary LEO/GPS/GEO
positions in front, but does not promise this for every highly elliptical orbit.
These are reviewable simulator settings, not production operating limits.

## Consequences
- The camera is fixed in the observer's Earth-fixed frame, not inertial space.
  Height is relative to the supplied observer position, not sea level or Earth's
  center. Geodetic local east/right and north/up define the image axes; at exact
  poles the supplied longitude defines their limiting orientation.
- FOV is the full angle across the circular viewport, not a map radius or a
  ground/slant distance. Wheel/arrows scale magnification geometrically (1.2×
  per step), clamped to the lens range. No camera travel, retargeting, automatic
  zoom, or behind-camera indicators are added.
- Keep true elevation > 0 eligibility before presentation clipping. Accept
  forward depth >= 1 km and marker centers on/inside the viewport; clip entire
  glyphs to the circle too. There is no far plane. Behind-camera/near-plane
  clipping is not a tracking failure and lens zoom cannot change forward depth.
- Earth is an outline of the perspective-projected WGS-84 ellipsoid, not an
  independently scaled sphere. Its image center can differ slightly from the
  observer aim point. Clip the outline even when only part fits; rendering work
  is bounded by viewport dimensions rather than the enlarged globe's extent.
  No coastlines, filled occlusion mask, or separate camera-occultation eligibility
  rule is introduced; true observer-horizon filtering remains the sky contract.
- Reject camera configurations whose Earth view is not finite or whose ellipsoid
  is not wholly beyond the near plane, rather than silently drawing invalid
  geometry. This affects extreme observer heights, not ordinary terrestrial sites.
  Preserve typed per-object failures and all physical outputs independently.
- Replace `--radius-km` with `--fov-deg` and `R ... KM (MAP)` with `FOV ... DEG`.
  The old option errors with migration guidance; it is never silently reinterpreted.
  Keep the existing demo catalogue/time/site and radar layout defaults. An empty
  default viewport must not trigger automatic widening; see [usage](../../sim/README.md).
- Replacement tests cover altitude/depth effects, markers outside Earth's limb,
  lens-only magnification, near-plane/circular clipping, poles/dateline, independent
  ellipsoid tangency, physical look-angle projection references, true counts,
  failures and deterministic snapshots. Old ground-map pixel/count expectations
  are not carried forward as if the projection were unchanged.
- Stay in M3: review moving radar, density and information-area proportions next.
  No prediction integration, slowdown, live fetching, new dependencies, engine
  changes, firmware work or renewal of frozen M2 evidence is part of this slice.
