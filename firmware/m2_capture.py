#!/usr/bin/env python3
"""Fail-closed frozen M2 protocol validation; optional reset/capture, NEVER flash.

Default operation is offline. Captures preserve raw/partial logs and require the
frozen source gates and exact ELF/image gate before opening a monitor.
"""
import argparse
import json
import math
import os
from pathlib import Path
import selectors
import subprocess
import time

from generate_m2 import manifest, outcomes
from image_gate import validate as image_validate

START = 'OVERHEAD_S3_M2_V1'
DONE = 'OVERHEAD_S3_M2_DONE'
PATTERNS = [0xA5A5A5A5, 0x5A5A5A5A]
NAMES = ['input-copy', 'initialization', 'input-release', 'aggregation', 'destruction']
HARDWARE = 'cpu_mhz=240,cores_used=1,psram=unused,allocator=esp-alloc-0.11.0-LLFF,math=f64_libm'
TAGS = ('MANIFEST,', 'MODE,', 'HEAP,', 'PROBE,', 'NUM,', 'BEGIN,', 'WARMUP,', 'ENDPOINT',
        'ACCEPT,', 'PARSE,', 'INVALID,', 'CONFLICT,', 'ORIGIN,', 'FAIL,', 'WORK,', 'SAT,',
        'PHASE,', 'PIPE,', 'RAW,', 'RECORD,', 'OWN,', 'STACK,', 'SAMPLE_DONE,')


def require(ok, message):
    if not ok:
        raise ValueError(message)


def integers(fields):
    require(all(s.isascii() and s.isdecimal() and (s == '0' or not s.startswith('0')) for s in fields),
            'noncanonical unsigned integer')
    values = tuple(map(int, fields))
    require(all(v < 2**64 for v in values), 'integer overflow')
    return values


class Records:
    def __init__(self, lines):
        self.lines = iter(lines)

    def line(self):
        try:
            return next(self.lines)
        except StopIteration as e:
            raise ValueError('truncated M2 capture') from e

    def exact(self, expected):
        actual = self.line()
        require(actual == expected, f'expected {expected!r}, got {actual!r}')

    def numbers(self, prefix, count):
        fields = self.line().split(',')
        p = prefix.split(',')
        require(fields[:len(p)] == p and len(fields) == len(p) + count, f'invalid {prefix} row')
        return integers(fields[len(p):])


def memory(records, prefix, free, diagnostic, previous=(0, 0)):
    us, rp, rl, op, ol, calls, requested, failures = records.numbers(prefix, 8)
    require(failures == 0 and rl <= rp <= op <= free and rl <= ol <= op,
            'invalid heap peak/live/failure counters')
    require(rp >= previous[0] and op >= previous[1], 'phase peak lost retained storage')
    require((calls == 0) == (requested == 0), 'allocation calls/bytes mismatch')
    require(rl <= previous[0] + requested and rp <= previous[0] + requested,
            'requested live/peak exceeds starting live plus allocations')
    if calls == 0:
        require((rp, op) == previous and ol <= previous[1], 'peak/occupancy grew without allocation')
    if diagnostic:
        require(us == 0, 'diagnostic run cannot report timings')
    elif prefix in ('PHASE,initialization', 'PHASE,aggregation'):
        require(us > 0, 'missing elapsed phase timing')
    return (rp, rl, op, ol, calls, requested, failures)


