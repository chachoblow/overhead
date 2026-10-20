#!/usr/bin/env python3
"""Offline 40km trial encoding/culling diagnostic; NOT a renderer contract.

Reuse pinned input and fresh headless simulator sources from the 0056 study.
Requires host Pillow as in coastline_compare.py; no new Cargo dependency.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct

from coastline_compare import draw, pixels, reduce_part
from coastline_preflight import SITES, load, trial_bytes

SITES_BY_KEY = {'co': SITES['Colorado'], 'sf': SITES['San Francisco'],
                'dateline': SITES['equator/dateline'], 'polar': SITES['north polar']}
FOVS = {'co': (0.1, 1, 5, 20, 100), 'sf': (0.1, 1, 5, 20, 100),
        'dateline': (0.1, 1, 5, 20, 100), 'polar': (0.1, 1, 5, 20, 100)}


def decode_trial(data, n_parts):
    """Read back the unapproved i32 lon/lat trial using its fixed part count."""
    if len(data) < 4 * (n_parts + 1) or (len(data)-4*(n_parts+1)) % 8:
        raise ValueError('invalid trial length')
    n = (len(data)-4*(n_parts+1))//8
    offsets = struct.unpack_from(f'<{n_parts+1}I', data, 8*n)
    if offsets[0] != 0 or offsets[-1] != n or any(a >= b for a, b in zip(offsets, offsets[1:])):
        raise ValueError('invalid part offsets')
    points = [tuple(v / 10_000 for v in struct.unpack_from('<ii', data, 8*i)) for i in range(n)]
    return [points[a:b] for a, b in zip(offsets, offsets[1:])]


def compare(archive, out):
    from PIL import Image
    _, parts, _ = load(archive)
    float_parts = [reduce_part(p, 40) for p in parts]
    binary, max_geo_error = trial_bytes(float_parts)
    quant_parts = decode_trial(binary, len(float_parts))
    assert trial_bytes(quant_parts)[0] == binary
    out.mkdir(parents=True, exist_ok=True)
    report = dict(sha256=hashlib.sha256(binary).hexdigest(),
                  stored_bytes=len(binary), vertices=sum(map(len, float_parts)),
                  max_coordinate_error_deg=max_geo_error, scenes={})
    for site, fovs in FOVS.items():
        for fov in fovs:
            key = f'{site}-{fov:g}'
            reference, full = draw(float_parts, SITES_BY_KEY[site], fov)
            culled, fast = draw(float_parts, SITES_BY_KEY[site], fov, cull=True)
            encoded_full, _ = draw(quant_parts, SITES_BY_KEY[site], fov)
            encoded, q = draw(quant_parts, SITES_BY_KEY[site], fov, cull=True)
            reference_pixels = list(pixels(reference))
            if reference_pixels != list(pixels(culled)):
                raise AssertionError(f'coarse cull changed float pixels: {key}')
            if list(pixels(encoded_full)) != list(pixels(encoded)):
                raise AssertionError(f'coarse cull changed encoded pixels: {key}')
            diff = sum(a != b for a, b in zip(reference_pixels, pixels(encoded)))
            stats = {'source_segments': fast['coarse_segments'],
                     'admitted_segments': fast['coarse_kept'],
                     'full_intervals': full['surface_intervals'],
                     'admitted_intervals': fast['surface_intervals'],
                     'full_front_tests': full['front_tests'],
                     'admitted_front_tests': fast['front_tests'],
                     'full_tangents': full['tangents'],
                     'culled_float_tangents': fast['tangents'],
                     'culled_encoded_tangents': q['tangents'],
                     'float_ink': full['coast_ink'],
                     'encoded_ink': q['coast_ink'],
                     'encoded_pixel_differences': diff}
            report['scenes'][key] = stats
            if site == 'sf' and fov in (0.1, 1, 5):
                encoded.save(out/f'{key}-40km-encoded-coast.png')
                # Exact native-size cells. Both coast columns are offline masks.
                sheet = Image.new('1', (800, 270), 1)
                sheet.paste(reference, (0, 30))
                sheet.paste(encoded, (400, 30))
                from PIL import ImageDraw
                pen = ImageDraw.Draw(sheet)
                pen.text((6, 8), f'FLOAT / {key}', fill=0)
                pen.text((406, 8), f'ENCODED / {key}', fill=0)
                sheet.save(out/f'{key}-float-vs-encoded.png')
    (out/'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    compare(args.archive, args.output)


if __name__ == '__main__':
    main()
