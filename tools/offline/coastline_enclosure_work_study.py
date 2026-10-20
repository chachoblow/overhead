#!/usr/bin/env python3
"""Offline source-wide second-order *visibility-enclosure* work preflight.

Uses an unapproved 40 km reduction and optional trial-coordinate decoding.
Counts mathematical candidate intervals, NOT projected coast pixels, S3 cycles,
stack or a production budget. Unknowns are retained; never hidden on exhaustion.
"""
import argparse
import hashlib
import json
from pathlib import Path

from coastline_compare import reduce_part
from coastline_preflight import SITES, load, trial_bytes
from coastline_precision_study import check_shortest_longitude_branches
from coastline_refine_40km import decode_trial
from coastline_visibility_study import MAX_NODES, MIN_SPAN_DEG, enclose

SITES_BY_KEY = {'co': SITES['Colorado'], 'sf': SITES['San Francisco'],
                'dateline': SITES['equator/dateline'], 'polar': SITES['north polar']}
FOVS = (.1, 1., 5., 20., 100.)


def sweep(parts, site, fov, max_nodes=MAX_NODES):
    totals = {k: 0 for k in ('source_segments', 'nodes', 'outside_frustum',
                             'front', 'back', 'unknown', 'budget_unknown')}
    totals.update(max_nodes_per_segment=0, max_unknown_span_deg=0.,
                  total_unknown_span_deg=0., unknown_source_segments=0,
                  budget_source_segments=0, worst_segment=None)
    unknown_ids = set()
    budget_ids = set()
    for part_id, part in enumerate(parts):
        for segment_id, (a, b) in enumerate(zip(part, part[1:])):
            r = enclose(a, b, site, fov, order=2, max_nodes=max_nodes)
            totals['source_segments'] += 1
            for key in ('nodes', 'outside_frustum', 'front', 'back', 'unknown', 'budget_unknown'):
                totals[key] += r[key]
            totals['total_unknown_span_deg'] += r['total_unknown_span_deg']
            totals['max_unknown_span_deg'] = max(totals['max_unknown_span_deg'],
                                                 r['max_unknown_span_deg'])
            identity = (part_id, segment_id)
            if r['unknown']:
                unknown_ids.add(identity)
            if r['budget_unknown']:
                budget_ids.add(identity)
            if r['nodes'] > totals['max_nodes_per_segment']:
                totals['max_nodes_per_segment'] = r['nodes']
                totals['worst_segment'] = {'part': part_id, 'segment': segment_id,
                                           'endpoints': [a, b], 'nodes': r['nodes'],
                                           'unknown': r['unknown'],
                                           'budget_unknown': r['budget_unknown']}
    totals['unknown_source_segments'] = len(unknown_ids)
    totals['budget_source_segments'] = len(budget_ids)
    return totals, unknown_ids, budget_ids


def study(archive, output):
    _, source, _ = load(archive)
    parts = [reduce_part(part, 40) for part in source]
    encoded, error = trial_bytes(parts)
    decoded = decode_trial(encoded, len(parts))
    check_shortest_longitude_branches(parts, decoded)
    if trial_bytes(decoded)[0] != encoded:
        raise AssertionError('trial decode/encode mismatch')
    report = {'trial_sha256': hashlib.sha256(encoded).hexdigest(),
              'trial_bytes': len(encoded), 'max_coordinate_error_deg': error,
              'parts': len(parts), 'vertices': sum(map(len, parts)),
              'source_segments': sum(len(p)-1 for p in parts),
              'per_segment_node_cap': MAX_NODES, 'min_unknown_span_deg': MIN_SPAN_DEG,
              'cases': {}}
    for key, site in SITES_BY_KEY.items():
        for fov in FOVS:
            label = f'{key}-{fov:g}'
            floating, a, a_budget = sweep(parts, site, fov)
            packed, b, b_budget = sweep(decoded, site, fov)
            report['cases'][label] = {
                'unquantized': floating, 'decoded_trial': packed,
                'unknown_segment_symmetric_difference': len(a ^ b),
                'budget_segment_symmetric_difference': len(a_budget ^ b_budget),
            }
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
