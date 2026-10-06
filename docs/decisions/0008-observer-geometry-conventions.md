# 0008 — Observer geometry and singularities

Date: 2026-10-04
Status: accepted (caller API updated by 0009)

## Decision and rationale
Compute `ecef_to_look_angles(target, observer)` by subtracting the WGS-84
observer position and rotating into south/east/zenith. Return slant range,
optional azimuth, and elevation; units follow 0006.

- Azimuth: `atan2(east, -south)` in [0, 2π). Elevation:
  `atan2(zenith, horizontal)` in [-π/2, π/2]. The horizon follows geodetic,
  not geocentric, vertical.
- If horizontal/slant range ≤ 1e-12, azimuth is `None`; do not clamp elevation.
  This suppresses numerical noise at zenith/nadir, not a UI dead zone.
- Exactly coincident positions are an error. Representable nonzero ranges
  remain valid without an arbitrary minimum.
- At poles, supplied observer longitude defines the limiting compass basis;
  observer ECEF still has x=y=0.
- Use `hypot` and normalize displacement before rotation to avoid unnecessary
  overflow. Reject non-finite targets and unrepresentable ranges; delegate
  observer validation to geodetic conversion (0007).

## Consequences
Negative elevation is valid geometry, not visibility. No refraction, terrain,
occultation, threshold, range rate, velocity conversion, or UI policy is added.
Use [0009's bound-state pipeline](0009-core-time-and-frame-api.md).

Independent pymap3d fixtures isolate geometry; Skyfield fixtures verify the
composed ISS pipeline. Analytic tests own singularity behavior, not library
conventions. The named Vallado site is **not** a published `razel` output-vector
comparison. Gates and model differences live in the
[fixture README](../../core/tests/fixtures/README.md). No new Rust dependencies.

References: [pymap3d AER](https://geospace-code.github.io/pymap3d/aer.html),
[ENU implementation](https://github.com/geospace-code/pymap3d/blob/v3.2.0/src/pymap3d/enu.py),
[Skyfield satellites](https://rhodesmill.org/skyfield/earth-satellites.html).
