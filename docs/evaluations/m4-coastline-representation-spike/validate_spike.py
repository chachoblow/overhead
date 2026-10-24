#!/usr/bin/env python3
"""Audit retained/reproduced host logs; optionally compose native 1-bit review sheets.

No target timing inference. Image composition alone needs the existing host Pillow;
the default log/asset audit uses only Python's standard library.
"""
import argparse
import csv
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
CASES = ["sf-tight", "sf-1", "sf-5", "sf-100", "colorado-20", "polar-20",
         "dateline-5", "limb-20", "limb-100", "viewport-tight", "iss-ground-20"]
ASSET_HASH = "951ed542a445201c24241bfb263b81567cef20641854be281778788a7a7a6796"


def fields(line):
    return dict(item.split("=", 1) for item in line.split(",")[1:])


def read_log(path):
    lines = path.read_text().splitlines()
    assert any("test result: ok. 1 passed; 0 failed" in line for line in lines), path
    representation = [fields(line) for line in lines if line.startswith("representation,")]
    sweeps = [fields(line) for line in lines if line.startswith("tight_sweep,")]
    assert len(representation) == len(sweeps) == 1
    rep, sweep = representation[0], sweeps[0]
    assert int(rep["bytes"]) == 540 + 134 * 9 + int(rep["pieces"]) * 15 == 32211
    assert int(rep["source_segments"]) == 1734
    assert float(rep["sampled_error_km"]) < 0.025 < float(rep["chord_error_km"])
    assert int(rep["pending_bytes"]) == 768  # host layout, NOT target scratch
    assert int(sweep["scenes"]) == int(sweep["visible"]) == 1810
    assert int(sweep["beyond1"]) == 0 and int(sweep["maximum"]) == 1
    assert not any(line.startswith("tight_difference,") for line in lines)
    headers = [line for line in lines if line.startswith("case,pieces,")]
    assert len(headers) == 1
    data = [line for line in lines if line.split(",")[0] in CASES]
    rows = list(csv.DictReader([headers[0], *data]))
    assert [row["case"] for row in rows] == CASES
    for row in rows:
        number = lambda key: int(row[key])
        assert number("pieces") == int(rep["pieces"]) == 2031
        assert number("transforms") == 3 * (number("pieces") - number("hidden"))
        assert number("nodes") < 65536 and number("pending") <= 16
        assert number("strokes") > 0 and number("on_writes") >= number("strokes")
        assert number("isolated_changed") > 0
        assert number("isolated_beyond1") == number("isolated_beyond3") == 0
        assert number("isolated_max") == number("whole_max") == 1
        assert 0 <= float(row["projected_error_px"]) < 0.02
        assert float(row["baseline_us"]) > 0 and float(row["candidate_us"]) > 0
    return rep, sweep, rows


def compose(directory):
    from PIL import Image, ImageChops, ImageDraw, ImageFont
    # These are coast-only executable TEST masks, not integrated simulator output.
    font = ImageFont.load_default()
    manifest = {}
    outputs = {}

    def source(name, suffix):
        path = directory / f"{name}-{suffix}.pgm"
        image = Image.open(path).convert("L")
        assert image.size == (400, 240) and set(image.tobytes()) <= {0, 255}
        manifest[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
        return image

    def sheet(names, filename, isolated=False):
        sheet = Image.new("L", (1232, 32 + len(names) * 266), 255)
        pen = ImageDraw.Draw(sheet)
        for x, text in [(8, "Existing renderer"), (416, "Quadratic trial"),
                        (824, "Changed pixels (black)")]:
            pen.text((x, 8), text, fill=0, font=font)
        for index, name in enumerate(names):
            y = 32 + index * 266
            suffix = "isolated-" if isolated else ""
            reference = source(name, suffix + "reference")
            candidate = source(name, suffix + "candidate")
            changed = ImageChops.invert(ImageChops.difference(reference, candidate))
            pen.text((8, y), name + (" / isolated worst segment" if isolated else " / coast only"),
                     fill=0, font=font)
            for x, image in [(8, reference), (416, candidate), (824, changed)]:
                sheet.paste(image, (x, y + 18))
        sheet = sheet.convert("1", dither=Image.Dither.NONE)
        outputs[filename] = sheet

    sheet(["sf-tight", "sf-5", "polar-20", "limb-20", "viewport-tight", "sf-100"],
          "native-review.png")
    sheet(["sf-1", "colorado-20", "dateline-5", "limb-100", "iss-ground-20"],
          "other-scenes.png")
    sheet(["sf-tight", "sf-5", "polar-20", "limb-20", "viewport-tight"],
          "isolated-review.png", isolated=True)
    trial = (directory / "quadratic-trial.bin").read_bytes()
    assert len(trial) == 32211
    manifest["quadratic-trial.bin"] = hashlib.sha256(trial).hexdigest()
    output = json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    path = HERE / "review-manifest.json"
    if path.exists():
        assert path.read_text() == output, "review input changed; investigate before replacing evidence"
    else:
        path.write_text(output)
    for filename, image in outputs.items():
        image.save(HERE / filename)
    print("Review source/packed-byte hashes:", manifest["quadratic-trial.bin"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("logs", nargs="*", type=Path)
    parser.add_argument("--images", type=Path, help="optional PGM/bin output from the Rust probe")
    args = parser.parse_args()
    assert hashlib.sha256((ROOT / "render/assets/coastline-110m-40km.bin").read_bytes()).hexdigest() == ASSET_HASH
    logs = args.logs or [HERE / "host-run1.txt", HERE / "host-run2.txt"]
    results = [read_log(path) for path in logs]
    first_rep, first_sweep, first_rows = results[0]
    for rep, sweep, rows in results[1:]:
        assert rep == first_rep and sweep == first_sweep
        for a, b in zip(first_rows, rows):
            assert {k: v for k, v in a.items() if not k.endswith("_us")} == {
                k: v for k, v in b.items() if not k.endswith("_us")}
    if args.images:
        compose(args.images)
    print(f"PASS: {len(logs)} log(s), 11 scenes, 1734 isolated segments/scene, 1810 tight controls.")
    print("Host-only finite-scene evidence; not S3 time, compiled size, stack safety or visual acceptance.")


if __name__ == "__main__":
    main()
