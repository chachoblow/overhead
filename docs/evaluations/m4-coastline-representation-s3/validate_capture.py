#!/usr/bin/env python3
"""Offline audit of the single representation feasibility capture. No hardware.

Require the exact ordered scene/mode/sample matrix, complete framebuffer dumps,
repeatability, host-byte parity, off/on FrameOff parity, and rollback controls.
Timings/observed stack writes are measurements, not universal bounds.
"""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import re
import statistics
import zipfile

HERE = Path(__file__).resolve().parent
CASES = ["sf-tight", "sf-1", "sf-5", "sf-100", "colorado-20", "polar-20",
         "dateline-5", "limb-20", "limb-100", "viewport-tight", "iss-ground-20"]


def modes(name, on):
    if not on:
        return ["FrameOff"]
    return ["Coast", "FrameOn", "FrameOff"] + (
        ["Late", "LateWarning"] if name in ["polar-20", "iss-ground-20"] else [])


def checksum(data):
    h = 0
    for byte in data:
        h = (31 * h + byte) & 0xffffffff
    return h


def parse(raw, on):
    text = re.sub(r"\x1b\[[0-9;]*[a-zA-Z]", "", raw.decode("utf-8", errors="replace"))
    lines = text.splitlines()
    assert "FAILED" not in text and "panicked" not in text
    start = "OVERHEAD_S3_COAST_REPRESENTATION_V1"
    done = "OVERHEAD_S3_COAST_REPRESENTATION_DONE"
    assert lines.count(start) == lines.count(done) == 1
    lines = lines[lines.index(start):lines.index(done) + 1]
    assert f"coast_on={str(on).lower()},cpu_mhz=240,buffer_bytes=12000,heap_reserved=65536,samples=5" in lines
    assert "pending_bytes=640" in lines
    assert "Every ROW asserts zero allocation calls/failures/requested/live/peak growth" in lines
    probes = [line for line in lines if line.startswith("PROBE,")]
    assert len(probes) == 1
    probe = {k: int(v) for k, v in (item.split("=") for item in probes[0].split(",")[1:])}
    assert probe["loaded"] >= probe["baseline"] + 4096
    assert probe["stack_low"] < probe["paint_low"] < probe["paint_high"] < probe["stack_top"]
    cases = [line.split(",") for line in lines if line.startswith("CASE,")]
    assert [c[1] for c in cases] == CASES
    sites = [(37.7749, -122.4194, fov) for fov in [0.1, 1, 5, 100]] + [
        (39.007, -104.883, 20), (89.9, 0, 20), (0, 179.9, 5),
        (1.364900, -156.229044, 20), (1.364900, -156.229044, 100),
        (1.364900, 119.225793, 0.1), (0.000017, -85.180357, 20)]
    for c, site in zip(cases, sites):
        assert len(c) == 6
        assert all(abs(float(actual) - expected) < 0.000001 for actual, expected in zip(c[2:5], site))
        assert c[-1] == f"selected={str(c[1] == 'iss-ground-20').lower()}"
    expected_keys = [(name, mode, i) for name in CASES for mode in modes(name, on) for i in range(5)]
    rows = []
    for line in lines:
        if line.startswith("ROW,"):
            _, name, mode, sample, us, check, writes, ink, stack = line.split(",")
            row = dict(case=name, mode=mode, sample=int(sample), us=int(us), checksum=int(check),
                       writes=int(writes), on_writes=int(ink), stack=int(stack))
            assert 0 < row["us"] and 0 < row["on_writes"] <= row["writes"]
            assert 0 < row["stack"] < probe["stack_top"] - probe["stack_low"] - 256
            rows.append(row)
    assert [(r["case"], r["mode"], r["sample"]) for r in rows] == expected_keys
    groups = {(name, mode): [r for r in rows if (r["case"], r["mode"]) == (name, mode)]
              for name in CASES for mode in modes(name, on)}
    for key, group in groups.items():
        for field in ["checksum", "writes", "on_writes", "stack"]:
            assert len({row[field] for row in group}) == 1, (key, field)
    mask_lines = [line for line in lines if line.startswith("MASK,")]
    assert len(mask_lines) == len(groups) * 240
    masks = {}
    iterator = iter(mask_lines)
    for name, mode in groups:
        pixels = bytearray()
        for y in range(240):
            prefix, case, kind, line_y, hexdata = next(iterator).split(",")
            assert (prefix, case, kind, line_y) == ("MASK", name, mode, str(y))
            assert re.fullmatch(r"[0-9a-f]{100}", hexdata)
            pixels.extend(bytes.fromhex(hexdata))
        pixels = bytes(pixels)
        assert checksum(pixels) == groups[name, mode][0]["checksum"]
        masks[name, mode] = pixels
    controls = [line.split(",") for line in lines if line.startswith("OFF_CONTROL,")]
    assert [c[1] for c in controls] == CASES
    assert all(int(c[2]) == checksum(masks[c[1], "FrameOff"]) for c in controls)
    late = [line.split(",") for line in lines if line.startswith("PIXEL_CONTROL,")]
    expected_late = [(name, mode) for name in CASES for mode in modes(name, on) if mode.startswith("Late")]
    assert [(c[1], c[2]) for c in late] == expected_late
    for _, name, mode, status, check in late:
        assert status == "matched" and int(check) == checksum(masks[name, mode])
    return rows, masks, probe


