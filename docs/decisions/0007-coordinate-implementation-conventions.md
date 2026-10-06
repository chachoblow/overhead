# 0007 — Coordinate implementation conventions

Date: 2026-10-04
Status: accepted (caller API updated by 0009)

## Decision and rationale
Implement position-only TEME→ECEF and WGS-84 forward/inverse geodetic conversion.
This clarifies 0006 without changing AFSPC propagation or the GMST-only model.

- TEME is at **propagation time**, not frozen at element epoch; see
  [Astropy's satellite example](https://docs.astropy.org/en/stable/coordinates/satellites.html).
- Use `sgp4::iau_epoch_to_sidereal_time`, independently of AFSPC propagation
  mode. Compute Julian years using Chrono elapsed time from J2000 noon: sgp4
  2.4's calendar helper is wrong after February 2100. Upstream propagation
  epoch handling remains unchanged.
- Use iterative ellipsoid-normal latitude, capped at 16 iterations with a
  1e-13 radian threshold, and pole-safe normal-projection height. Declare
  existing `libm` 0.2 directly for no_std math; no new library.
- Validate rotation with ERFA and ellipsoid geometry with pymap3d **forward**
  references, avoiding its approximate inverse's high-altitude limitations.
  See [fixture provenance](../../core/tests/fixtures/README.md).

## Boundary contract
- Rotate at the propagation timestamp; prefer the bound-state API in
  [0009](0009-core-time-and-frame-api.md). Its shared UTC contract applies.
- Inverse longitude: east-positive [-π, π), zero on the exact polar axis.
  Forward longitude accepts ±π; latitude is [-π/2, π/2].
- Heights are WGS-84 ellipsoidal km; negative heights are valid. Reject invalid
  angles, non-finite input/results, geocenter, and non-convergence. Ambiguous
  deep-interior normal coordinates are outside the tracker domain.
- TEME velocity is unchanged. Earth-fixed velocity would also need an
  Earth-rotation cross product; it is not a position rotation.

Algorithm reference: [Vallado ecef2ll](https://github.com/CelesTrak/fundamentals-of-astrodynamics/blob/main/software/matlab/ecef2ll.m),
using WGS-84 rather than that example's older constants.
