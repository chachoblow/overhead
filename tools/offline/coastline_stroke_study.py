#!/usr/bin/env python3
"""Offline pixel-center clipped-stroke experiment; NOT a coastline renderer.

The stroke is the union of radius-0.5px capsules around projected, clipped
segments. Pixels are tested at integer centers, so splitting a straight segment
cannot repeatedly round its endpoints or add short-stroke raster ink. A curved
surface polyline can still change pixels when subdivided. Pillow is used only
for the existing reference raster and optional 1-bit review sheet.
"""
import argparse
import json
import math
from pathlib import Path
import struct

from coastline_compare import draw, pixels, reduce_part
from coastline_preflight import SITES, load

WIDTH, HEIGHT = 400, 240
# The existing wide radar's drawable rectangle, NOT the whole display/strip.
BOUNDS = (3, 349, 3, 237)


def distance_squared(x, y, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    length = dx * dx + dy * dy
    t = max(0., min(1., ((x-a[0])*dx + (y-a[1])*dy)/length)) if length else 0.
    return (x-a[0]-t*dx)**2 + (y-a[1]-t*dy)**2


def f32(v):
    return struct.unpack('<f', struct.pack('<f', v))[0]


def candidate_bounds(a, b):
    """Clamped pixel-center candidate box for a clipped screen segment."""
    if not all(map(math.isfinite, (*a, *b))):
        raise ValueError('nonfinite screen segment')
    return (max(BOUNDS[0], math.ceil(min(a[0], b[0]) - .5)),
            min(BOUNDS[1], math.floor(max(a[0], b[0]) + .5)),
            max(BOUNDS[2], math.ceil(min(a[1], b[1]) - .5)),
            min(BOUNDS[3], math.floor(max(a[1], b[1]) + .5)))


def candidate_count(a, b):
    xmin, xmax, ymin, ymax = candidate_bounds(a, b)
    return max(0, xmax-xmin+1) * max(0, ymax-ymin+1)


def stroke(segments, quantize=False):
    """Binary mask of continuous clipped strokes; f32 only rounds screen endpoints.

    Endpoint rounding is a sensitivity probe, NOT f32 camera/ECEF emulation.
    Bounding boxes are clipped before integer conversion; no off-radar ink.
    """
    from PIL import Image
    mask = bytearray([255]) * (WIDTH * HEIGHT)
    candidates = 0
    for a, b in segments:
        if quantize:
            a, b = tuple(map(f32, a)), tuple(map(f32, b))
        xmin, xmax, ymin, ymax = candidate_bounds(a, b)
        for y in range(ymin, ymax + 1):
            for x in range(xmin, xmax + 1):
                candidates += 1
                if distance_squared(x, y, a, b) <= .25:
                    mask[y * WIDTH + x] = 0
    image = Image.frombytes('L', (WIDTH, HEIGHT), bytes(mask)).convert('1')
    return image, candidates


def difference(a, b):
    # Pillow's freshly created mode-1 image may expose white as 1 or 255;
    # decoded PNGs expose 255. Compare ink, not the white byte encoding.
    return sum((x == 0) != (y == 0) for x, y in zip(pixels(a), pixels(b)))


def ink(image):
    return sum(value == 0 for value in pixels(image))


def scene(parts, site, fov, steps=(.2, .05, .01)):
    masks, old, geometry = {}, {}, {}
    results = {}
    for step in steps:
        segments = []
        reference, stats = draw(parts, site, fov, cull=True, sample_step=step,
                                segments_out=segments)
        stable, candidates = stroke(segments)
        rounded, _ = stroke(segments, quantize=True)
        masks[step] = stable
        old[step] = reference
        geometry[step] = segments
        results[f'{step:g}'] = dict(intervals=stats['surface_intervals'],
                                    tangents=stats['tangents'],
                                    interior_front=stats['interior_front'],
                                    clipped_segments=stats['clipped_segments'],
                                    pixel_candidates=candidates,
                                    ink=ink(stable),
                                    old_ink=ink(reference),
                                    old_vs_stroke=difference(reference, stable),
                                    f32_screen_endpoint_vs_f64=difference(rounded, stable))
    a, b, c = steps
    changed = []
    for index, (coarse, fine) in enumerate(zip(pixels(masks[a]), pixels(masks[c]))):
        if (coarse == 0) == (fine == 0):
            continue
        x, y = index % WIDTH, index // WIDTH
        def nearest(segments):
            return min((math.sqrt(distance_squared(x, y, start, end)) for start, end in segments),
                       default=None)
        changed.append(dict(x=x, y=y, coarse_ink=coarse == 0,
                            coarse_distance_px=nearest(geometry[a]),
                            fine_distance_px=nearest(geometry[c])))
    return masks, old, dict(steps=results, changed_pixels=changed,
                            old_coarse_vs_fine=difference(old[a], old[c]),
                            stroke_coarse_vs_fine=difference(masks[a], masks[c]),
                            stroke_middle_vs_fine=difference(masks[b], masks[c]))


def study(archive, output, frames=None):
    from PIL import Image, ImageDraw
    _, source, _ = load(archive)  # pinned hash/version/shape validation
    parts = [reduce_part(part, 40) for part in source]  # study only
    cases = {
        'sf-0.1': (parts, SITES['San Francisco'], .1),
        'sf-1': (parts, SITES['San Francisco'], 1.),
        'sf-5': (parts, SITES['San Francisco'], 5.),
        'edge-crossing': ([[(-2., 0.), (2., 0.)]], (0., 0., 0.), .1),
        'near-tangent': ([[ (84.8, .02), (86.2, .02) ]], (0., 0., 0.), 20.),
        'hidden-endpoints-visible-middle': ([[(-89., 0.), (89., 0.)]], (0., 0., 0.), 5.),
    }
    output.mkdir(parents=True, exist_ok=True)
    report = {'source': 'pinned Natural Earth v4.1.0; unquantized 40 km ECEF study input',
              'stroke': 'integer pixel center <= 0.5 px from clipped screen segment',
              'cases': {}}
    row_images = []
    for name, (case_parts, site, fov) in cases.items():
        masks, old, result = scene(case_parts, site, fov)
        report['cases'][name] = result
        if name.startswith('sf-'):
            # Raw masks are reusable independent of the finished-frame OR overlay.
            for key, image in (('old-0.2', old[.2]), ('stroke-0.2', masks[.2]),
                               ('stroke-0.01', masks[.01])):
                image.save(output/f'{name}-{key}.png')
            if frames is not None:
                source_image = Image.open(frames/f'{name}.png').convert('1')
                if source_image.size != (WIDTH, HEIGHT):
                    raise ValueError('source frame size mismatch')
                def composite(mask):
                    image = Image.new('1', (WIDTH, HEIGHT), 1)
                    image.putdata([0 if s == 0 or m == 0 else 255
                                   for s, m in zip(pixels(source_image), pixels(mask))])
                    return image
                row_images.append((name, (source_image, composite(old[.2]),
                                          composite(masks[.2]), composite(masks[.01]))))
    if row_images:
        sheet = Image.new('1', (1600, 842), 1)
        pen = ImageDraw.Draw(sheet)
        for col, label in enumerate(('SOURCE', '0057 0.2 DEG', 'STROKE 0.2 DEG', 'STROKE 0.01 DEG')):
            pen.text((col * WIDTH + 6, 8), label, fill=0)
        for row, (name, images) in enumerate(row_images):
            for col, image in enumerate(images):
                sheet.paste(image, (col * WIDTH, 32 + row * 270))
            pen.text((6, 32 + row * 270 + 242), name, fill=0)
        sheet.save(output/'sf-sheet.png')
    (output/'report.json').write_text(json.dumps(report, indent=2) + '\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--frames', type=Path, help='optional 0057 headless no-coast source frames')
    args = parser.parse_args()
    print(json.dumps(study(args.archive, args.output, args.frames), indent=2))


if __name__ == '__main__':
    main()
