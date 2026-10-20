#!/usr/bin/env python3
"""Offline real-arithmetic visibility enclosure on synthetic surface curves.

NOT a renderer, target-float bound, or proof of projected-curve raster accuracy.
Indeterminate intervals are retained, never silently classified as hidden.
"""
import argparse
import json
import math
from pathlib import Path

from coastline_compare import draw, interpolate, segment_might_enter
from coastline_preflight import A, HEIGHT, dot, setup, surface
from coastline_stroke_study import ink, stroke

# Host double computations of the test functions are not certified roundoff.
# Slacks help experiments but DO NOT turn them into interval arithmetic.
FRONT_SLACK_KM = 1e-6
MIN_SPAN_DEG = 1e-5
MAX_NODES = 8191


def angular_span(a, b):
    dl = b[0] - a[0]
    if abs(dl) > 180:
        dl -= math.copysign(360, dl)
    return math.hypot(dl, b[1] - a[1])


def front_lipschitz(a, b, height):
    """|df/dt| bound in km for unit parameter t over geographic-linear arc.

    |normal'| <= angular speed, |ECEF'| <6500 km/rad on WGS-84,
    |camera-ECEF| <= HEIGHT+2*A+|site altitude|.
    """
    return (HEIGHT + 2*A + abs(height) + 6500) * math.radians(angular_span(a, b))


def front_value(point, camera):
    xyz, normal = surface(*point)
    return dot(normal, tuple(camera[i] - xyz[i] for i in range(3)))


def front_second_bound(a, b, height):
    """Conservative |d²f/dt²| for a geographic-linear WGS-84 interval.

    |n''| <= 2*w²; |n'| <= w; |p'| < 6500*w;
    |p''| < 15000*w² from N=a/sqrt(1-e²sin²(lat)) and its first two
    derivatives (N<6400, |dN/dlat|<22, |d²N/dlat²|<44).
    """
    w = math.radians(angular_span(a, b))
    return (2*(HEIGHT + 2*A + abs(height)) + 2*6500 + 15000) * w*w


def front_slope(point, a, b, camera):
    """Analytic df/dt at point for the shortest-longitude interval a->b."""
    lat, lon = map(math.radians, (point[1], point[0]))
    alpha = math.radians(b[1]-a[1])
    dl = b[0]-a[0]
    if abs(dl) > 180:
        dl -= math.copysign(360, dl)
    beta = math.radians(dl)
    s, c = math.sin(lat), math.cos(lat)
    cl, sl = math.cos(lon), math.sin(lon)
    e2 = 1 - (1 - 1/298.257223563)**2
    n = A/math.sqrt(1-e2*s*s)
    dn = A*e2*s*c*alpha/(1-e2*s*s)**1.5
    normal = (c*cl, c*sl, s)
    normal_prime = (-s*alpha*cl-c*sl*beta,
                    -s*alpha*sl+c*cl*beta, c*alpha)
    xyz = (n*c*cl, n*c*sl, n*(1-e2)*s)
    position_prime = (dn*c*cl+n*normal_prime[0],
                      dn*c*sl+n*normal_prime[1],
                      (1-e2)*(dn*s+n*c*alpha))
    return (dot(normal_prime, tuple(camera[i]-xyz[i] for i in range(3)))
            - dot(normal, position_prime))


