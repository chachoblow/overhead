#!/usr/bin/env python3
"""Offline 40km coastline sampling convergence study; no renderer or export rule.

Requires host Pillow for 1-bit mask comparisons; uses pinned archive validator.
"""
import argparse
import json
import math
from pathlib import Path

from coastline_compare import draw, interpolate, pixels, reduce_part, segment_might_enter
from coastline_preflight import HEIGHT, SITES, dot, load, setup, surface

SITES_BY_KEY = {'co': SITES['Colorado'], 'sf': SITES['San Francisco'],
                'polar': SITES['north polar'], 'dateline': SITES['equator/dateline']}
FOVS = {'co': (0.1, 1, 5), 'sf': (0.1, 1, 5),
        'polar': (0.1, 5), 'dateline': (0.1, 5)}
STEPS = (0.2, 0.05, 0.01)


def curve_deviation(parts, site, fov):
    """Max on-screen point-to-coarse-chord deviation from 20 subdivisions.

    Only intervals whose coarse endpoints and fine point face camera are
    counted. Crossing/tangent intervals are separately counted, not certified.
    """
    observer, camera, east, north, up = setup(site)
    half = math.tan(math.radians(fov)/2)

    def front_project(p):
        xyz, normal = surface(*p)
        facing = dot(normal, tuple(camera[i] - xyz[i] for i in range(3))) > 0
        d = tuple(xyz[i] - observer[i] for i in range(3))
        depth = HEIGHT - dot(d, up)
        if depth < 1:
            raise ValueError('invalid Earth view')
        x = dot(d, east)/depth/half/(173/117)
        y = dot(d, north)/depth/half
        return facing, (x, y)

    def distance(p, a, b):
        dx, dy = b[0]-a[0], b[1]-a[1]
        t = max(0., min(1., ((p[0]-a[0])*dx+(p[1]-a[1])*dy)/(dx*dx+dy*dy))) if dx*dx+dy*dy else 0.
        return math.hypot(173*(p[0]-a[0]-t*dx), 117*(p[1]-a[1]-t*dy))

    stats = dict(coarse_intervals=0, visible_fine_points=0, not_both_front_intervals=0,
                 max_chord_distance_px=None, over_half_pixel=0, over_one_pixel=0)
    for part in parts:
        for a, b in zip(part, part[1:]):
            if not segment_might_enter(a, b, observer, east, north, up, half):
                continue
            dl = b[0]-a[0]
            if abs(dl) > 180:
                dl -= math.copysign(360, dl)
            n = max(1, math.ceil(math.hypot(dl, b[1]-a[1])/.2))
            for i in range(n):
                u, v = interpolate(a, b, i/n), interpolate(a, b, (i+1)/n)
                fu, pu = front_project(u)
                fv, pv = front_project(v)
                stats['coarse_intervals'] += 1
                if not (fu and fv):
                    stats['not_both_front_intervals'] += 1
                    continue
                for j in range(1, 20):
                    facing, p = front_project(interpolate(u, v, j/20))
                    if not facing or abs(p[0]) > 1 or abs(p[1]) > 1:
                        continue
                    err = distance(p, pu, pv)
                    stats['visible_fine_points'] += 1
                    old = stats['max_chord_distance_px']
                    stats['max_chord_distance_px'] = max(old if old is not None else 0., err)
                    stats['over_half_pixel'] += err > .5
                    stats['over_one_pixel'] += err > 1
    return stats


def study(archive, output, frames=None):
    from PIL import Image, ImageDraw
    _, source, _ = load(archive)
    parts = [reduce_part(p, 40) for p in source]
    output.mkdir(parents=True, exist_ok=True)
    report = {'stored_vertices': sum(map(len, parts)), 'scenes': {}}
    for site, fovs in FOVS.items():
        for fov in fovs:
            key = f'{site}-{fov:g}'
            masks = {}
            stats = {}
            for step in STEPS:
                layer, sample = draw(parts, SITES_BY_KEY[site], fov, cull=True, sample_step=step)
                masks[step] = layer
                stats[f'{step:g}'] = {k: sample[k] for k in ('surface_intervals', 'front_tests', 'tangents', 'coast_ink')}
            def difference(a, b):
                return sum(x != y for x, y in zip(pixels(masks[a]), pixels(masks[b])))
            report['scenes'][key] = {'steps': stats,
                                     'diff_0.2_vs_0.05': difference(.2, .05),
                                     'diff_0.05_vs_0.01': difference(.05, .01),
                                     'diff_0.2_vs_0.01': difference(.2, .01),
                                     'curve': curve_deviation(parts, SITES_BY_KEY[site], fov)}
            if frames and site == 'sf':
                source_image = Image.open(frames/f'{key}.png').convert('L').point(lambda c: 255 if c > 127 else 0).convert('1')
                for step in STEPS:
                    # Source ink preserved, but not executable layering.
                    new = Image.new('1', (400, 240), 1)
                    new.putdata([0 if a == 0 or b == 0 else 255 for a, b in zip(pixels(source_image), pixels(masks[step]))])
                    new.save(output/f'{key}-{step:g}.png')
    controls = {
        'two_offscreen_endpoints_edge': ([ [(-2., 0.), (2., 0.)] ], (0., 0., 0.), .1),
        'near_surface_tangent': ([ [(84.8, .02), (86.2, .02)] ], (0., 0., 0.), 20.),
    }
    report['controls'] = {}
    for name, (part, site, fov) in controls.items():
        rendered = [draw(part, site, fov, cull=True, sample_step=step) for step in STEPS]
        ref = list(pixels(rendered[-1][0]))
        report['controls'][name] = {
            'intervals': [s['surface_intervals'] for _, s in rendered],
            'tangents': [s['tangents'] for _, s in rendered],
            'coast_ink': [s['coast_ink'] for _, s in rendered],
            'diff_0.2_vs_0.01': sum(a != b for a, b in zip(pixels(rendered[0][0]), ref)),
        }
    if frames:
        fovs = FOVS['sf']
        sheet = Image.new('1', (1600, len(fovs)*270+32), 1)
        pen = ImageDraw.Draw(sheet)
        for col, title in enumerate(('SOURCE', '0.2 DEG', '0.05 DEG', '0.01 DEG')):
            pen.text((400*col+6, 8), title, fill=0)
        for row, fov in enumerate(fovs):
            key = f'sf-{fov:g}'
            images = [Image.open(frames/f'{key}.png').convert('L').point(lambda c: 255 if c > 127 else 0).convert('1')]
            images.extend(Image.open(output/f'{key}-{step:g}.png') for step in STEPS)
            for col, im in enumerate(images):
                sheet.paste(im, (400*col, 32+270*row))
            pen.text((6, 32+270*row+242), f'SF {fov:g} deg', fill=0)
        sheet.save(output/'sf-sheet.png')
    (output/'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--frames', type=Path, help='optional fresh 0056 simulator source PNG directory')
    args = parser.parse_args()
    study(args.archive, args.output, args.frames)


if __name__ == '__main__':
    main()
