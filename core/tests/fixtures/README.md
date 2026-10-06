# Test fixtures

Normal Cargo tests read checked-in data: no Python or network needed. Generators
import no Overhead code. Numerical gates verify implementations/models, not
real-world orbit accuracy. JSON angles are degrees; positions/heights/ranges
are km (engine APIs use radians).

## `iss-25544.json`
CelesTrak OMM for ISS (ZARYA), NORAD 25544, stored byte-for-byte as retrieved.

- Source: https://celestrak.org/NORAD/elements/gp.php?CATNR=25544&FORMAT=json
- Retrieved: 2026-10-04T20:15:08Z
- Element epoch: 2026-10-04T12:43:41.833056 UTC

| Test time | UTC | Minutes since epoch |
|---|---|---:|
| T0 | 2026-10-04T12:43:41.833056Z | 0 |
| T1 | 2026-10-04T14:13:41.833056Z | 90 |
| T2 | 2026-10-05T12:43:41.833056Z | 1440 |

**Do not refresh casually.** Replacement requires updated retrieval metadata
and regeneration of every derived expectation. Propagation is meaningful near
the element epoch, not indefinitely.

## `coordinates.json`
Generated 2026-10-04 by `tools/generate_coordinate_fixtures.py`.

- **Rotation:** PyERFA 2.0.1.5 `gmst82`, `rz`, `rxp` (IAU-1982). UTC calendar
  inputs are treated as UT1; no EOP, polar motion, leap tables, or kinematic
  terms. This is our approximate reduction, not full ITRF.
- Nine cases cover J2000 noon, pre-J2000 fractions, leap day, non-leap century
  2100, and supported-era boundaries. Two use Vallado 00005 TEME vectors at
  epoch/+360 min; the rest isolate rotation with synthetic vectors.
- **Geodetic:** pymap3d 3.2.0 `geodetic2ecef`, WGS-84. Forward references test
  both our forward and inverse conversions; its approximate inverse is not
  an oracle at high altitude. Thirteen cases cover poles/near-poles, equator,
  antimeridian, negative altitude, LEO, GEO, and high orbit.
- Gates: Euclidean position/height error <1 m; angular error <1e-6°.
  Do not compare longitude at exact poles. Rust tests add 594 round trips,
  analytic axes, invalid/overflow/non-convergence and time-boundary cases.

Sources: [ERFA gmst82](https://github.com/liberfa/erfa/blob/master/src/gmst82.c),
[Vallado teme2ecef](https://github.com/CelesTrak/fundamentals-of-astrodynamics/blob/main/software/matlab/teme2ecef.m),
[pymap3d ECEF](https://github.com/geospace-code/pymap3d/blob/v3.2.0/src/pymap3d/ecef.py).

Regenerate from the workspace root in a temporary environment:

```sh
python3 -m venv /tmp/overhead-coordinate-refs
/tmp/overhead-coordinate-refs/bin/python -m pip install \
  'pyerfa==2.0.1.5' 'pymap3d==3.2.0' 'numpy==2.4.2'
/tmp/overhead-coordinate-refs/bin/python tools/generate_coordinate_fixtures.py \
  > core/tests/fixtures/coordinates.json
```

## `observer.json`
Generated 2026-10-04 by `tools/generate_observer_fixtures.py`, without refreshing
ISS elements or downloading orbit/EOP data or planetary ephemerides.

- **Isolated geometry:** pymap3d 3.2.0 `ecef2aer`, WGS-84, 27 cases: three ECEF
  targets at nine sites. Includes both hemispheres, equator, antimeridian,
  negative height, poles with nonzero longitude, and near-pole geometry.
- **Pipeline:** Skyfield 1.54 `EarthSatellite.from_omm`, WGS-84 observer
  subtraction, geometric `altaz()` (no refraction). T0/T1/T2 at four sites:
  Vallado (39.007, -104.883, 2.187), Sydney (-33.8688, 151.2093, 0.058),
  equator 80°W (0, -80, 0), equator 90°E (0, 90, 0), in degrees/km.
  The 12 cases include above/below-horizon targets. The named Vallado site
  in either fixture is not a published Vallado output-vector comparison.
- Skyfield `load.timescale(builtin=True)` uses bundled leap-second/delta-T
  tables, no polar-motion table. DUT1 ≈ +0.0935..+0.0937 s is stored per case,
  unlike Overhead's UT1≈UTC. Skyfield's frame reduction and Python sgp4 defaults
  also differ from our AFSPC/GMST-only pipeline; bitwise equality is not expected.
- Isolated gates: 1e-8 km range, 1e-9° angles. Pipeline: 0.1 km / 0.01°;
  azimuth differences wrap north. Observed maxima: 0.04110 km range,
  0.004672° azimuth, 0.004897° elevation.
- Analytic tests cover singularities: pymap3d snaps sub-mm ENU components to
  zero and supplies a vertical azimuth, so it is not the oracle there.
  Our vertical azimuth is `None` ([0008](../../../docs/decisions/0008-observer-geometry-conventions.md)).

Sources: [pymap3d AER](https://geospace-code.github.io/pymap3d/aer.html),
[Skyfield satellites](https://rhodesmill.org/skyfield/earth-satellites.html).

```sh
python3 -m venv /tmp/overhead-observer-refs
/tmp/overhead-observer-refs/bin/python -m pip install \
  'pymap3d==3.2.0' 'numpy==2.4.2' 'skyfield==1.54' \
  'sgp4==2.24' 'jplephem==2.24' 'certifi==2026.7.22'
/tmp/overhead-observer-refs/bin/python tools/generate_observer_fixtures.py \
  > core/tests/fixtures/observer.json
```

Both fixtures were generated with Python 3.14.6. Package versions are pinned,
checked by the scripts, and stored in JSON. Regeneration is offline after
installation; these packages are not Cargo or firmware dependencies.
