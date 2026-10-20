#!/usr/bin/env python3
"""Offline two-pass whole-geography fallback experiment, NOT a renderer.

Preflight walks all trial geometry without a coast mask or segment list. If the
illustrative policy permits drawing, a second traversal streams pixels into an
already-existing *simulated frame*. The host frame is only an oracle for pixel
comparison; its storage, enclose() interval lists and Python objects are NOT a
1 KiB target-scratch design or a device implementation.
"""
import argparse
import json
import math
from pathlib import Path

from coastline_compare import reduce_part
from coastline_preflight import A, HEIGHT, SITES, load
from coastline_stroke_study import (WIDTH, HEIGHT as SCREEN_HEIGHT, candidate_bounds,
                                    difference, distance_squared, ink)
from coastline_unknown_policy_study import illustrative_fallback, trial
from coastline_unknown_precision_study import FOVS, SITES_BY_KEY

EXAMPLE = 'arc_le_0.001_viewport_le_8'  # illustrative sensitivity input ONLY


def atomic_case(parts, site, fov, max_nodes=8191, policy=EXAMPLE, *,
                pending_limit=None):
    # Nothing is written to the shared target before this completes.
    _, preflight = trial(parts, site, fov, max_nodes=max_nodes, raster=False,
                         pending_limit=pending_limit)
    fallback = illustrative_fallback(preflight)[policy]
    # Simulated pre-existing frame, not a second persistent coast-only layer.
    frame = bytearray([255]) * (WIDTH * SCREEN_HEIGHT)
    commit = None
    drawn_candidates = 0
    if not fallback:
        def draw_segment(a, b):
            nonlocal drawn_candidates
            xmin, xmax, ymin, ymax = candidate_bounds(a, b)
            for y in range(ymin, ymax + 1):
                for x in range(xmin, xmax + 1):
                    drawn_candidates += 1
                    if distance_squared(x, y, a, b) <= .25:
                        frame[y * WIDTH + x] = 0

        _, commit = trial(parts, site, fov, max_nodes=max_nodes, raster=False,
                          on_segment=draw_segment, pending_limit=pending_limit)
        # A mismatch discovered after writing would NOT be recoverable on a
        # real shared target. This only detects drift in the host experiment.
        if any(preflight[k] != commit[k] for k in preflight if k != 'mask_ink'):
            raise AssertionError('late traversal mismatch; simulated ink is not atomic')
        if drawn_candidates != commit['pixel_candidates']:
            raise AssertionError('candidate box / stream count mismatch')

    # Mask oracle is a third, *validation-only* walk, excluded from two-pass
    # accounting. It checks the streamed raster against the existing trial.
    mask, oracle = trial(parts, site, fov, max_nodes=max_nodes,
                         pending_limit=pending_limit)
    if any(preflight[k] != oracle[k] for k in preflight if k != 'mask_ink'):
        raise AssertionError('preflight / oracle work mismatch')
    from PIL import Image
    displayed = Image.frombytes('L', (WIDTH, SCREEN_HEIGHT), bytes(frame)).convert('1')
    changed = difference(displayed, mask)
    if changed != (oracle['mask_ink'] if fallback else 0):
        raise AssertionError('fallback was partial or streamed pixels differ from oracle')
    keys = ('nodes', 'front_subintervals', 'pixel_candidates')
    combined = {k: preflight[k] + (commit[k] if commit else 0) for k in keys}
    return {'fallback': fallback, 'preflight': preflight,
            'commit': commit, 'combined_two_pass_proxy': combined,
            'displayed_ink': ink(displayed), 'local_gap_oracle_ink': oracle['mask_ink'],
            'display_vs_local_gap_pixels': changed,
            'preflight_draw_calls': 0, 'commit_candidate_visits': drawn_candidates}


def study(archive, output):
    _, source, _ = load(archive)  # pinned candidate hash/version checks
    parts = [reduce_part(part, 40) for part in source]  # NOT an approved export
    tangent = math.degrees(math.acos(A/(A+HEIGHT)))
    cases = {f'{key}-{fov:g}': (parts, site, fov, 8191)
             for key, site in SITES_BY_KEY.items() for fov in FOVS}
    cases['mixed-forced-budget'] = (
        [[(-2., 0.), (2., 0.)], [(tangent-.3, 0.), (tangent+.3, 0.)]],
        (0., 0., 0.), 20., 1)
    report = {'source': 'pinned v4.1.0 / unquantized 40 km study only',
              'example_policy_not_selected': EXAMPLE,
              'counts_exclude_validation_oracle': True, 'cases': {}}
    for name, (case_parts, site, fov, max_nodes) in cases.items():
        report['cases'][name] = atomic_case(case_parts, site, fov, max_nodes)
    output.mkdir(parents=True, exist_ok=True)
    (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    report = study(args.archive, args.output)
    print(json.dumps({name: {'fallback': case['fallback'],
                             'combined_two_pass_proxy': case['combined_two_pass_proxy']}
                      for name, case in report['cases'].items()}, indent=2))


if __name__ == '__main__':
    main()