def audit(directory):
    expected = list(csv.DictReader((directory / "host-expected.csv").read_text().splitlines()))
    assert [(r["case"], r["mode"]) for r in expected] == [
        (name, mode) for name in CASES for mode in modes(name, True)]
    expected = {(r["case"], r["mode"]): r for r in expected}
    with zipfile.ZipFile(directory / "host-masks.zip") as archive:
        assert sorted(archive.namelist()) == sorted(f"{n}-{m}.bin" for n, m in expected)
        host = {key: archive.read(f"{key[0]}-{key[1]}.bin") for key in expected}
    for key, data in host.items():
        assert len(data) == 12000 and checksum(data) == int(expected[key]["checksum"])
    summary = {}
    captures = {}
    for side in ["off", "on"]:
        raw = (directory / f"capture-{side}.log").read_bytes()
        rows, masks, probe = parse(raw, side == "on")
        for key, data in masks.items():
            assert data == host[key], f"TARGET PIXEL DIFFERENCE: {side}/{key}"
        for row in rows:
            key = row["case"], row["mode"]
            for field in ["checksum", "writes", "on_writes"]:
                assert row[field] == int(expected[key][field]), (side, key, field)
        captures[side] = rows, masks
        summary[side] = dict(rows=len(rows), masks=len(masks), raw_sha256=hashlib.sha256(raw).hexdigest(),
                             probe=probe, modes={})
        for mode in modes("polar-20", side == "on"):
            selected = [r for r in rows if r["mode"] == mode]
            maximum = max(selected, key=lambda r: r["us"])
            summary[side]["modes"][mode] = dict(min_us=min(r["us"] for r in selected),
                max_us=maximum["us"], max_case=maximum["case"],
                stack_min=min(r["stack"] for r in selected), stack_max=max(r["stack"] for r in selected))
    for name in CASES:
        assert captures["off"][1][name, "FrameOff"] == captures["on"][1][name, "FrameOff"]
    summary["coast_cases"] = {}
    rows = captures["on"][0]
    for name in CASES:
        coast = [r for r in rows if r["case"] == name and r["mode"] == "Coast"]
        summary["coast_cases"][name] = dict(median_us=statistics.median(r["us"] for r in coast),
                                            max_us=max(r["us"] for r in coast))
    summary["exact_host_masks"] = sum(len(x[1]) for x in captures.values())
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", nargs="?", type=Path, default=HERE)
    args = parser.parse_args()
    print(json.dumps(audit(args.directory), indent=2))


if __name__ == "__main__":
    main()
