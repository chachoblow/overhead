#!/usr/bin/env python3
"""Offline Natural Earth 1:110m coastline size/work preflight; no runtime asset or renderer.

Stdlib only. Never treats sampled endpoint visibility as a correct tangent clipper.
Optional --out-dir writes *temporary* candidate byte-count artifacts, not an asset.
Usage: python3 tools/offline/coastline_preflight.py /tmp/ne_110m_coastline.zip
"""
import argparse
import hashlib
import math
import struct
import zipfile

EXPECTED_SHA256 = "664449b39070027e882abb295974d182afec18ca21107273d17e9e8bf6f64817"
A = 6378.137
B = A * (1 - 1 / 298.257223563)
HEIGHT = 75_000.0
SITES = {"Colorado": (39.007, -104.883, 2.187),
         "San Francisco": (37.7749, -122.4194, 0.0),
         "equator/dateline": (0.0, 179.9, 0.0),
         "north polar": (89.9, 0.0, 0.0)}
FOVS = (0.1, 1.0, 5.0, 20.0, 100.0)


def load(path):
    blob = path.read_bytes()
    digest = hashlib.sha256(blob).hexdigest()
    if digest != EXPECTED_SHA256:
        raise ValueError(f"archive SHA-256 mismatch: {digest}")
    with zipfile.ZipFile(path) as archive:
        version = archive.read("ne_110m_coastline.VERSION.txt").decode().strip()
        shp = archive.read("ne_110m_coastline.shp")
        prj = archive.read("ne_110m_coastline.prj").decode()
    if version != "4.1.0" or "WGS_1984" not in prj:
        raise ValueError(f"unexpected version or projection: {version}, {prj!r}")
    if len(shp) < 100 or struct.unpack_from(">I", shp)[0] != 9994 or struct.unpack_from("<II", shp, 28) != (1000, 3):
        raise ValueError("not a version-1000 PolyLine shapefile")
    if struct.unpack_from(">I", shp, 24)[0] * 2 != len(shp):
        raise ValueError("shapefile length mismatch")
    offset, parts, records, overlong = 100, [], 0, 0
    while offset < len(shp):
        if offset + 8 > len(shp):
            raise ValueError("short record header")
        _, length = struct.unpack_from(">II", shp, offset)
        offset += 8
        end = offset + 2 * length
        if end > len(shp) or end - offset < 44 or struct.unpack_from("<I", shp, offset)[0] != 3:
            raise ValueError("invalid PolyLine record")
        count_parts, count_points = struct.unpack_from("<II", shp, offset + 36)
        if end - offset != 44 + 4 * count_parts + 16 * count_points:
            raise ValueError("invalid part/point lengths")
        indices = struct.unpack_from(f"<{count_parts}I", shp, offset + 44) + (count_points,)
        if not count_parts or indices[0] != 0 or any(a >= b for a, b in zip(indices, indices[1:])):
            raise ValueError("invalid part offsets")
        xy = [struct.unpack_from("<dd", shp, offset + 44 + 4 * count_parts + i * 16)
              for i in range(count_points)]
        if any(not (math.isfinite(x) and math.isfinite(y) and -180 <= x <= 180 + 1e-6 and -90 <= y <= 90)
               for x, y in xy):
            raise ValueError("invalid lon/lat")
        overlong += sum(x > 180 for x, _ in xy)
        # The archive contains four ~180+epsilon longitudes. Normalize only
        # these after checking their deviation; do not silently wrap bad data.
        xy = [(180.0 if x > 180 else x, y) for x, y in xy]
        parts.extend(xy[a:b] for a, b in zip(indices, indices[1:]))
        records += 1
        offset = end
    return records, parts, overlong


def surface(lon, lat):
    lat, lon = math.radians(lat), math.radians(lon)
    s, c = math.sin(lat), math.cos(lat)
    n = A / math.sqrt(1 - (1 - (B/A)**2) * s * s)
    xyz = ((n*c*math.cos(lon)), (n*c*math.sin(lon)), (n*(B/A)**2*s))
    normal = (c*math.cos(lon), c*math.sin(lon), s)
    return xyz, normal


def setup(site):
    lat, lon, alt = site
    lat, lon = math.radians(lat), math.radians(lon)
    cl, sl, co, so = math.cos(lat), math.sin(lat), math.cos(lon), math.sin(lon)
    up = (cl*co, cl*so, sl)
    east = (-so, co, 0.0)
    north = (-sl*co, -sl*so, cl)
    p, _ = surface(math.degrees(lon), math.degrees(lat))
    observer = tuple(p[i] + alt*up[i] for i in range(3))
    camera = tuple(observer[i] + HEIGHT*up[i] for i in range(3))
    return observer, camera, east, north, up


def dot(a, b):
    return sum(x*y for x, y in zip(a, b))


