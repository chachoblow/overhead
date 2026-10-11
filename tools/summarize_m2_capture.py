#!/usr/bin/env python3
"""Derive M2 timing/heap/stack tables from FOUR strictly validated target logs.

Standard library only. Never accesses hardware, refreshes fixtures or accepts
partial runs. Raw logs and separate build/reset provenance remain authoritative.
"""
import argparse
import csv
import hashlib
import io
import json
from pathlib import Path
import statistics
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'firmware'))
from generate_m2 import manifest  # noqa: E402
from m2_capture import PATTERNS, validate_set  # noqa: E402

PHASES = ['input-copy', 'initialization', 'input-release', 'aggregation', 'destruction']
MEMORY = ['requested_peak', 'requested_live', 'occupied_peak', 'occupied_live',
          'calls', 'requested', 'failures']
OWN = ['parse_capacity', 'candidate_capacity', 'entries_length', 'diagnostics_length',
       'memberships_length', 'reports_length', 'passes_length', 'conflict_origins_length']
STACK = ['stack_low', 'stack_top', 'paint_low', 'paint_high', 'untouched_bytes', 'observed_bytes']


def samples(text):
    """Only call after validate_set; this is extraction, not another validator."""
    result = []
    current = None
    for line in text.splitlines():
        f = line.split(',')
        if f[0] == 'BEGIN':
            current = {'case_id': int(f[1]), 'sample': int(f[2]), 'phases': {},
                       'raw_capacities': [], 'stack': {}}
        elif f[0] == 'PHASE':
            current['phases'][f[1]] = {'us': int(f[2]), **dict(zip(MEMORY, map(int, f[3:]), strict=True))}
        elif f[0] == 'PIPE':
            current['pipeline'] = dict(zip(MEMORY, map(int, f[3:]), strict=True))
        elif f[0] == 'RAW':
            current['raw_capacities'].append(int(f[2]))
        elif f[0] == 'OWN':
            current['collections'] = dict(zip(OWN, map(int, f[1:]), strict=True))
        elif f[0] == 'STACK':
            current['stack'][f[1]] = dict(zip(STACK, map(int, f[2:]), strict=True))
        elif f[0] == 'SAMPLE_DONE':
            result.append(current)
            current = None
    return result


def csv_text(rows, fields):
    output = io.StringIO(newline='')
    writer = csv.DictWriter(output, fieldnames=fields, lineterminator='\n')
    writer.writeheader()
    writer.writerows(rows)
    return output.getvalue()


def summarize(measurements, diagnostics):
    validate_set(measurements, diagnostics)
    frozen, digest = manifest()
    timed = [(boot, sample) for boot, text in enumerate(measurements, 1) for sample in samples(text)]
    diagnostic = [(boot, sample) for boot, text in enumerate(diagnostics, 1) for sample in samples(text)]
    timing_rows = []
    for boot, sample in timed:
        row = {'boot': boot, 'case_id': sample['case_id'], 'sample': sample['sample']}
        for phase in PHASES:
            row[phase + '_us'] = sample['phases'].get(phase, {}).get('us', '')
        row.update({'pipeline_' + k: v for k, v in sample['pipeline'].items()})
        for kind in ['requested_live', 'occupied_live']:
            row['aggregation_' + kind] = sample['phases'].get('aggregation', {}).get(kind, '')
        timing_rows.append(row)
    stack_rows = []
    for boot, sample in diagnostic:
        for phase, stack in sample['stack'].items():
            stack_rows.append({'boot': boot, 'pattern': f'0x{PATTERNS[sample["sample"]]:08X}',
                               'case_id': sample['case_id'], 'phase': phase, **stack})
    cases = []
    for frozen_row in frozen['cases']:
        identifier = frozen_row['case']['id']
        rows = [s for _, s in timed if s['case_id'] == identifier]
        phase_times = {}
        for phase in PHASES:
            times = [s['phases'][phase]['us'] for s in rows if phase in s['phases']]
            phase_times[phase] = ({'min_us': min(times), 'median_us': statistics.median(times),
                                   'max_us': max(times)} if times else None)
        first = rows[0]
        cases.append({'case_id': identifier, 'timed_samples': len(rows),
                      'phase_times': phase_times, 'pipeline': first['pipeline'],
                      'phase_memory': {phase: {k: v for k, v in p.items() if k != 'us'}
                                       for phase, p in first['phases'].items()},
                      'raw_capacities': first['raw_capacities'], 'collections': first['collections']})
    summary = {'schema_version': 1, 'manifest_id': frozen['manifest_id'], 'manifest_sha256': digest,
               'scope': 'instrumented target samples, not worst-case/safe-stack/device capacity or operating policy',
               'timed_samples': len(timed), 'stack_observations': len(stack_rows),
               'numerical_rows_per_diagnostic_boot': frozen['numerical']['total_rows'],
               'cases': cases}
    timing_fields = ['boot', 'case_id', 'sample'] + [p + '_us' for p in PHASES]
    timing_fields += ['pipeline_' + k for k in MEMORY] + ['aggregation_requested_live', 'aggregation_occupied_live']
    return {'summary.json': json.dumps(summary, indent=2) + '\n',
            'samples.csv': csv_text(timing_rows, timing_fields),
            'stack.csv': csv_text(stack_rows, ['boot', 'pattern', 'case_id', 'phase'] + STACK)}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--measure', type=Path, nargs=2, required=True)
    p.add_argument('--diagnostic', type=Path, nargs=2, required=True)
    p.add_argument('--output-prefix', type=Path, required=True)
    p.add_argument('--check', action='store_true', help='compare with existing outputs without writing')
    args = p.parse_args()
    paths = args.measure + args.diagnostic
    if len({path.resolve() for path in paths}) != 4:
        p.error('four distinct input paths required; verify reset provenance separately')
    output = summarize([path.read_text() for path in args.measure], [path.read_text() for path in args.diagnostic])
    # Fingerprint original bytes, preserving CRLF/monitor output, not normalized text.
    provenance = {'generator_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                  'inputs': [{'name': path.name, 'bytes': path.stat().st_size,
                              'sha256': hashlib.sha256(path.read_bytes()).hexdigest()} for path in paths]}
    output['summary-inputs.json'] = json.dumps(provenance, indent=2) + '\n'
    outputs = [(Path(str(args.output_prefix) + '-' + suffix), text) for suffix, text in output.items()]
    if args.check:
        for path, text in outputs:
            if path.read_text() != text:
                raise SystemExit(f'derived evidence mismatch: {path}')
        print('All derived M2 tables reproduce from the four validated logs')
    else:
        if any(path.exists() for path, _ in outputs):
            raise SystemExit('refusing to overwrite existing derived evidence; use --check')
        for path, text in outputs:
            with path.open('x') as destination:
                destination.write(text)
            print(path)


if __name__ == '__main__':
    main()
