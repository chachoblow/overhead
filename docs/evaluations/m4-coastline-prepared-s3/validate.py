#!/usr/bin/env python3
"""Offline audit of the prepared-layer S3 captures; no hardware.

Checks completion, the full 11-case x 6-mode x 5-sample matrix, on-target
exactness markers, zero-heap header, and that every non-Prepare checksum and
write count equals the host oracle. Prints per-mode timing/stack summaries.
"""
import csv, json, statistics, sys
from pathlib import Path

HERE = Path(__file__).parent
MODES = ["Prepare", "CoastRaster", "CoastCached", "FrameRaster", "FrameCached", "FrameOff"]
CASES = ["sf-tight", "sf-1", "sf-5", "sf-100", "colorado-20", "polar-20", "dateline-5",
         "limb-20", "limb-100", "viewport-tight", "iss-ground-20"]


def parse(path):
    lines = [l.strip("\r") for l in path.read_bytes().decode("utf-8", "replace").split("\n")]
    assert "OVERHEAD_S3_COAST_PREPARED_V1" in lines, path
    assert "OVERHEAD_S3_COAST_PREPARED_DONE" in lines, path
    assert not any("_FAILED" in l for l in lines), path
    assert any(l.startswith("cpu_mhz=240,") and "samples=5" in l for l in lines)
    rows, exact = {}, set()
    for l in lines:
        p = l.split(",")
        if p[0] == "ROW":
            key = (p[1], p[2], int(p[3]))
            assert key not in rows, key
            rows[key] = tuple(int(x) for x in p[4:9])
        elif p[0] == "EXACT":
            exact.add((p[1], p[2]))
    assert set(rows) == {(c, m, i) for c in CASES for m in MODES for i in range(5)}, path
    assert exact == {(c, m) for c in CASES for m in MODES}, path
    return rows


def main():
    host = {(r["case"], r["mode"]): (int(r["checksum"]), int(r["writes"]), int(r["on_writes"]))
            for r in csv.DictReader((HERE / "host-expected.csv").open())}
    runs = [parse(HERE / f"capture-on-{n}.log") for n in (1, 2)]
    for rows in runs:
        for (c, m, i), (_, checksum, writes, on, _) in rows.items():
            if m != "Prepare":
                assert (checksum, writes, on) == host[(c, m)], (c, m, i)
    summary = {}
    for m in MODES:
        us = [r[(c, m, i)][0] for r in runs for c in CASES for i in range(5)]
        stack = [r[(c, m, i)][4] for r in runs for c in CASES for i in range(5)]
        per_case = {c: max(r[(c, m, i)][0] for r in runs for i in range(5)) for c in CASES}
        summary[m] = {"min_us": min(us), "median_us": statistics.median(us), "max_us": max(us),
                      "max_stack_bytes": max(stack), "max_us_by_case": per_case}
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    sys.exit(main())
