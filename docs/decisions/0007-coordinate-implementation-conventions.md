# 0007 — Coordinate implementation conventions and reference validation

Date: 2026-10-04
Status: accepted

## Decision
Implement position-only TEME→ECEF conversion using IAU-1982 GMST at the
propagation timestamp, plus WGS-84 geodetic inverse/forward conversions in
`overhead-core`. This clarifies decision 0006's frame wording and makes its
boundary behavior and reference validation concrete; it does not change the
propagator, its AFSPC mode, or the agreed GMST-only approximation.

## Why
- The original “TEME of epoch” wording could imply a frame frozen at the
  orbital element epoch. The direct GMST reduction already specified in
  0006 uses TEME at the ephemeris/propagation time (TEME of date). Astropy's
  satellite example likewise attaches the propagation time as TEME's
  `obstime` before conversion to Earth-fixed coordinates:
  https://docs.astropy.org/en/stable/coordinates/satellites.html
- Use `sgp4::iau_epoch_to_sidereal_time`, not the AFSPC sidereal expression,
  for the agreed IAU-1982 frame rotation. AFSPC propagation mode remains
  independent of this downstream conversion.
- Compute Julian years from Chrono elapsed time since J2000 noon. The
  sgp4 2.4 calendar helper applies a simplified leap-year formula that is
  incorrect after February 2100; our transform tests include that boundary.
- Declare `libm` 0.2 directly for no_std trig/hypot/sqrt. It is already in
  the locked dependency graph through sgp4, so this adds no new library.
- Vallado's iterative geodetic latitude is simple and converges rapidly for
  satellites and terrestrial positions. A normal-projection height formula
  avoids division by sin/cos at equator/poles. Cap work at 16 iterations
  with a 1e-13 radian latitude threshold rather than an unbounded loop.
- Independent ERFA rotations and pymap3d forward WGS-84 positions isolate
  frame and ellipsoid errors. Forward references also avoid depending on an
  approximate inverse's high-altitude accuracy. Retain a pinned generator;
  Python packages are offline tools, never runtime dependencies.

## Consequences
- `teme_to_ecef` accepts positions in km and an explicit UTC timestamp.
  Callers must supply the time used for propagation, not the element epoch.
  It supports years 1957–2100 and rejects explicit leap-second values;
  it neither fetches EOP data nor models leap seconds. This calendar fix
  applies to the transform only; upstream SGP4 element-epoch handling is
  unchanged.
- Inverse geodetic longitude is east-positive in [-π, π), conventionally
  zero on the exact polar axis. Forward longitude accepts both ±π;
  latitude is in [-π/2, π/2], and height is km above WGS-84, not sea level.
- Reject non-finite input/results, invalid angles, the geocenter, and
  non-convergence explicitly. Negative heights are allowed. Deep-interior
  normal-coordinate ambiguity is outside the tracker domain and is not
  resolved by this algorithm.
- Raw TEME velocity is unchanged; rotating it as a position would omit the
  Earth-rotation cross product. Velocity conversion is not part of this task.
- `core/tests/fixtures/README.md` documents generated reference cases and
  accuracy gates (1 m position/height, 1e-6 degrees). The UT1≈UTC and omitted
  polar-motion errors from 0006 remain; these test gates do not imply
  metre-accurate real satellite tracking.
