#!/usr/bin/env python3
"""Offline bounded pending-stack and streaming-leaf comparison; NOT target code.

16 pending intervals is an illustrative study cap, not a byte or stack limit.
Unknowns (including unfinished work) remain unknown on exhaustion.
"""
import argparse
import json
import math
from pathlib import Path

from coastline_atomic_fallback_study import atomic_case
from coastline_compare import reduce_part
from coastline_preflight import A, HEIGHT, load
from coastline_stroke_study import difference
from coastline_unknown_policy_study import trial
from coastline_unknown_precision_study import FOVS, SITES_BY_KEY

EXAMPLE_PENDING_LIMIT = 16  # unapproved comparison input, not 1 KiB


def study(archive, output):
    _, source, _ = load(archive)
    parts = [reduce_part(part, 40) for part in source]  # unapproved trial
    report = {'source': 'pinned v4.1.0 / unquantized 40 km study only',
              'example_pending_limit': EXAMPLE_PENDING_LIMIT,
              'cases': {}, 'synthetic': {}}
    for key, site in SITES_BY_KEY.items():
        for fov in FOVS:
            name = f'{key}-{fov:g}'
            baseline_mask, baseline = trial(parts, site, fov)
            stream_mask, stream = trial(parts, site, fov,
                                        pending_limit=EXAMPLE_PENDING_LIMIT)
            shared = {k: stream[k] for k in baseline}
            if baseline != shared or difference(stream_mask, baseline_mask):
                raise AssertionError(f'{name}: streaming changed uncapped trial')
            atomic = atomic_case(parts, site, fov,
                                 pending_limit=EXAMPLE_PENDING_LIMIT)
            report['cases'][name] = {
                'peak_pending': stream['peak_pending'],
                'pending_cap_hits': stream['pending_cap_hits'],
                'front_leaves': stream['front_leaves'],
                'unknown': stream['geographic_unknown'],
                'mask_ink': stream['mask_ink'],
                'streamed_vs_list_pixels': difference(stream_mask, baseline_mask),
                'two_pass_fallback': atomic['fallback'],
                'two_pass_displayed_ink': atomic['displayed_ink'],
                'two_pass_work': atomic['combined_two_pass_proxy']}
    tangent = math.degrees(math.acos(A / (A + HEIGHT)))
    controls = {
        'exact_graze_cap4': ([[(tangent, -.1), (tangent, .1)]], (0., 0., 0.), 20., 4),
        'crossing_cap4': ([[(tangent-.3, 0.), (tangent+.3, 0.)]], (0., 0., 0.), 20., 4),
        'mixed_late_cap4': (
            [[(-2., 0.), (2., 0.)], [(tangent-.3, 0.), (tangent+.3, 0.)]],
            (0., 0., 0.), 20., 4),
        'two_offscreen_endpoints': ([[(-2., 0.), (2., 0.)]], (0., 0., 0.), .1, 16),
        'polar_dateline': (
            [[(179.6, 89.9), (-179.6, 89.9)]], (89.9, 179.9, 0.), .1, 16)}
    for name, (case_parts, site, fov, limit) in controls.items():
        atomic = atomic_case(case_parts, site, fov, pending_limit=limit)
        record = {'pending_limit': limit, 'peak_pending': atomic['preflight']['peak_pending'],
                  'pending_cap_hits': atomic['preflight']['pending_cap_hits'],
                  'budget_unknown': atomic['preflight']['budget_unknown'],
                  'geographic_unknown': atomic['preflight']['geographic_unknown'],
                  'fallback': atomic['fallback'],
                  'preflight_draw_calls': atomic['preflight_draw_calls'],
                  'commit_candidate_visits': atomic['commit_candidate_visits'],
                  'displayed_ink': atomic['displayed_ink'],
                  'local_gap_oracle_ink': atomic['local_gap_oracle_ink']}
        report['synthetic'][name] = record
    output.mkdir(parents=True, exist_ok=True)
    (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    report = study(args.archive, args.output)
    print(json.dumps({'max_peak_pending': max(c['peak_pending'] for c in
                                                 report['cases'].values()),
                      'scene_cap_hits': sum(c['pending_cap_hits'] for c in
                                            report['cases'].values()),
                      'synthetic': report['synthetic']}, indent=2))


if __name__ == '__main__':
    main()
