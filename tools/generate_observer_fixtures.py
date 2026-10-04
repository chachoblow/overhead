#!/usr/bin/env python3
"""Offline independent look-angle references; see core/tests/fixtures/README.md.

No Overhead code, live orbit/EOP data, or network access. JSON goes to stdout.
"""

import json
from datetime import datetime, timedelta, timezone
from importlib.metadata import version
from pathlib import Path

import pymap3d
from skyfield.api import EarthSatellite, load, wgs84

VERSIONS = {
    "pymap3d": "3.2.0", "numpy": "2.4.2", "skyfield": "1.54",
    "sgp4": "2.24", "jplephem": "2.24", "certifi": "2026.7.22",
}
for package, expected in VERSIONS.items():
    assert version(package) == expected, (package, version(package), expected)

SITES = [
    ("vallado_site_wgs84", 39.007, -104.883, 2.187),
    ("sydney", -33.8688, 151.2093, 0.058),
    ("equator_west", 0, -80, 0),
    ("equator_east", 0, 90, 0),
    ("antimeridian", 30, -180, 0.4),
    ("negative_height", 31.5, 35.5, -0.43),
    ("north_pole", 90, 123, 0),
    ("south_pole", -90, -45, 0),
    ("near_pole", 89.999999, 123, 0.1),
]
TARGETS = [[7000, -2000, 3500], [-4200, 3500, -4100], [42000, 1000, 0]]


def site_fields(name, lat, lon, height):
    return {"name": name, "latitude_deg": lat, "longitude_deg": lon,
            "altitude_km": height}


def measurements(az, el, distance_km):
    return {"azimuth_deg": float(az), "elevation_deg": float(el),
            "range_km": float(distance_km)}


geometry = []
for name, lat, lon, height in SITES:
    for target in TARGETS:
        az, el, distance = pymap3d.ecef2aer(
            *(x * 1000 for x in target), lat, lon, height * 1000)
        geometry.append({"observer": site_fields(name, lat, lon, height),
                         "ecef_km": target,
                         **measurements(az, el, distance / 1000)})

# Use the pinned Skyfield wheel's built-in leap-second and delta-T tables.
# No EOP/polar-motion table is installed; DUT1 is Skyfield's built-in estimate,
# NOT forced to UTC as in Overhead. altaz() defaults to no refraction.
fixtures = Path(__file__).resolve().parents[1] / "core/tests/fixtures"
fields = json.loads((fixtures / "iss-25544.json").read_text())[0]
ts = load.timescale(builtin=True)
satellite = EarthSatellite.from_omm(ts, fields)
epoch = datetime.fromisoformat(fields["EPOCH"]).replace(tzinfo=timezone.utc)
pipeline = []
for minutes in [0, 90, 1440]:
    utc = epoch + timedelta(minutes=minutes)
    time = ts.from_datetime(utc)
    for name, lat, lon, height in SITES[:4]:
        observer = wgs84.latlon(lat, lon, elevation_m=height * 1000)
        position = (satellite - observer).at(time)
        assert position.message is None, position.message
        altitude, azimuth, distance = position.altaz()
        pipeline.append({"utc": utc.replace(tzinfo=None).isoformat(),
                         "dut1_seconds": float(time.dut1),
                         "observer": site_fields(name, lat, lon, height),
                         **measurements(azimuth.degrees, altitude.degrees, distance.km)})

print(json.dumps({"generator": "tools/generate_observer_fixtures.py",
                  "versions": VERSIONS, "geometry": geometry,
                  "iss_pipeline": pipeline}, indent=2, allow_nan=False))
