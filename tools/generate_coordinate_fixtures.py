#!/usr/bin/env python3
"""Offline reference generator; see core/tests/fixtures/README.md.

No Overhead/Rust code is imported. Print deterministic JSON to stdout.
"""

import json
from datetime import datetime
from importlib.metadata import version

import erfa
import pymap3d

VERSIONS = {"pyerfa": "2.0.1.5", "pymap3d": "3.2.0", "numpy": "2.4.2"}
for package, expected in VERSIONS.items():
    assert version(package) == expected, (package, version(package), expected)

# The Vanguard TEME vectors/times come from Vallado's SGP4 verification
# case 00005, also tested in core/tests/propagate.rs. Other vectors are
# deliberately synthetic: the rotation is independent of orbital dynamics.
ROTATIONS = [
    ("start_of_supported_era", "1957-01-01T00:00:00", [6378.137, 0, 0]),
    ("before_j2000", "1999-12-31T23:59:59.123456", [3500, -4500, 4000]),
    ("j2000_noon", "2000-01-01T12:00:00", [7000, 0, 0]),
    ("vanguard_epoch", "2000-06-27T18:50:19.733568", [7022.46529266, -1400.08296755, 0.03995155]),
    ("vanguard_plus_360_min", "2000-06-28T00:50:19.733568", [-7154.03120202, -3783.17682504, -3536.19412294]),
    ("leap_day", "2024-02-29T23:59:59.999999", [-4000, 3000, -5000]),
    ("subsecond", "2026-10-04T12:43:41.833056", [4200, -3500, 4100]),
    ("century_not_leap", "2100-03-01T00:00:00", [0, 7000, 0]),
    ("end_of_supported_era", "2100-12-31T23:59:59.999999", [3500, -4500, -4000]),
]

rotations = []
for name, utc, position in ROTATIONS:
    t = datetime.fromisoformat(utc)
    # Treat the UTC calendar as UT1 explicitly: no leap-second/EOP lookup.
    jd1, jd2 = erfa.dtf2d("UT1", t.year, t.month, t.day, t.hour, t.minute,
                        t.second + t.microsecond / 1e6)
    gmst = float(erfa.gmst82(jd1, jd2))
    # ERFA's coordinate rotation implementation, not Overhead's matrix.
    rotation = erfa.rz(gmst, erfa.ir())
    ecef = [float(x) for x in erfa.rxp(rotation, position)]
    rotations.append({"name": name, "utc": utc, "teme_km": position,
                      "ecef_km": ecef})

# Forward references are exact ellipsoid geometry, independent of our
# iterative inverse. Include high altitudes without relying on the angular
# accuracy of pymap3d's approximate inverse at GEO.
GEODETIC = [
    ("greenwich_equator", 0, 0, 0),
    ("east_equator", 0, 90, 0),
    ("north_pole", 90, 0, 0),
    ("south_pole", -90, 0, 0),
    ("north_near_pole", 89.999999, 123, 0.1),
    ("south_near_pole", -89.999999, -45, 500),
    ("antimeridian", 30, -180, 0),
    ("below_sea_level", 31.5, 35.5, -0.43),
    ("vallado_site_wgs84", 39.007, -104.883, 2.187),
    ("southern_leo", -51.6, 151.2, 420),
    ("inclined_geo_height", 45, -73, 35786),
    ("equatorial_geo", 0, 80, 35786),
    ("high_orbit", -63.4, 179.999999, 100000),
]
geodetic = []
for name, lat, lon, height in GEODETIC:
    ecef = [float(x) / 1000 for x in pymap3d.geodetic2ecef(lat, lon, height * 1000)]
    geodetic.append({"name": name, "latitude_deg": lat, "longitude_deg": lon,
                     "altitude_km": height, "ecef_km": ecef})

print(json.dumps({"generator": "tools/generate_coordinate_fixtures.py",
                  "versions": VERSIONS, "rotations": rotations,
                  "geodetic": geodetic}, indent=2, allow_nan=False))
