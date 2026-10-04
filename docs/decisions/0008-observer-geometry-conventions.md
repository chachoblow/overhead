# 0008 — Observer look angles and singularity conventions

Date: 2026-10-04
Status: accepted

## Decision
Add `ecef_to_look_angles(target_km, observer)` in `overhead-core`, returning
`LookAngles` (slant range in km, optional azimuth and elevation in radians).
Reuse WGS-84 observer conversion and decision 0006's topocentric SEZ convention;
make vertical, coincident-position, and polar behavior explicit.

## Why
- Subtract the observer ECEF position and rotate into south/east/zenith.
  Azimuth is `atan2(east, -south)`, clockwise from true north in [0, 2π);
  elevation is `atan2(zenith, horizontal)` in [-π/2, π/2]. The horizon is
  normal to the WGS-84 geodetic vertical, not the geocentric radius.
- Zenith/nadir have no unique azimuth. Returning `None` avoids inventing
  north or exposing unstable numerical directions. Treat horizontal range /
  slant range <= 1e-12 as vertical to suppress rotation/subtraction roundoff
  at ordinary satellite ranges; do not clamp elevation. This is a numerical
  singularity convention, not a display dead zone or a near-pass policy.
- At exactly coincident ECEF positions neither angle is defined. Return
  `ObservationError::CoincidentPositions`, not a zero range with fake angles.
  Small representable nonzero ranges remain valid; no arbitrary minimum range.
- At a pole geographic north is non-unique. Preserve the supplied longitude
  to define the limiting local compass basis rather than silently replacing
  it with zero. Exact poles still have x=y=0 in observer ECEF coordinates.
- `hypot` avoids unnecessary squared-distance overflow. Normalize the
  displacement before SEZ rotation to avoid overflowing intermediate sums.
- Independent pymap3d ECEF→AER fixtures isolate the look-angle math. Skyfield
  OMM→observer fixtures at fixed ISS timestamps validate the composed pipeline,
  including intentional Earth-orientation model differences. Analytical tests
  handle singularities independently of either library's singularity policy.

## Consequences
- Observer validation is delegated to `GeodeticPosition::to_ecef()` and
  wrapped in `ObservationError::Coordinate`. Reject non-finite target inputs
  and unrepresentable ranges explicitly. Observer heights may be negative,
  with the same terrestrial-domain caveat as decision 0007.
- Negative elevations remain valid geometric outputs. No atmospheric
  refraction, terrain, occultation, visibility threshold, or UI state is
  introduced. Range rate and velocity-frame conversion are out of scope.
- The caller composes `Satellite::state_at(time)` →
  `teme_to_ecef(state.position, time)` → `ecef_to_look_angles(ecef, observer)`.
  No new clock or implicit use of the element epoch is introduced.
- Default core remains no_std/allocation-free with no new Rust dependencies.
  Python packages are only offline fixture-generation dependencies.
- This makes 0006's reference plan concrete: use pymap3d at the named Vallado
  site and other sites, plus Skyfield for the ISS fixture. The named site is
  not a claim to reproduce a published Vallado `razel` output vector.
- Preserve 0006's full-pipeline gates of 0.1 km and 0.01 degrees. Isolated
  geometry uses tighter 1e-8 km and 1e-9 degree gates. See
  `core/tests/fixtures/README.md` for versions, model differences, measured
  errors, and regeneration commands. These gates do not imply equivalent
  real-world satellite accuracy.

References:
- https://geospace-code.github.io/pymap3d/aer.html
- https://github.com/geospace-code/pymap3d/blob/v3.2.0/src/pymap3d/enu.py
- https://rhodesmill.org/skyfield/earth-satellites.html
