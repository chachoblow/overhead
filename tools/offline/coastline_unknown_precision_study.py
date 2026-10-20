#!/usr/bin/env python3
"""Paired decoded-coordinate local-gap diagnostic; no export/renderer/float proof."""
import argparse
import hashlib
import json
import math
from pathlib import Path

from coastline_compare import reduce_part, segment_might_enter
from coastline_preflight import A, HEIGHT, SITES, load, setup, trial_bytes
from coastline_precision_study import check_shortest_longitude_branches
from coastline_refine_40km import decode_trial
from coastline_stroke_study import WIDTH, difference, pixels
from coastline_unknown_policy_study import illustrative_fallback, trial, viewport_piece
from coastline_visibility_study import enclose, front_value

SITES_BY_KEY = {'co': SITES['Colorado'], 'sf': SITES['San Francisco'],
                'dateline': SITES['equator/dateline'], 'polar': SITES['north polar']}
FOVS = (.1, 1., 5., 20., 100.)


def compare(a_parts, b_parts, site, fov):
    a_trace, b_trace = {}, {}
    a_mask, a = trial(a_parts, site, fov, trace=a_trace)
    b_mask, b = trial(b_parts, site, fov, trace=b_trace)
    changed = difference(a_mask, b_mask)
    first = [(i % WIDTH, i // WIDTH) for i, (x, y) in
             enumerate(zip(pixels(a_mask), pixels(b_mask))) if x != y][:8]
    a['illustrative_fallback'] = illustrative_fallback(a)
    b['illustrative_fallback'] = illustrative_fallback(b)
    return {'unquantized': a, 'decoded_trial': b,
            'host_f64_threshold_distances': {'unquantized': a_trace, 'decoded_trial': b_trace},
            'changed_stroke_pixels': changed, 'first_changed_pixels': first,
            'different_fallback_examples': [key for key in a['illustrative_fallback']
                                            if a['illustrative_fallback'][key] !=
                                            b['illustrative_fallback'][key]]}


def synthetic_threshold_probes():
    """Nextafter input probes: condition examples, NOT arithmetic error bounds."""
    site = (0., 0., 0.)
    observer, camera, east, north, up = setup(site)
    tangent = math.degrees(math.acos(A/(A+HEIGHT)))
    near_tangent = []
    for direction in (-math.inf, math.inf):
        lon = tangent
        for _ in range(16):
            lon = math.nextafter(lon, direction)
        r = enclose((lon, 0.), (lon, 0.), site, 20., order=2)
        near_tangent.append({'input_lon_deg': lon, 'front_km': front_value((lon, 0.), camera),
                             'classification': {key: r[key] for key in ('front', 'back', 'unknown')}})
    span = math.tan(math.radians(.1/2))
    # Degenerate surface segment at the positive east frustum edge. Binary
    # search only locates a *host* f64 transition, not a true real root.
    lo, hi = 0., 2.
    for _ in range(60):
        mid = (lo+hi)/2
        if segment_might_enter((mid, 0.), (mid, 0.), observer, east, north, up, span):
            lo = mid
        else:
            hi = mid
    cull = []
    for lon in (math.nextafter(lo, -math.inf), lo, hi,
                math.nextafter(hi, math.inf)):
        trace = {}
        kept = segment_might_enter((lon, 0.), (lon, 0.), observer, east, north,
                                   up, span, trace=trace)
        cull.append({'lon_deg': lon, 'kept': kept, 'host_threshold_distance_km':
                     trace['closest_cull_plane_km']})
    # Projected-chord bbox just outside the +x viewport. Expand it by .05px;
    # moving one input ulp can change a strict <0 test at that boundary.
    x = 1+.05/173
    viewport = []
    for value in (math.nextafter(x, -math.inf), x, math.nextafter(x, math.inf)):
        trace = {}
        disposition, _ = viewport_piece((value, 0.), (value, .1), .05, trace)
        viewport.append({'normalized_x': value, 'disposition': disposition,
                         'host_threshold_distance_px':
                         trace['closest_expanded_viewport_edge_px']})
    return {'front_near_tangent': near_tangent, 'cull_near_edge': cull,
            'viewport_near_edge': viewport}


def study(archive, output):
    _, source, _ = load(archive)
    parts = [reduce_part(part, 40) for part in source]
    packed, error = trial_bytes(parts)
    decoded = decode_trial(packed, len(parts))
    if trial_bytes(decoded)[0] != packed:
        raise AssertionError('round-trip trial encoding mismatch')
    check_shortest_longitude_branches(parts, decoded)
    report = {'trial_sha256': hashlib.sha256(packed).hexdigest(),
              'trial_bytes': len(packed), 'parts': len(parts),
              'source_segments': sum(len(p)-1 for p in parts),
              'max_coordinate_error_deg': error,
              'synthetic_nextafter_input_probes': synthetic_threshold_probes(),
              'cases': {}}
    for name, site in SITES_BY_KEY.items():
        for fov in FOVS:
            report['cases'][f'{name}-{fov:g}'] = compare(parts, decoded, site, fov)
    output.mkdir(parents=True, exist_ok=True)
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