def enclose(a, b, site, fov, min_span=MIN_SPAN_DEG, max_nodes=MAX_NODES,
            order=1, collect_front=False, trace=None, *, on_front=None,
            on_unknown=None, collect_unknown=True, pending_limit=None):
    """Subdivide/cull; return certified-sign leaves and retained unknown leaves.

    Sound in exact real arithmetic given WGS-84 derivative/sphere bounds; in
    Python this is a diagnostic because f64 trig/dot roundoff is not bounded.
    """
    if not math.isfinite(min_span) or min_span <= 0 or max_nodes < 1 or order not in (1, 2):
        raise ValueError('finite positive min_span, node budget and order 1/2 required')
    if pending_limit is not None and (type(pending_limit) is not int or pending_limit < 1):
        raise ValueError('positive integer pending limit required')
    observer, camera, east, north, up = setup(site)
    span = math.tan(math.radians(fov / 2))
    if not 0 < span < math.inf:
        raise ValueError('invalid FOV')
    result = {'outside_frustum': 0, 'front': 0, 'back': 0,
              'unknown': 0, 'budget_unknown': 0, 'nodes': 0,
              'max_unknown_span_deg': 0., 'total_unknown_span_deg': 0.}
    if collect_unknown:
        result['unknown_intervals'] = []
    if collect_front:
        result['front_intervals'] = []
    if pending_limit is not None:
        result.update(peak_pending=1, pending_cap_hits=0)
    pending = [(a, b)]

    def unknown(x, y, budget=False):
        result['unknown'] += 1
        result['budget_unknown'] += int(budget)
        arc = angular_span(x, y)
        result['max_unknown_span_deg'] = max(result['max_unknown_span_deg'], arc)
        result['total_unknown_span_deg'] += arc
        if collect_unknown:
            result['unknown_intervals'].append([x, y])
        if on_unknown is not None:
            on_unknown(x, y, budget)
    while pending:
        start, end = pending.pop()
        if result['nodes'] >= max_nodes:
            # Work exhaustion retains ALL pending intervals as unknown. No
            # partial "no coastline" or erroneous back-face classification.
            unknown(start, end, budget=True)
            for x, y in pending:
                unknown(x, y, budget=True)
            break
        result['nodes'] += 1
        if not segment_might_enter(start, end, observer, east, north, up, span,
                                   trace=trace):
            result['outside_frustum'] += 1
            continue
        midpoint = interpolate(start, end, .5)
        # Parameter-distance from midpoint is <= half the original span.
        half = (front_lipschitz(start, end, site[2]) / 2 if order == 1 else
                abs(front_slope(midpoint, start, end, camera))/2
                + front_second_bound(start, end, site[2])/8)
        f = front_value(midpoint, camera)
        if trace is not None:
            # Empirical host-f64 distance to the two sign-classification
            # thresholds. NOT a roundoff error bound or a root bound.
            if f-half > FRONT_SLACK_KM or f+half < -FRONT_SLACK_KM:
                margin = (f-half-FRONT_SLACK_KM if f > 0 else
                          -f-half-FRONT_SLACK_KM)
                trace['closest_certified_sign_km'] = min(
                    trace.get('closest_certified_sign_km', math.inf), margin)
                trace['smallest_certified_sign_vs_f_ulp'] = min(
                    trace.get('smallest_certified_sign_vs_f_ulp', math.inf),
                    margin / max(math.ulp(f), math.ulp(half)))
        if f - half > FRONT_SLACK_KM:
            result['front'] += 1
            if collect_front:
                result['front_intervals'].append([start, end])
            if on_front is not None:
                on_front(start, end)
        elif f + half < -FRONT_SLACK_KM:
            result['back'] += 1
        elif angular_span(start, end) <= min_span:
            unknown(start, end)
        elif pending_limit is not None and len(pending) + 2 > pending_limit:
            # An overflow cannot be retried after streaming earlier front leaves.
            # Keep BOTH would-be children and all unfinished work unknown; do
            # not call their surface interval back-facing or drop them.
            result['pending_cap_hits'] += 1
            unknown(midpoint, end, budget=True)
            unknown(start, midpoint, budget=True)
            for x, y in pending:
                unknown(x, y, budget=True)
            break
        else:
            pending.extend(((midpoint, end), (start, midpoint)))
            if pending_limit is not None:
                result['peak_pending'] = max(result['peak_pending'], len(pending))
    return result


def study(output):
    # Equatorial tangent for an equatorial zero-height observer on WGS-84:
    # f = (A+HEIGHT)*cos(lon)-A on the equator.
    tangent = math.degrees(math.acos(A / (A + HEIGHT)))
    cases = {
        'sign_crossing': ([(tangent-.3, 0.), (tangent+.3, 0.)], (0., 0., 0.), 20.),
        'exact_graze_both_hidden': ([(tangent, -.1), (tangent, .1)], (0., 0., 0.), 20.),
        'narrow_exact_graze': ([(tangent, -.00001), (tangent, .00001)], (0., 0., 0.), 20.),
        'back_back_visible_middle': ([(-89., 0.), (89., 0.)], (0., 0., 0.), 5.),
        'both_outside_crossing_viewport': ([(-2., 0.), (2., 0.)], (0., 0., 0.), .1),
        'polar_dateline': ([(179.6, 89.9), (-179.6, 89.9)], (89.9, 179.9, 0.), .1),
        'far_back': ([(170., 0.), (175., 0.)], (0., 0., 0.), 5.),
    }
    report = {'tangent_longitude_deg': tangent, 'min_span_deg': MIN_SPAN_DEG,
              'max_nodes_per_case': MAX_NODES, 'front_sign_slack_km': FRONT_SLACK_KM,
              'cases': {}}
    for name, (part, site, fov) in cases.items():
        a, b = part
        result = enclose(a, b, site, fov)
        # Draw's existing sampled-curve path is an illustrative control, NOT
        # a reference oracle for arbitrary tangent or viewport accuracy.
        segments = []
        _, old = draw([part], site, fov, cull=True, sample_step=.2, segments_out=segments)
        mask, _ = stroke(segments)
        result.update(endpoint_front_values=[front_value(p, setup(site)[1]) for p in part],
                      midpoint_front_value=front_value(interpolate(a, b, .5), setup(site)[1]),
                      sampled_tangents=old['tangents'], sampled_stroke_ink=ink(mask))
        report['cases'][name] = result
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