def sample(parts, step):
    result, max_geo, dateline, segments = [], 0.0, 0, 0
    for part in parts:
        points = [part[0]]
        for start, end in zip(part, part[1:]):
            lon0, lat0 = start
            lon1, lat1 = end
            delta = lon1 - lon0
            if abs(delta) > 180:
                dateline += 1
                delta -= math.copysign(360, delta)
            length = math.hypot(delta, lat1 - lat0)
            max_geo = max(max_geo, length)
            n = max(1, math.ceil(length / step))
            segments += n
            points.extend((lon0 + delta*i/n, lat0 + (lat1-lat0)*i/n) for i in range(1, n + 1))
        result.append(points)
    return result, segments, max_geo, dateline


def workload(parts, site, fov):
    observer, camera, east, north, up = setup(site)
    span = math.tan(math.radians(fov)/2)
    aspect = 173/117
    inspected = front = endpoint_in = both_front_box = 0
    for part in parts:
        prev = None
        for lon, lat in part:
            pos, normal = surface(lon, lat)
            visible = dot(normal, tuple(camera[i] - pos[i] for i in range(3))) >= 0
            delta = tuple(pos[i] - observer[i] for i in range(3))
            depth = HEIGHT - dot(delta, up)
            x = dot(delta, east) / depth / span / aspect if depth >= 1 else float("inf")
            y = dot(delta, north) / depth / span if depth >= 1 else float("inf")
            current = (visible, x, y)
            if prev is not None:
                inspected += 1
                v0, x0, y0 = prev
                if v0 or visible:
                    front += 1
                if (v0 and abs(x0) <= 1 and abs(y0) <= 1) or (visible and abs(x) <= 1 and abs(y) <= 1):
                    endpoint_in += 1
                if (v0 and visible and min(x0, x) <= 1 and max(x0, x) >= -1
                        and min(y0, y) <= 1 and max(y0, y) >= -1):
                    both_front_box += 1  # Only a coarse candidate count, not a line clip.
            prev = current
    return inspected, front, endpoint_in, both_front_box


def trial_bytes(parts):
    """Candidate layout: i32 lon/lat at 1e-4 deg, followed by u32 offsets.

    No header, spatial index or runtime decoder; only an exact byte-count trial.
    """
    data = bytearray()
    offsets = [0]
    max_error = 0.0
    for part in parts:
        for lon, lat in part:
            values = tuple(round(v * 10_000) for v in (lon, lat))
            data.extend(struct.pack("<ii", *values))
            max_error = max(max_error, *(abs(v - q / 10_000) for v, q in zip((lon, lat), values)))
        offsets.append(offsets[-1] + len(part))
    data.extend(struct.pack(f"<{len(offsets)}I", *offsets))
    assert len(data) == 8 * offsets[-1] + 4 * len(offsets)
    assert offsets[0] == 0 and all(a < b for a, b in zip(offsets, offsets[1:]))
    return bytes(data), max_error


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    from pathlib import Path
    parser.add_argument("archive", type=Path)
    parser.add_argument("--out-dir", type=Path, help="optional temporary raw/sample byte-count trial artifacts")
    args = parser.parse_args()
    records, parts, overlong = load(args.archive)
    raw_points = sum(map(len, parts))
    raw_segments = sum(len(part)-1 for part in parts)
    print(f"sha256={EXPECTED_SHA256} version=4.1.0 records={records} parts={len(parts)} vertices={raw_points} segments={raw_segments}")
    print(f"source closed parts={sum(p[0] == p[-1] for p in parts)}; longest part={max(map(len, parts))} vertices")
    sampled, total, max_geo, dateline = sample(parts, .2)
    count = sum(map(len, sampled))
    print(f"source lon slightly >180 normalized to 180={overlong}; raw max geographic endpoint span={max_geo:.3f} deg; raw shortest-lon antimeridian jumps={dateline}")
    print(f"0.2-deg geographic interpolation: parts={len(sampled)} vertices={count} segments={total}; longest part={max(map(len, sampled))} vertices")
    print(f"hypothetical full int32 lon/lat @ 1e-4 degree + u32 part offsets: raw={8*raw_points+4*(len(parts)+1)} bytes; 0.2-deg={8*count+4*(len(parts)+1)} bytes (no headers/alignment/index)")
    if args.out_dir:
        args.out_dir.mkdir(parents=True, exist_ok=True)
        for label, data_parts in (("raw", parts), ("sample-0.2", sampled)):
            data, max_error = trial_bytes(data_parts)
            path = args.out_dir / f"coastline-trial-{label}.bin"
            path.write_bytes(data)
            assert path.stat().st_size == len(data)
            print(f"temporary {label}: {len(data)} bytes; sha256={hashlib.sha256(data).hexdigest()}; max per-coordinate quantization error={max_error:.8f} deg")
    print("workload: inspected / >=1 front-facing endpoint / >=1 front-facing in-viewport endpoint / both-front coarse bounding-box candidates")
    for name, site in SITES.items():
        for fov in FOVS:
            print(f"{name:17} {fov:5.1f} deg: " + " / ".join(map(str, workload(sampled, site, fov))))
    print("All visibility counts are endpoint proxies, NOT correct tangent/viewport clipping or timings. Inspected is the exhaustive per-frame scan bound for this candidate.")


if __name__ == "__main__":
    main()
