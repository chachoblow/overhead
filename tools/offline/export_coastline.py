#!/usr/bin/env python3
"""Export the 40 km M4 coastline comparison asset from the pinned Natural Earth zip.

Usage: python3 tools/offline/export_coastline.py /path/to/ne_110m_coastline.zip
Requires only Python stdlib; does not install a package or fetch data.
"""
import argparse
import hashlib
import math
import struct
from pathlib import Path

from coastline_compare import reduce_part
from coastline_preflight import load, trial_bytes


def export(archive: Path, destination: Path) -> None:
    records, parts, _ = load(archive)  # checks archive hash, version, WGS-84, geometry
    if (records, len(parts), sum(map(len, parts))) != (134, 134, 5128):
        raise ValueError("pinned source topology changed")
    reduced = [reduce_part(part, 40) for part in parts]
    if len(reduced) != 134 or sum(map(len, reduced)) != 1868:
        raise ValueError("40 km reduction did not reproduce the reviewed topology")
    # The checked part-offset format is exactly the offline trial, not a runtime
    # shapefile parser. Reject changes to size rather than truncating geography.
    data, error = trial_bytes(reduced)
    if len(data) != 15484 or error > 0.00005 + 1e-10:
        raise ValueError("unexpected export size or coordinate error")
    decoded = [tuple(v / 10_000 for v in struct.unpack_from("<ii", data, i * 8))
               for i in range(1868)]
    offsets = struct.unpack_from("<135I", data, 1868 * 8)
    if offsets[0] != 0 or offsets[-1] != 1868 or any(a >= b for a, b in zip(offsets, offsets[1:])):
        raise ValueError("invalid exported part offsets")
    steps = []
    for start, end in zip(offsets, offsets[1:]):
        for a, b in zip(decoded[start:end], decoded[start + 1:end]):
            delta = b[0] - a[0]
            if abs(delta) > 180:
                delta -= math.copysign(360, delta)
            steps.append(max(1, math.ceil(math.hypot(delta, b[1] - a[1]) / 0.2)))
    if (len(steps), sum(steps), max(steps)) != (1734, 23377, 108):
        raise ValueError("decoded runtime work differs from pinned comparison")
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(data)
    print(f"{destination}: {len(data)} bytes, sha256={hashlib.sha256(data).hexdigest()}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    parser.add_argument("--out", type=Path, default=Path("render/assets/coastline-110m-40km.bin"))
    args = parser.parse_args()
    export(args.archive, args.out)
