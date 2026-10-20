#!/usr/bin/env python3
"""Offline surface-coast reduction/clip *comparison*, not a renderer or asset generator.

Requires Pillow for host-only PNG composites (no Rust/Cargo dependency).
Uses the pinned archive validator from coastline_preflight.py. Output ink is
OR-composited with finished simulator frames: NOT executable draw order.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path

from coastline_preflight import HEIGHT, dot, load, setup, surface, trial_bytes


def rdp(points, tolerance_km):
    """3D ECEF chord-distance RDP; preserve original vertices at kept indices."""
    xyz = [surface(*p)[0] for p in points]
    keep = {0, len(points) - 1}
    pending = [(0, len(points) - 1)]
    while pending:
        a, b = pending.pop()
        if b - a < 2:
            continue
        start, end = xyz[a], xyz[b]
        delta = tuple(end[i] - start[i] for i in range(3))
        sq = dot(delta, delta)
        winner, best = None, -1.0
        for j in range(a + 1, b):
            at = tuple(xyz[j][i] - start[i] for i in range(3))
            t = max(0.0, min(1.0, dot(at, delta) / sq)) if sq else 0.0
            gap = sum((at[i] - t * delta[i]) ** 2 for i in range(3))
            if gap > best:
                winner, best = j, gap
        if best > tolerance_km**2:
            keep.add(winner)
            pending.extend(((a, winner), (winner, b)))
    return [points[i] for i in sorted(keep)]


def reduce_part(points, tolerance_km):
    if not tolerance_km or len(points) <= 2:
        return points[:]
    if points[0] == points[-1]:
        # Avoid degenerate closed RDP chord. Split at farthest surface point.
        xyz = [surface(*p)[0] for p in points]
        furthest = max(range(1, len(points)-1),
                       key=lambda i: sum((xyz[i][j] - xyz[0][j])**2 for j in range(3)))
        return rdp(points[:furthest+1], tolerance_km)[:-1] + rdp(points[furthest:], tolerance_km)
    return rdp(points, tolerance_km)


def interpolate(a, b, t):
    # Parts never cross the antimeridian in the pinned source. Explicit shortest
    # longitude handling is still needed when a future validated source does.
    dl = b[0] - a[0]
    if abs(dl) > 180:
        dl -= math.copysign(360, dl)
    return (a[0] + t * dl, a[1] + t * (b[1] - a[1]))


def pieces(parts, eligible=None, sample_step=.2):
    if not math.isfinite(sample_step) or sample_step <= 0:
        raise ValueError('sampling step must be finite and positive')
    for part in parts:
        for a, b in zip(part, part[1:]):
            if eligible is not None and not eligible(a, b):
                continue
            dl = b[0] - a[0]
            if abs(dl) > 180:
                dl -= math.copysign(360, dl)
            steps = max(1, math.ceil(math.hypot(dl, b[1] - a[1]) / sample_step))
            for i in range(steps):
                yield interpolate(a, b, i/steps), interpolate(a, b, (i+1)/steps)


def clip_box(a, b):
    """Liang–Barsky against normalized wide rectangle; float until final round."""
    ax, ay = a
    dx, dy = b[0]-ax, b[1]-ay
    lo, hi = 0.0, 1.0
    for p, q in ((-dx, ax+1), (dx, 1-ax), (-dy, ay+1), (dy, 1-ay)):
        if p == 0:
            if q < 0:
                return None
        else:
            t = q/p
            if p < 0:
                lo = max(lo, t)
            else:
                hi = min(hi, t)
            if lo > hi:
                return None
    return (ax+lo*dx, ay+lo*dy), (ax+hi*dx, ay+hi*dy)


def pixels(image):
    # Pillow 12.3 renamed getdata; keep this disposable host tool compatible.
    get = getattr(image, 'get_flattened_data', image.getdata)
    return get()


def segment_might_enter(a, b, observer, east, north, up, span, trace=None):
    """Conservative real-arithmetic surface-curve bound against the camera frustum.

    For WGS-84, lat/lon tangent speeds are < 6500 km/rad. Along shortest-lon
    linear interpolation, distance from start ECEF <= 6500 * geographic span.
    0.1 km slack covers trial vertex quantization and host rounding. This is
    a preflight culling *candidate*, not an accepted target-float error proof.
    """
    dl = b[0] - a[0]
    if abs(dl) > 180:
        dl -= math.copysign(360, dl)
    radius = 6500 * math.radians(math.hypot(dl, b[1] - a[1])) + .1
    start, _ = surface(*a)
    delta = tuple(start[i] - observer[i] for i in range(3))
    for axis, scale in ((east, span*173/117), (north, span)):
        for sign in (-1, 1):
            normal = tuple(scale*up[i] + sign*axis[i] for i in range(3))
            plane = scale*HEIGHT - dot(delta, normal)
            margin = plane + radius * math.hypot(scale, 1)
            if trace is not None:
                trace['closest_cull_plane_km'] = min(
                    trace.get('closest_cull_plane_km', math.inf), abs(margin))
                if margin < 0:
                    trace['closest_rejected_cull_plane_km'] = min(
                        trace.get('closest_rejected_cull_plane_km', math.inf), -margin)
            if margin < 0:
                return False
    return True


def draw(parts, site, fov, cull=False, sample_step=.2, segments_out=None):
    from PIL import Image, ImageDraw
    observer, camera, east, north, up = setup(site)
    span = math.tan(math.radians(fov)/2)
    layer = Image.new('1', (400, 240), 1)
    pen = ImageDraw.Draw(layer)
    stats = dict(surface_intervals=0, front_tests=0, tangents=0,
                 interior_front=0, clipped_segments=0, drawn_segments=0)

    def eligible(a, b):
        stats['coarse_segments'] += 1
        allowed = segment_might_enter(a, b, observer, east, north, up, span)
        stats['coarse_kept'] += allowed
        return allowed

    if cull:
        stats.update(coarse_segments=0, coarse_kept=0)

    def front(p):
        pos, normal = surface(*p)
        stats['front_tests'] += 1
        return dot(normal, tuple(camera[i] - pos[i] for i in range(3)))

    def project(p):
        pos, _ = surface(*p)
        delta = tuple(pos[i]-observer[i] for i in range(3))
        depth = HEIGHT - dot(delta, up)
        if depth < 1:
            raise ValueError('surface behind camera near plane; no valid Earth view')
        return (dot(delta, east)/depth/span/(173/117), dot(delta, north)/depth/span)

    def root(a, b, fa):
        lo, hi = 0.0, 1.0
        for _ in range(32):
            mid = (lo+hi)/2
            if (front(interpolate(a, b, mid)) > 0) == (fa > 0):
                lo = mid
            else:
                hi = mid
        stats['tangents'] += 1
        return interpolate(a, b, (lo+hi)/2)

    for a, b in pieces(parts, eligible if cull else None, sample_step):
        stats['surface_intervals'] += 1
        fa, fb = front(a), front(b)
        fm = front(interpolate(a, b, 0.5))
        intervals = []
        if fa > 0 and fb > 0:
            if fm <= 0:
                m = interpolate(a, b, .5)
                intervals = [(a, root(a, m, fa)), (root(m, b, fm), b)]
            else:
                intervals = [(a, b)]
        elif fa <= 0 and fb <= 0:
            if fm > 0:
                stats['interior_front'] += 1
                m = interpolate(a, b, .5)
                intervals = [(root(a, m, fa), root(m, b, fm))]
        elif fa > 0:
            intervals = [(a, root(a, b, fa))]
        else:
            intervals = [(root(a, b, fa), b)]
        for start, end in intervals:
            line = clip_box(project(start), project(end))
            if line is None:
                continue
            stats['clipped_segments'] += 1
            def pixel(p):
                def rounded(v):
                    return math.trunc(v + (.5 if v >= 0 else -.5))
                return (176 + rounded(p[0]*173), 120 - rounded(p[1]*117))
            p, q = map(pixel, line)
            if not (3 <= p[0] <= 349 and 3 <= q[0] <= 349 and 3 <= p[1] <= 237 and 3 <= q[1] <= 237):
                raise AssertionError((p, q))
            pen.line([p, q], fill=0, width=1)
            if segments_out is not None:
                # Preserve continuous screen coordinates before endpoint rounding.
                # This is diagnostic geometry, not an approved raster path.
                segments_out.append(((176 + line[0][0]*173, 120 - line[0][1]*117),
                                     (176 + line[1][0]*173, 120 - line[1][1]*117)))
            stats['drawn_segments'] += 1
    stats['coast_ink'] = sum(v == 0 for v in pixels(layer))
    return layer, stats


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    parser.add_argument('frames', type=Path, help='directory of fresh simulator PNGs named site-FOV.png')
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    _, parts, _ = load(args.archive)
    args.output.mkdir(parents=True, exist_ok=True)
    variants = [('raw', 0), ('ecef-40km', 40), ('ecef-75km', 75)]
    from coastline_preflight import SITES
    sites = {'co': SITES['Colorado'], 'sf': SITES['San Francisco'],
             'dateline': SITES['equator/dateline'], 'polar': SITES['north polar']}
    frames = {'co': (5, 20, 100), 'sf': (.1, 1, 5, 20, 100),
              'dateline': (5,), 'polar': (5,)}
    from PIL import Image, ImageDraw
    report = {}
    for label, tolerance in variants:
        reduced = [reduce_part(part, tolerance) for part in parts]
        count = sum(map(len, reduced))
        scan = sum(len(p)-1 for p in reduced)
        full_mesh = sum(1 for _ in pieces(reduced))
        data, quant_error = trial_bytes(reduced)
        trial = args.output / f'trial-{label}.bin'
        trial.write_bytes(data)
        report[label] = {'vertices': count, 'stored_segments': scan,
                         'trial_bytes': trial.stat().st_size,
                         'trial_sha256': hashlib.sha256(data).hexdigest(),
                         'max_coordinate_error_deg': quant_error,
                         'surface_intervals_all_parts': full_mesh, 'frames': {}}
        for site, fovs in frames.items():
            for fov in fovs:
                key = f'{site}-{fov:g}'
                layer, stats = draw(reduced, sites[site], fov)
                src = Image.open(args.frames/f'{key}.png').convert('L').point(lambda c: 255 if c > 127 else 0).convert('1')
                assert src.size == (400, 240)
                composite = Image.new('1', src.size, 1)
                composite.putdata([0 if a == 0 or b == 0 else 255 for a, b in zip(pixels(src), pixels(layer))])
                composite.save(args.output/f'{key}-{label}.png')
                stats['new_ink'] = sum(a != 0 and b == 0 for a, b in zip(pixels(src), pixels(layer)))
                report[label]['frames'][key] = stats
    (args.output/'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({k: {n: v for n, v in data.items() if n != 'frames'} for k, data in report.items()}, indent=2))
    # Report/PNGs live under output; sheets have no annotation inside native cells.
    for site, fovs in frames.items():
        if site not in ('co', 'sf'):
            continue
        labels = ['SOURCE', 'RAW', '40 KM', '75 KM']
        sheet = Image.new('1', (1600, len(fovs)*270 + 32), 1)
        pen = ImageDraw.Draw(sheet)
        for x, label in enumerate(labels):
            pen.text((400*x + 5, 8), label, fill=0)
        for y, fov in enumerate(fovs):
            key = f'{site}-{fov:g}'
            source = Image.open(args.frames/f'{key}.png').convert('L').point(lambda c: 255 if c > 127 else 0).convert('1')
            images = [source] + [Image.open(args.output/f'{key}-{label}.png') for label, _ in variants]
            for col, image in enumerate(images):
                sheet.paste(image, (col*400, 32 + y*270))
            pen.text((5, 32 + y*270 + 242), f'{site} {fov:g} deg', fill=0)
        sheet.save(args.output/f'{site}-sheet.png')


if __name__ == '__main__':
    main()