def validate(text, frozen=None, digest=None, mode=None):
    if frozen is None:
        frozen, digest = manifest()
    lines = text.splitlines()
    require([l for l in lines if l.startswith('OVERHEAD_')] == [START, DONE], 'missing/repeated/failure markers')
    require(not any(s in text.lower() for s in ('panicked at', 'guru meditation', 'multiple drom', 'stack overflow', 'alloc error')),
            'target failure/boot diagnostic')
    start, end = lines.index(START), lines.index(DONE)
    require(start < end, 'reversed markers')
    require(not any(l.startswith(TAGS) for l in lines[:start] + lines[end + 1:]), 'M2 record outside run')
    r = Records(lines[start + 1:end])
    r.exact(f'MANIFEST,{frozen["manifest_id"]},{digest}')
    selected = r.line()
    require(selected in ('MODE,measure', 'MODE,diagnostic'), 'invalid mode')
    selected = selected.split(',')[1]
    require(mode is None or mode == selected, 'wrong capture mode')
    diagnostic = selected == 'diagnostic'
    capacity, free, used = r.numbers('HEAP', 3)
    require(capacity == 65536 and 0 < free <= capacity and used == 0, 'wrong heap/baseline')
    require(r.line().startswith('compiler=rustc '), 'missing compiler inventory')
    r.exact(HARDWARE)
    if diagnostic:
        for family, count in frozen['numerical']['rows'].items():
            for index in range(count):
                r.exact(f'NUM,{family},{index},Ok(())')
    signatures = {}
    bounds = None

    def sample(row, index):
        nonlocal bounds
        c = row['case']
        identifier = c['id']
        r.exact(f'BEGIN,{identifier},{index}')
        r.exact(f'ENDPOINT,{identifier},{index}')
        for line in outcomes(row).splitlines():
            r.exact(line)
        r.exact('ENDPOINT_DONE')
        phase_names = [name for name in NAMES if name != 'aggregation' or row['search'] is not None]
        phases = {}
        previous = (0, 0)
        for name in phase_names:
            p = memory(r, f'PHASE,{name}', free, diagnostic, previous)
            phases[name] = p
            previous = (p[1], p[3])
        pipe = memory(r, 'PIPE,whole', free, True)
        require(pipe[1] == pipe[3] == 0, 'pipeline did not fully release')
        require(pipe[0] == max(p[0] for p in phases.values()) and pipe[2] == max(p[2] for p in phases.values()),
                'whole-pipeline peak differs from independently observed phase maxima')
        require(pipe[4] == sum(p[4] for p in phases.values()) and pipe[5] == sum(p[5] for p in phases.values()),
                'unaccounted pipeline allocation')
        copy = phases['input-copy']
        init = phases['initialization']
        release = phases['input-release']
        destroy = phases['destruction']
        ram = c['lifetime'] != 'flash-control'
        input_bytes = row['input_json_bytes'] if ram else 0
        require(copy[0] == copy[1] == copy[5] == input_bytes and copy[4] == (len(row['groups']) if ram else 0),
                'RAM copy missing from pipeline or unexpected flash allocation')
        require(init[4] > 0 and init[1] > input_bytes, 'missing initialization/retention')
        require(release[4] == destroy[4] == 0, 'release/destruction allocated')
        require(release[1] == init[1] - (input_bytes if c['lifetime'] == 'ram-dropped' else 0), 'wrong input lifetime')
        require(destroy[1] == destroy[3] == 0, 'destruction did not restore baseline')
        if 'aggregation' in phases:
            require(phases['aggregation'][1] > release[1] and phases['aggregation'][3] > release[3], 'missing retained reports/candidates')
        raw, observations = [], []
        for g in row['groups']:
            (cap,) = r.numbers(f'RAW,{g["index"]}', 1)
            count = g['records'] or 0
            require((cap == 0 if count == 0 else count <= cap <= 32), 'invalid parser-array capacity')
            raw.append(cap)
        for g in row['groups']:
            for record in range(g['records'] or 0):
                live, occupied, calls, requested = r.numbers(f'RECORD,{g["index"]},{record}', 4)
                require(live <= occupied <= pipe[2] and live <= pipe[0] and calls <= init[4] and requested <= init[5],
                        'invalid record allocation observation')
                observations.append((live, occupied, calls, requested))
        own = r.numbers('OWN', 8)
        accepted = row['accepted'] or []
        search = row['search']
        require(own[0] >= len(row['diagnostics']['parse']) and own[0] <= 32, 'parse diagnostic capacity')
        candidates = search['earliest_candidates'] if search else 0
        require(candidates <= own[1] <= 32 and (own[1] == 0 if not candidates else True), 'candidate capacity')
        require(own[2:] == (len(accepted), len(row['diagnostics']['core']), sum(len(a['groups']) for a in accepted),
                           len(search['satellites']) if search else 0, search['stored_passes'] if search else 0,
                           sum(len(d.get('origins', [])) for d in row['diagnostics']['core'])), 'collection lengths differ from frozen outcomes')
        if diagnostic:
            for name, ids in frozen['stack_captures']['phase_case_ids'].items():
                if identifier not in ids:
                    continue
                low, top, paint_low, high, untouched, observed = r.numbers(f'STACK,{name}', 6)
                require(0 < low < paint_low < high < top and not any(v % 4 for v in (low, top, paint_low, high, untouched, observed))
                        and 256 <= untouched <= high - paint_low and observed == top - paint_low - untouched,
                        'invalid/exhausted stack watermark')
                current = low, top, paint_low
                require(bounds is None or bounds == current, 'linker stack bounds changed')
                bounds = current
        r.exact(f'SAMPLE_DONE,{identifier},{index}')
        signature = (phases, pipe, raw, observations, own)
        require(identifier not in signatures or signatures[identifier] == signature, 'nonrepeating work/heap/collection counters')
        signatures[identifier] = signature

    if diagnostic:
        for index, pattern in enumerate(PATTERNS):
            baseline, loaded = r.numbers(f'PROBE,{pattern}', 2)
            require(0 < baseline and loaded >= baseline + 4096 and baseline % 4 == loaded % 4 == 0, 'stack smoke test failed')
            for row in frozen['cases']:
                sample(row, index)
            require(bounds is not None and loaded < bounds[1] - bounds[0], 'smoke depth exceeds linker stack')
    else:
        for row in frozen['cases']:
            r.exact(f'WARMUP,{row["case"]["id"]},ok')
            for index in range(3):
                sample(row, index)
    require(next(r.lines, None) is None, 'unknown/extra records in run')
    return selected, signatures


