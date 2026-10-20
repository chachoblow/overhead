#!/usr/bin/env python3
"""Headless 1-bit coast-only local-gap/fallback sensitivity comparison.

No finished-frame compositing, approved tolerance, target-float proof, or renderer.
The panels deliberately omit limb, objects, notices and other foreground ink.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path

from coastline_compare import reduce_part
from coastline_preflight import A, HEIGHT, SITES, load
from coastline_unknown_policy_study import trial, illustrative_fallback

PANELS = ('co-20', 'sf-20', 'mixed-forced-budget')
# Tolerances are illustrative *inputs*, never a selected runtime policy.
EXAMPLES = {'co-20': 'arc_le_0.0001_viewport_le_0',
            'sf-20': 'arc_le_0.001_viewport_le_8',
            'mixed-forced-budget': 'arc_le_0.001_viewport_le_8'}


def cases(parts):
    tangent = math.degrees(math.acos(A/(A+HEIGHT)))
    return {'co-20': (parts, SITES['Colorado'], 20., 8191),
            'sf-20': (parts, SITES['San Francisco'], 20., 8191),
            # Keep separately certified, visible center coast while a second
            # segment exhausts a one-node limit near a horizon crossing.
            'mixed-forced-budget': (
                [[(-2., 0.), (2., 0.)], [(tangent-.3, 0.), (tangent+.3, 0.)]],
                (0., 0., 0.), 20., 1)}


def study(archive, output, poster=None):
    from PIL import Image
    _, source, _ = load(archive)
    parts = [reduce_part(part, 40) for part in source]
    report = {'poster_layout': {'columns_left_to_right': list(PANELS),
                                'top': 'local-gap coast-only trial',
                                'bottom': 'illustrative whole-geography fallback'},
              'cases': {}}
    tiles = []
    for name in PANELS:
        case_parts, site, fov, max_nodes = cases(parts)[name]
        mask, stats = trial(case_parts, site, fov, max_nodes=max_nodes)
        choices = illustrative_fallback(stats)
        trigger = choices[EXAMPLES[name]]
        final = Image.new('1', mask.size, 1) if trigger else mask
        record = {'illustrative_threshold': EXAMPLES[name], 'fallback_triggered': trigger,
                  'pre_fallback_ink': stats['mask_ink'],
                  'post_fallback_ink': stats['mask_ink'] if not trigger else 0,
                  'mask_sha256': hashlib.sha256(mask.tobytes()).hexdigest(),
                  'displayed_sha256': hashlib.sha256(final.tobytes()).hexdigest(),
                  'work': stats}
        report['cases'][name] = record
        tiles.append((mask, final))
    image = Image.new('1', (1200, 480), 1)
    for i, (gap, fallback) in enumerate(tiles):
        image.paste(gap, (400*i, 0))
        image.paste(fallback, (400*i, 240))
    output.mkdir(parents=True, exist_ok=True)
    (output/'report.json').write_text(json.dumps(report, indent=2) + '\n')
    image.save(output/'poster.png')
    if poster is not None:
        poster.parent.mkdir(parents=True, exist_ok=True)
        image.save(poster)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--poster', type=Path,
                        help='optionally save diagnostic poster outside tmp')
    args = parser.parse_args()
    print(json.dumps(study(args.archive, args.output, args.poster), indent=2))


if __name__ == '__main__':
    main()
