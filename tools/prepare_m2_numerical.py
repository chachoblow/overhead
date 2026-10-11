#!/usr/bin/env python3
"""Select published vectors and existing fixtures offline; never run Overhead."""
import argparse
import hashlib
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEST = ROOT / "tools/fixtures/m2-numerical/references.json"
UPSTREAM = "49df0479c950917c0a2c5ac31f2db6477bba57f6"
IDS = (5, 8195, 24208, 28129)
MINUTES = (0, 360, 1440)


def ephemeris(text):
    if "CoordinateSystem\t\t\tTEME" not in text or "DistanceUnit\t\t\tKilometers" not in text:
        raise ValueError("expected TEME kilometres")
    body = text.split("EphemerisTimePosVel", 1)[1].split("END Ephemeris", 1)[0]
    rows = {}
    for line in body.splitlines():
        if not line.strip():
            continue
        values = [float(v) for v in line.split()]
        if len(values) != 7 or not all(math.isfinite(v) for v in values):
            raise ValueError("invalid ephemeris row")
        seconds = values[0]
        if seconds in rows:
            raise ValueError("duplicate ephemeris time")
        rows[seconds] = values[1:]
    return rows


def prepare():
    sources = []

    def read(path, upstream_path=None):
        data = (ROOT / path).read_bytes()
        sources.append({"path": path, "bytes": len(data),
                        "sha256": hashlib.sha256(data).hexdigest(),
                        "url": None if upstream_path is None else
                        f"https://raw.githubusercontent.com/CelesTrak/fundamentals-of-astrodynamics/{UPSTREAM}/{upstream_path}"})
        return data.decode("utf-8")

    directory = "tools/fixtures/m2-numerical/vallado"
    tle = read(f"{directory}/SGP4-VER.TLE", "datalib/SGP4-VER.TLE").splitlines()
    records = {}
    for index, line in enumerate(tle):
        if line.startswith("1 "):
            ident = int(line[2:7])
            if ident in IDS:
                line2 = tle[index + 1]
                if not line2.startswith(f"2 {ident:05}") or ident in records:
                    raise ValueError("mismatched/duplicate TLE identity")
                records[ident] = [line[:69], line2[:69]]
    if set(records) != set(IDS):
        raise ValueError("missing TLE")
    teme = []
    for ident in IDS:
        name = f"{ident:05}.e"
        rows = ephemeris(read(f"{directory}/{name}", f"software/cpp/TestSGP4/TestSGP4/{name}"))
        for minutes in MINUTES:
            vector = rows[minutes * 60]
            teme.append({"id": f"teme-{ident:05}-{minutes}", "norad_id": ident,
                         "tle": records[ident], "minutes": minutes,
                         "position_km": vector[:3], "velocity_km_s": vector[3:]})
    coordinates = json.loads(read("core/tests/fixtures/coordinates.json"))
    observer = json.loads(read("core/tests/fixtures/observer.json"))
    read("core/tests/fixtures/iss-25544.json")
    suites = {"teme": teme, "rotations": coordinates["rotations"],
              "geodetic": coordinates["geodetic"], "geometry": observer["geometry"],
              "iss_pipeline": observer["iss_pipeline"]}
    if [len(v) for v in suites.values()] != [12, 9, 13, 27, 12]:
        raise ValueError("fixture selection changed")
    return {"schema_version": 1, "sources": sources,
            "selection": "all existing independent coordinate/observer fixture rows, in original order; TEME ID then 0/360/1440 minutes",
            "strict_less_than_tolerances": {
                "teme_position_axis_km": 1e-6, "teme_velocity_axis_km_s": 1e-9,
                "coordinate_position_norm_km": 1e-3, "geodetic_height_km": 1e-3,
                "geodetic_angle_deg": 1e-6, "geometry_range_km": 1e-8,
                "geometry_angle_deg": 1e-9, "pipeline_range_km": 0.1,
                "pipeline_angle_deg": 0.01}, **suites}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = (json.dumps(prepare(), indent=2, ensure_ascii=False) + "\n").encode()
    if args.check:
        if DEST.read_bytes() != data:
            raise SystemExit("numerical references differ; review before regeneration")
        print("numerical references match original sources and complete fixture selection")
    else:
        DEST.write_bytes(data)
        print(f"wrote {DEST.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
