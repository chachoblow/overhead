#!/usr/bin/env python3
"""Offline local-gap/fallback policy comparison; NOT a coastline renderer.

Only certified-front pieces contribute to the experimental stroke. All unknown
surface or viewport intervals stay diagnostic; no joins across missing pieces.
Illustrative fallback thresholds are deliberately NOT an approved policy.
"""
import argparse
import json
import math
from itertools import pairwise
from pathlib import Path

from coastline_compare import clip_box, draw, interpolate, pieces, reduce_part
from coastline_curvature_study import chord_bound_px
from coastline_preflight import A, HEIGHT, SITES, dot, load, setup, surface
from coastline_stroke_study import candidate_count, difference, ink, stroke
from coastline_visibility_study import enclose

CASES = {'sf-0.1': (SITES['San Francisco'], .1),
         'sf-5': (SITES['San Francisco'], 5.),
         'sf-20': (SITES['San Francisco'], 20.),
         'sf-100': (SITES['San Francisco'], 100.),
         'co-5': (SITES['Colorado'], 5.),
         'co-20': (SITES['Colorado'], 20.)}


def normalized(point, observer, east, north, up, span):
    xyz, _ = surface(*point)
    d = tuple(xyz[i]-observer[i] for i in range(3))
    depth = HEIGHT-dot(d, up)
    if not math.isfinite(depth) or depth < 1:
        raise ValueError('invalid surface view')
    return (dot(d, east)/depth/span/(173/117), dot(d, north)/depth/span)


def viewport_piece(p, q, error_px, trace=None):
    """Clip a chord or retain an uncertain curved-viewport contact."""
    ex, ey = error_px/173, error_px/117
    margins = ((max(p[0], q[0])+ex+1)*173, (1-min(p[0], q[0])+ex)*173,
               (max(p[1], q[1])+ey+1)*117, (1-min(p[1], q[1])+ey)*117)
    if trace is not None:
        trace['closest_expanded_viewport_edge_px'] = min(
            trace.get('closest_expanded_viewport_edge_px', math.inf),
            *(abs(value) for value in margins))
    if any(value < 0 for value in margins):
        return 'outside', None
    clipped = clip_box(p, q)
    return ('unknown', None) if clipped is None else ('clipped', clipped)


