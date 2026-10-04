# Test fixtures

## iss-25544.json

OMM (JSON) for the ISS (ZARYA), NORAD 25544, as published by Celestrak.

- Source: https://celestrak.org/NORAD/elements/gp.php?CATNR=25544&FORMAT=json
- Retrieved: 2026-10-04T20:15:08Z
- Element epoch: 2026-10-04T12:43:41.833056 UTC
- Stored byte-for-byte as retrieved (single-line JSON array of one object).

Fixed test timestamps for pipeline tests (chosen relative to the element
epoch; propagation is only meaningful near the epoch):

- T0 = 2026-10-04T12:43:41.833056 UTC (the epoch itself, t = 0 min)
- T1 = 2026-10-04T14:13:41.833056 UTC (t = +90 min, ~one orbit)
- T2 = 2026-10-05T12:43:41.833056 UTC (t = +1440 min, one day)

Do not refresh this file casually: expected values in tests are tied to this
exact element set. If it is ever replaced, update the retrieval metadata here
and regenerate every derived expectation.

## coordinates.json

Independent position-transform references, generated on 2026-10-04 by
`tools/generate_coordinate_fixtures.py`. No Overhead code is imported.

- **Rotation:** ERFA `gmst82` (IAU-1982), `rz`, and `rxp`, via PyERFA
  2.0.1.5. UTC calendar inputs are explicitly treated as UT1; no EOP, polar
  motion, leap-second tables, or extra kinematic terms. This matches our
  approximate reduction, not full ITRF. References:
  https://github.com/liberfa/erfa/blob/master/src/gmst82.c and
  https://github.com/CelesTrak/fundamentals-of-astrodynamics/blob/main/software/matlab/teme2ecef.m
- Nine rotation cases include J2000 noon, pre-J2000 fractional seconds,
  leap day, the non-leap century 2100, and supported-era boundaries. Two
  use published Vallado case 00005 TEME vectors at epoch and +360 minutes;
  the others use synthetic vectors to isolate the frame conversion.
- **Geodetic:** pymap3d 3.2.0 `geodetic2ecef`, default WGS-84 ellipsoid,
  produces independent ECEF vectors from specified latitudes, longitudes,
  and heights. Test both our inverse against those specified coordinates
  and our forward conversion against the ECEF references. Includes poles,
  near-poles, equator, antimeridian, negative altitude, LEO, GEO, and high
  orbit. The named Vallado site uses that site's coordinates on WGS-84;
  it is not a claim to reproduce a published Vallado output vector.
  https://github.com/geospace-code/pymap3d/blob/v3.2.0/src/pymap3d/ecef.py
- pymap3d's approximate inverse is deliberately not the oracle at high
  altitudes. Forward ellipsoid geometry supplies the reference instead.
- Fixture ECEF/TEME coordinates and heights are km; geodetic angles are
  degrees for readability. Production APIs use radians. Gates: Euclidean
  position error <1 m, height error <1 m, angular error <1e-6 degrees;
  longitude is not compared at exact poles. These are implementation
  accuracy gates, not an orbit-data or absolute Earth-orientation guarantee.
- Rust tests additionally exercise 594 round trips and analytic axes,
  invalid values, overflow, bounded non-convergence, time validation,
  subsecond motion, calendar continuity, and propagation/geometry composition.

Regenerate with a separate temporary Python environment (not a Cargo or
firmware dependency):

```sh
python3 -m venv /tmp/overhead-coordinate-refs
/tmp/overhead-coordinate-refs/bin/python -m pip install \
  'pyerfa==2.0.1.5' 'pymap3d==3.2.0' 'numpy==2.4.2'
/tmp/overhead-coordinate-refs/bin/python tools/generate_coordinate_fixtures.py \
  > core/tests/fixtures/coordinates.json
```

Generated with Python 3.14.6. Versions are checked by the script and stored
in the JSON. Normal `cargo test` reads the checked-in JSON and needs no
Python packages or network access.

## observer.json

Independent observer geometry and complete ISS pipeline references, generated
on 2026-10-04 by `tools/generate_observer_fixtures.py`. No Overhead code is
imported; the existing ISS elements are not refreshed.

- **Isolated geometry:** pymap3d 3.2.0 `ecef2aer` on WGS-84, 27 cases:
  three explicit ECEF targets at nine sites, including the named Vallado
  site (39.007°N, 104.883°W, 2.187 km), Sydney, both hemispheres, equator,
  antimeridian, negative height, both poles with nonzero longitudes, and
  near-pole geometry. This is independently generated geometry at the
  Vallado site's coordinates, not a published `razel` worked-example vector.
  https://geospace-code.github.io/pymap3d/aer.html
- **Complete pipeline:** Skyfield 1.54 `EarthSatellite.from_omm`, WGS-84
  observer subtraction, and geometric `altaz()` with no refraction.
  Twelve cases use `iss-25544.json` at T0/T1/T2 above, viewed from the first
  four sites (Vallado site, Sydney, equator 80°W, equator 90°E). Include
  both above- and below-horizon targets. No downloaded orbit/EOP data or
  planetary ephemeris is used.
  https://rhodesmill.org/skyfield/earth-satellites.html
- Skyfield's `load.timescale(builtin=True)` uses the pinned wheel's bundled
  leap-second/delta-T tables. No polar-motion table is installed. Its DUT1
  estimate is approximately +0.0935..+0.0937 s for these timestamps (stored
  per case); unlike Overhead it is not forced to zero. Skyfield also uses
  its own satellite frame reduction and Python sgp4 initialization defaults;
  Overhead retains AFSPC propagation and GMST-only UT1≈UTC reduction. The
  fixture intentionally checks agreement within the agreed gates, not bitwise
  equality between these models. Regeneration is offline after installation.
- Angles in JSON are degrees; ECEF coordinates, observer height, and range
  are km. Production engine APIs use radians. Isolated geometry gates are
  1e-8 km in range and 1e-9 degrees in azimuth/elevation. Full-pipeline gates
  remain decision 0006's 0.1 km / 0.01 degrees; azimuth differences wrap
  across north. Observed pipeline maxima: 0.04110 km, 0.004672° azimuth,
  0.004897° elevation. These are implementation/model comparisons, not
  real-world orbit accuracy claims.
- Rust analytic tests separately cover cardinal directions/quadrants,
  north wrapping, exact horizon, below horizon, zenith/nadir, polar axes,
  ellipsoid normals at different heights, antimeridian equivalence,
  coincident/tiny ranges, singularity tolerance, invalid inputs, and numeric
  overflow. pymap3d snaps sub-millimetre ENU components to zero and supplies
  a numeric azimuth on the vertical; those library-specific singularities
  are not our oracle. Our vertical azimuth is `None` per decision 0008.

Regenerate in a separate temporary environment (no Cargo/firmware dependency):

```sh
python3 -m venv /tmp/overhead-observer-refs
/tmp/overhead-observer-refs/bin/python -m pip install \
  'pymap3d==3.2.0' 'numpy==2.4.2' 'skyfield==1.54' \
  'sgp4==2.24' 'jplephem==2.24' 'certifi==2026.7.22'
/tmp/overhead-observer-refs/bin/python tools/generate_observer_fixtures.py \
  > core/tests/fixtures/observer.json
```

Generated with Python 3.14.6. All installed runtime package versions are
pinned, checked by the generator, and recorded in JSON. Normal `cargo test`
reads the checked-in fixture and needs neither Python nor network access.
