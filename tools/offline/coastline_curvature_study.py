#!/usr/bin/env python3
"""Offline second-derivative front/viewport chord bounds; no renderer or asset.

Real-arithmetic inequalities evaluated in host f64. Indeterminate intervals
stay unknown; this does NOT certify target roundoff or clipped raster pixels.
"""
import argparse
import json
import math
from pathlib import Path

from coastline_compare import interpolate
from coastline_preflight import A, HEIGHT, dot, setup, surface
from coastline_visibility_study import MIN_SPAN_DEG, MAX_NODES, enclose


def chord_bound_px(span_deg, fov, height=2.187):
    """Uniform screen-axis chord error for one entire surface interval.

    Depth >= H-2*A-|height|; |east/depth|, |north/depth| <=
    (2*A+|height|)/depth. With |p'|<6500*w, |p''|<15000*w²,
    |(axis/depth)''| <= (1+Q)*(15000+2*6500²/depth)*w²/depth.
    Linear interpolation's midpoint worst-case bound is max|q''|/8.
    Both screen axes have scale 117/tan(FOV/2) in the wide viewport.
    It bounds projected point-to-same-parameter *straight chord* error,
    including paths outside the viewport; it does not bound clipped
    intersection location, front-face root, or the final raster mask.
    """
    if not math.isfinite(span_deg) or span_deg < 0 or not 0 < fov <= 100:
        raise ValueError('invalid span or FOV')
    depth = HEIGHT-2*A-abs(height)
    if depth <= 1:
        raise ValueError('no positive whole-Earth depth bound')
    q = (2*A+abs(height))/depth
    w = math.radians(span_deg)
    return (117/math.tan(math.radians(fov/2)) * (1+q)
            * (15000+2*6500**2/depth) * w*w / depth / 8)


def project(point, site, fov):
    observer, _, east, north, up = setup(site)
    xyz, _ = surface(*point)
    d = tuple(xyz[i]-observer[i] for i in range(3))
    depth = HEIGHT-dot(d, up)
    scale = 117/math.tan(math.radians(fov/2))
    return (scale*dot(d, east)/depth, scale*dot(d, north)/depth)


def study(output):
    tangent = math.degrees(math.acos(A/(A+HEIGHT)))
    cases = {
        'sign_crossing': ((tangent-.3, 0.), (tangent+.3, 0.), (0., 0., 0.), 20.),
        'wide_exact_graze': ((tangent, -.1), (tangent, .1), (0., 0., 0.), 20.),
        'narrow_exact_graze': ((tangent, -.00001), (tangent, .00001), (0., 0., 0.), 20.),
        'hidden_endpoints_visible_middle': ((-89., 0.), (89., 0.), (0., 0., 0.), 5.),
        'both_outside_viewport': ((-2., 0.), (2., 0.), (0., 0., 0.), .1),
        'polar_dateline': ((179.6, 89.9), (-179.6, 89.9), (89.9, 179.9, 0.), .1),
        'front_only': ((-.1, .05), (.1, .05), (0., 0., 0.), .1),
    }
    report = {'min_span_deg': MIN_SPAN_DEG, 'max_nodes': MAX_NODES,
              'chord_bound_px_at_0.2deg_0.1fov': chord_bound_px(.2, .1),
              'cases': {}}
    for name, (a, b, site, fov) in cases.items():
        first = enclose(a, b, site, fov, order=1)
        second = enclose(a, b, site, fov, order=2)
        def compact(result):
            return {k:v for k,v in result.items() if k != 'unknown_intervals'}
        report['cases'][name] = {'first': compact(first), 'second': compact(second)}
    # Geometry-only projection control: do not use pixel equality as a curve oracle.
    a, b, site, fov = cases['front_only']
    p, q = project(a, site, fov), project(b, site, fov)
    maximum = 0.
    for i in range(1001):
        t = i/1000
        r = project(interpolate(a, b, t), site, fov)
        maximum = max(maximum, *(abs(r[j]-(p[j]+t*(q[j]-p[j]))) for j in range(2)))
    report['front_only_chord'] = {'bound_px': chord_bound_px(.2, .1, 0.),
                                  'sampled_max_axis_deviation_px': maximum}
    output.mkdir(parents=True, exist_ok=True)
    (output/'report.json').write_text(json.dumps(report, indent=2) + '\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    print(json.dumps(study(args.output), indent=2))


if __name__ == '__main__':
    main()