def validate_set(measurements, diagnostics):
    """Two separately captured boots of EACH exact image; latency remains open."""
    require(len(measurements) == len(diagnostics) == 2, 'need two timed and two diagnostic captures')
    frozen, digest = manifest()
    reference = None
    for mode, texts in [('measure', measurements), ('diagnostic', diagnostics)]:
        for text in texts:
            _, signature = validate(text, frozen, digest, mode)
            require(reference is None or signature == reference, 'heap/collection counters differ across boots/images')
            reference = signature
    return reference


def timeout_seconds(frozen, mode):
    # Frozen search work, 10ms/evaluation allowance used ONLY for capture planning:
    # >5x the historical eight-object ~14.53s/11616 evaluations. Four passes per
    # timed boot (warmup+3), two per diagnostic boot. Add 300s for ingestion,
    # UART/numerics/painting, then a further 2x margin; round up to a minute.
    evaluations = sum((r['search'] or {}).get('evaluations', 0) for r in frozen['cases'])
    invocations = 4 if mode == 'measure' else 2
    return math.ceil(2 * (evaluations * invocations * .010 + 300) / 60) * 60


def capture(port, elf, image, output, mode):
    frozen, digest = manifest()  # Full frozen source check BEFORE reset.
    from generate_m2 import DEST, generate
    require(DEST.read_text() == generate(), 'stale generated flash tables')
    elf, image, output = Path(elf), Path(image), Path(output)
    gate = image_validate(elf.read_bytes(), image.read_bytes())
    timeout = timeout_seconds(frozen, mode)
    command = ['espflash', 'monitor', '--port', port, '--elf', str(elf), '--non-interactive', '--skip-update-check']
    # Both files are exclusive-created BEFORE monitor/reset. Never overwrite
    # accepted or failed evidence; a failure leaves the raw partial log intact.
    metadata = output.with_suffix(output.suffix + '.json')
    require(not output.exists() and not metadata.exists(), 'output/sidecar already exists')
    with output.open('xb') as log, metadata.open('x') as sidecar:
        json.dump({'mode': mode, 'manifest_sha256': digest, 'timeout_s': timeout,
                   'command': command, 'image_gate': gate, 'status': 'capture-attempt-not-acceptance'}, sidecar, indent=2)
        sidecar.flush()
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        selector = selectors.DefaultSelector()
        selector.register(process.stdout, selectors.EVENT_READ)
        data = bytearray()
        try:
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                if not selector.select(timeout=1):
                    if process.poll() is not None:
                        raise RuntimeError(f'monitor exited {process.returncode}; partial log saved')
                    continue
                chunk = os.read(process.stdout.fileno(), 65536)
                if not chunk:
                    raise RuntimeError('monitor EOF; partial log saved')
                log.write(chunk)
                log.flush()
                data.extend(chunk)
                require(b'OVERHEAD_S3_M2_FAILED' not in data, 'firmware failed; partial log saved')
                if DONE.encode() + b'\n' in data or DONE.encode() + b'\r\n' in data:
                    validate(data.decode(errors='replace'), frozen, digest, mode)
                    print(f'Validated {mode} boot: {output}; not full evidence-set acceptance')
                    return
            raise TimeoutError(f'{timeout}s timeout; partial log saved')
        finally:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            selector.close()
            process.stdout.close()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    sub = p.add_subparsers(dest='action', required=True)
    single = sub.add_parser('validate')
    single.add_argument('log', type=Path)
    pair = sub.add_parser('validate-set')
    pair.add_argument('--measure', type=Path, nargs=2, required=True)
    pair.add_argument('--diagnostic', type=Path, nargs=2, required=True)
    sub.add_parser('timeouts')
    live = sub.add_parser('capture', help='resets device; use only after hardware approval')
    for name in ['port', 'elf', 'image', 'output']:
        live.add_argument('--' + name, required=True)
    live.add_argument('--mode', choices=['measure', 'diagnostic'], required=True)
    args = p.parse_args()
    if args.action == 'validate':
        mode, rows = validate(args.log.read_text())
        print(f'Valid {mode} boot: {len(rows)} cases; not full evidence-set acceptance')
    elif args.action == 'validate-set':
        paths = args.measure + args.diagnostic
        require(len({path.resolve() for path in paths}) == 4, 'four distinct capture files required')
        validate_set([path.read_text() for path in args.measure], [path.read_text() for path in args.diagnostic])
        print('Four logs pass protocol checks; review reset/build/frame provenance before evidence acceptance')
    elif args.action == 'timeouts':
        frozen, _ = manifest()
        for mode in ['measure', 'diagnostic']:
            print(f'{mode}: {timeout_seconds(frozen, mode)} seconds per boot')
    else:
        capture(args.port, args.elf, args.image, args.output, args.mode)


if __name__ == '__main__':
    main()
