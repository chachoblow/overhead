#!/usr/bin/env python3
"""Offline 40 km trial-coordinate precision check, not asset adoption.

Compare decoded 1e-4 degree trial vertices against unquantized vertices using
one common 0.2 degree sampled curve and continuous clipped-stroke raster.
Pixel equality is not a tangent, float implementation, or source-detail bound.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path

from coastline_compare import draw, reduce_part
from coastline_preflight import A, HEIGHT, SITES, load, trial_bytes
from coastline_refine_40km import decode_trial
from coastline_stroke_study import WIDTH, difference, distance_squared, ink, pixels, stroke

CASES = {
    'co-0.1': (SITES['Colorado'], .1),
    'co-5': (SITES['Colorado'], 5.),
    'sf-0.1': (SITES['San Francisco'], .1),
    'sf-1': (SITES['San Francisco'], 1.),
    'sf-5': (SITES['San Francisco'], 5.),
    'dateline-0.1': (SITES['equator/dateline'], .1),
    'dateline-5': (SITES['equator/dateline'], 5.),
    'polar-0.1': (SITES['north polar'], .1),
    'polar-5': (SITES['north polar'], 5.),
}


def vertex_screen_bound(error_deg, min_fov=.1, max_height=2.187):
    """Conservative real-arithmetic bound for corresponding *on-screen points*.

    A linear lat/shortest-lon parameterization has coordinate error at most
    sqrt(2)*error_deg; WGS-84 coordinate speed <6500 km/rad. The deliberately
    loose positive depth bound is H-2*A-|height|, valid for these surface sites.
    For a point with |east/depth| <= aspect*tan(fov/2), the max pixel delta
    in one screen axis is 117/tan * delta/(depth-delta) * (1+aspect*tan).
    NOT a bound on tangent/root location, clipped endpoint, or raster pixels.
    """
    delta = 6500 * math.radians(math.sqrt(2) * error_deg)
    span = math.tan(math.radians(min_fov / 2))
    depth = HEIGHT - 2*A - abs(max_height)
    assert 0 < delta < depth
    return dict(max_surface_position_km=delta, min_depth_km=depth,
                onscreen_axis_displacement_px=(117/span) * delta/(depth-delta)
                                              * (1 + (173/117)*span))


def check_shortest_longitude_branches(parts, decoded):
    """Fail closed if trial rounding changes an interpolation wrap choice."""
    for a, b in zip(parts, decoded):
        for (p, q), (r, s) in zip(zip(a, a[1:]), zip(b, b[1:])):
            if (abs(q[0]-p[0]) > 180) != (abs(s[0]-r[0]) > 180):
                raise ValueError('trial coordinate rounding changed longitude branch')


def render(parts, site, fov, cull):
    segments = []
    _, stats = draw(parts, site, fov, cull=cull, segments_out=segments)
    mask, candidates = stroke(segments)
    return mask, segments, stats, candidates


def changed_pixels(first, second, a, b):
    changed = []
    for index, (x, y) in enumerate(zip(pixels(first), pixels(second))):
        if (x == 0) == (y == 0):
            continue
        px, py = index % WIDTH, index // WIDTH
        def nearest(segments):
            return min((math.sqrt(distance_squared(px, py, s, e)) for s, e in segments),
                       default=None)
        changed.append(dict(x=px, y=py, unquantized_ink=x == 0,
                            unquantized_distance_px=nearest(a),
                            decoded_distance_px=nearest(b)))
    return changed


def study(archive, output):
    _, source, _ = load(archive)
    parts = [reduce_part(part, 40) for part in source]
    encoded, error = trial_bytes(parts)
    decoded = decode_trial(encoded, len(parts))
    if trial_bytes(decoded)[0] != encoded:
        raise AssertionError('trial round-trip changed')
    check_shortest_longitude_branches(parts, decoded)
    output.mkdir(parents=True, exist_ok=True)
    report = dict(trial_sha256=hashlib.sha256(encoded).hexdigest(),
                  trial_bytes=len(encoded), vertices=sum(map(len, parts)),
                  max_coordinate_error_deg=error,
                  corresponding_onscreen_point_bound=vertex_screen_bound(error), cases={})
    for name, (site, fov) in CASES.items():
        a, aseg, astats, atests = render(parts, site, fov, True)
        b, bseg, bstats, btests = render(decoded, site, fov, True)
        # Check the cull separately for both geometries; no hidden ink from it.
        afull, _, _, _ = render(parts, site, fov, False)
        bfull, _, _, _ = render(decoded, site, fov, False)
        if difference(a, afull) or difference(b, bfull):
            raise AssertionError(f'coarse cull changed stroke pixels: {name}')
        report['cases'][name] = dict(pixel_differences=difference(a, b),
                                     changed_pixels=changed_pixels(a, b, aseg, bseg),
                                     unquantized_ink=ink(a),
                                     decoded_ink=ink(b),
                                     admitted_segments=astats['coarse_kept'],
                                     inspected_segments=astats['coarse_segments'],
                                     unquantized_intervals=astats['surface_intervals'],
                                     decoded_intervals=bstats['surface_intervals'],
                                     unquantized_tangents=astats['tangents'],
                                     decoded_tangents=bstats['tangents'],
                                     unquantized_clipped_segments=astats['clipped_segments'],
                                     decoded_clipped_segments=bstats['clipped_segments'],
                                     unquantized_pixel_candidates=atests,
                                     decoded_pixel_candidates=btests)
    (output/'report.json').write_text(json.dumps(report, indent=2) + '\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    print(json.dumps(study(args.archive, args.output), indent=2))


if __name__ == '__main__':
    main()
