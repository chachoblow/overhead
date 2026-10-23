"""Validate this capture's protocol matrix; summaries are observed samples, not bounds."""
from collections import defaultdict
from pathlib import Path
import json

root = Path(__file__).resolve().parent
expected = ['sf-tight', 'sf-1', 'sf-5', 'sf-100', 'colorado-20',
            'polar-20', 'dateline-5', 'limb-20', 'limb-100',
            'viewport-tight', 'iss-ground-20']


def parse(variant):
    text = (root / f'capture-{variant}-boot1.log').read_text(errors='replace')
    lines = [line.strip() for line in text.splitlines()]
    assert [line for line in lines if line.startswith('OVERHEAD_S3_COAST_MEASUREMENT_')] == [
        'OVERHEAD_S3_COAST_MEASUREMENT_V1', 'OVERHEAD_S3_COAST_MEASUREMENT_DONE']
    assert f'coast_on={str(variant == "on").lower()},cpu_mhz=240,buffer_bytes=12000,heap_reserved=65536,samples=5' in lines
    assert 'heap_free=65536,heap_used=0' in lines
    assert 'PROBE,baseline=1632,loaded=9580,stack_low=1070210472,stack_top=1070446336,paint_low=1070210536,paint_high=1070444704' in lines
    cases, controls, pixels, rows = [], {}, [], defaultdict(list)
    for line in lines:
        if line.startswith('CASE,'):
            cases.append(line)
        if line.startswith('OFF_CONTROL,'):
            _, name, checksum = line.split(',')
            assert name not in controls
            controls[name] = int(checksum)
        if line.startswith('PIXEL_CONTROL,'):
            pixels.append(line)
        if line.startswith('ROW,'):
            _, name, mode, index, us, checksum, writes, on_writes, stack = line.split(',')
            rows[name, mode].append(dict(index=int(index), us=int(us), checksum=int(checksum),
                                         writes=int(writes), on_writes=int(on_writes), stack=int(stack)))
    assert [line.split(',')[1] for line in cases] == expected
    assert list(controls) == expected
    modes = {name: (['Coast', 'FrameOn', 'FrameOff', 'Late', 'LateWarning']
                    if name in ('polar-20', 'iss-ground-20') else
                    ['Coast', 'FrameOn', 'FrameOff']) if variant == 'on' else ['FrameOff']
             for name in expected}
    assert list(rows) == [(name, mode) for name in expected for mode in modes[name]]
    assert len(pixels) == (4 if variant == 'on' else 0)
    assert pixels == ([f'PIXEL_CONTROL,{name},{mode},matched,{rows[name, mode][0]["checksum"]}'
                       for name in ('polar-20', 'iss-ground-20')
                       for mode in ('Late', 'LateWarning')] if variant == 'on' else [])
    for (name, mode), samples in rows.items():
        assert [sample['index'] for sample in samples] == list(range(5))
        assert all(sample['us'] > 0 and sample['stack'] > 0 for sample in samples)
        assert all(all(sample[k] == samples[0][k] for k in
                       ('checksum', 'writes', 'on_writes', 'stack')) for sample in samples)
        if mode == 'FrameOff':
            assert samples[0]['checksum'] == controls[name]
        else:
            assert samples[0]['on_writes'] > 0
    return cases, controls, rows

oc, oo, off = parse('off')
nc, no, on = parse('on')
assert oc == nc and oo == no
for name in expected:
    for key in ('checksum', 'writes', 'on_writes', 'stack'):
        assert off[name, 'FrameOff'][0][key] == on[name, 'FrameOff'][0][key]
summary = {}
for mode in ('Coast', 'FrameOn', 'FrameOff', 'Late', 'LateWarning'):
    samples = [(name, item['us']) for (name, current), items in on.items()
               if current == mode for item in items]
    summary[mode] = dict(count=len(samples), min_us=min(us for _, us in samples),
                         max_us=max(us for _, us in samples),
                         max_case=max(samples, key=lambda entry: entry[1])[0])
summary['off'] = dict(cases=len(oc), samples=sum(map(len, off.values())))
summary['on'] = dict(cases=len(nc), samples=sum(map(len, on.values())),
                     matched_late_controls=4)
print(json.dumps(summary, indent=2))