def trial(parts, site, fov, max_nodes=8191, trace=None, *, raster=True,
          on_segment=None, pending_limit=None):
    """Walk geography; raster=False streams clipped segments without a coast mask.

    The streaming mode counts raster *candidates* via bounding boxes, but does
    not test their distance or track union ink. pending_limit additionally
    streams front leaves from a bounded pending stack, without per-segment
    front/unknown lists. It is a work proxy, not a target memory/timing bound.
    Default behavior remains the original mask trial.
    """
    observer, _, east, north, up = setup(site)
    span = math.tan(math.radians(fov/2))
    stats = dict(source_segments=0, nodes=0, front_leaves=0, geographic_unknown=0,
                 budget_unknown=0, total_unknown_span_deg=0., widest_unknown_span_deg=0.,
                 front_subintervals=0, viewport_unknown=0, outside_viewport=0,
                 clipped_segments=0)
    screen_segments = [] if raster else None
    candidates = 0
    if pending_limit is not None:
        stats.update(peak_pending=0, pending_cap_hits=0)

    def visit_front(start, end):
        nonlocal candidates
        # Sample ONLY certified front leaves. Part/hidden/unknown gaps
        # are never bridged by a projected chord.
        for x, y in pieces([[start, end]], sample_step=.2):
            stats['front_subintervals'] += 1
            p = normalized(x, observer, east, north, up, span)
            q = normalized(y, observer, east, north, up, span)
            geo_span = math.hypot(y[1]-x[1], (y[0]-x[0]+180)%360-180)
            error = chord_bound_px(geo_span, fov, site[2])
            disposition, clipped = viewport_piece(p, q, error, trace=trace)
            if disposition == 'outside':
                stats['outside_viewport'] += 1
                continue
            if disposition == 'unknown':
                # Curved front piece might graze the viewport even when its
                # chord misses it. Never call the curve hidden.
                stats['viewport_unknown'] += 1
                continue
            stats['clipped_segments'] += 1
            a0, b0 = clipped
            segment = ((176+173*a0[0], 120-117*a0[1]),
                       (176+173*b0[0], 120-117*b0[1]))
            if raster:
                screen_segments.append(segment)
            else:
                candidates += candidate_count(*segment)
            if on_segment is not None:
                on_segment(*segment)

    for part in parts:
        for a, b in pairwise(part):
            stats['source_segments'] += 1
            r = enclose(a, b, site, fov, order=2,
                        collect_front=pending_limit is None,
                        collect_unknown=pending_limit is None,
                        on_front=visit_front if pending_limit is not None else None,
                        pending_limit=pending_limit, max_nodes=max_nodes, trace=trace)
            for key in ('nodes', 'budget_unknown'):
                stats[key] += r[key]
            stats['front_leaves'] += r['front']
            stats['geographic_unknown'] += r['unknown']
            stats['total_unknown_span_deg'] += r['total_unknown_span_deg']
            stats['widest_unknown_span_deg'] = max(stats['widest_unknown_span_deg'],
                                                  r['max_unknown_span_deg'])
            if pending_limit is not None:
                stats['peak_pending'] = max(stats['peak_pending'], r['peak_pending'])
                stats['pending_cap_hits'] += r['pending_cap_hits']
            else:
                for start, end in r['front_intervals']:
                    visit_front(start, end)
    if raster:
        mask, candidates = stroke(screen_segments)
        stats.update(pixel_candidates=candidates, mask_ink=ink(mask))
        return mask, stats
    stats.update(pixel_candidates=candidates, mask_ink=None)
    return None, stats


def reference(parts, site, fov):
    segments = []
    draw(parts, site, fov, cull=True, sample_step=.2, segments_out=segments)
    return stroke(segments)[0]


def illustrative_fallback(stats):
    """Threshold *sensitivity*, not approval of any cutoff or on-device rule."""
    return {f'arc_le_{limit:g}_viewport_le_{vp}':
            (stats['budget_unknown'] > 0 or
             stats['total_unknown_span_deg'] > limit or
             stats['viewport_unknown'] > vp)
            for limit, vp in ((.0001, 0), (.001, 8), (.01, 100))}


def study(archive, output):
    _, source, _ = load(archive)
    parts = [reduce_part(part, 40) for part in source]
    tangent = math.degrees(math.acos(A/(A+HEIGHT)))
    controls = {'exact_graze': ([[(tangent, -.1), (tangent, .1)]], (0., 0., 0.), 20.),
                'forced_budget_crossing': ([[(tangent-.3, 0.), (tangent+.3, 0.)]],
                                           (0., 0., 0.), 20.),
                'two_outside_endpoints': ([[(-2., 0.), (2., 0.)]], (0., 0., 0.), .1)}
    report = {'source': 'pinned v4.1.0 / unquantized 40 km study only', 'cases': {},
              'controls': {}}
    for name, (site, fov) in CASES.items():
        mask, stats = trial(parts, site, fov)
        stats['vs_sampled_stroke_pixels'] = difference(mask, reference(parts, site, fov))
        report['cases'][name] = stats
    for name, (case_parts, site, fov) in controls.items():
        mask, stats = trial(case_parts, site, fov,
                            max_nodes=1 if name == 'forced_budget_crossing' else 8191)
        stats['vs_sampled_stroke_pixels'] = difference(mask, reference(case_parts, site, fov))
        report['controls'][name] = stats
    # Illustrative sensitivity ONLY. These cutoffs must not silently become a
    # runtime contract, nor can they certify target-float correctness.
    for case in (report['cases'], report['controls']):
        for stats in case.values():
            stats['illustrative_fallback'] = illustrative_fallback(stats)
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
