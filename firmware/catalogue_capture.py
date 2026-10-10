"""Offline protocol validation for the bounded catalogue memory experiment.

No serial I/O here. capture.py owns reset/capture and preserves failed logs.
"""
START = "OVERHEAD_S3_CATALOGUE_MEMORY_V1"
EXPECTED = "OVERHEAD_S3_CATALOGUE_EXPECTED_V1"
DONE = "OVERHEAD_S3_BENCHMARK_DONE"
PREFIXES = ("MATRIX,", "HEAP,", "PROBE,", "CASE,", "BEGIN,", "PHASE,", "WORK,", "STACK,", "ROW,", "SUITE,")


def parse_expected(text):
    lines = text.splitlines()
    if len(lines) != 19 or lines[0] != EXPECTED or lines[-1] != "OVERHEAD_S3_EXPECTED_DONE":
        raise ValueError("invalid catalogue manifest markers/length")
    tag, samples, count = lines[1].split(",")
    samples, count = int(samples), int(count)
    if tag != "MATRIX" or not 1 <= samples <= 5 or count != 8:
        raise ValueError("invalid catalogue matrix")
    cases, work = [], []
    for identifier in range(count):
        case, result = lines[2 + 2 * identifier:4 + 2 * identifier]
        tag, index, size, window, allowance, json_bytes = case.split(",")
        size, window, allowance = int(size), int(window), int(allowance)
        expected_size = 4 if identifier < 4 else 8
        expected_window = 3600 if identifier % 4 == 0 else 86400
        expected_allowance = [200000 * expected_size, 200000 * expected_size, 1000, 0][identifier % 4]
        if (tag != "CASE" or int(index) != identifier or size != expected_size
                or window != expected_window or allowance != expected_allowance or int(json_bytes) <= 0):
            raise ValueError("invalid catalogue case")
        tag, index, *values = result.split(",")
        if tag != "WORK" or int(index) != identifier or len(values) != 5:
            raise ValueError("invalid expected work")
        evaluations, passes, candidates, searched, complete = map(int, values)
        if (min(evaluations, passes, candidates, searched, complete) < 0
                or evaluations > allowance or candidates > passes or searched > size
                or complete != int(identifier % 4 < 2)
                or (complete and searched != size)
                or (allowance == 0 and (evaluations, passes, candidates, searched) != (0, 0, 0, 0))):
            raise ValueError("invalid catalogue work counts")
        cases.append(case)
        work.append(tuple(map(int, values)))
    return samples, cases, work


def validate(text, expected):
    if expected is None:
        raise ValueError("catalogue capture requires host manifest")
    samples, cases, work = parse_expected(expected)
    lines = text.splitlines()
    markers = [line for line in lines if line.startswith("OVERHEAD_")]
    if markers != [START, DONE] or "OVERHEAD_S3_BENCHMARK_FAILED" in text:
        raise ValueError("invalid catalogue run markers/failure")
    start, end = lines.index(START), lines.index(DONE)
    if any(line.startswith(PREFIXES) for line in lines[:start] + lines[end + 1:]):
        raise ValueError("catalogue record outside run")
    records = [line for line in lines[start + 1:end] if line.startswith(PREFIXES)]
    if len(records) != 3 + 8 * (2 + 4 * samples):
        raise ValueError("incomplete/duplicate catalogue matrix")
    if records[0] != f"MATRIX,{samples},8":
        raise ValueError("catalogue sample count mismatch")
    tag, capacity, free, initial_used = records[1].split(",")
    capacity, free, initial_used = int(capacity), int(free), int(initial_used)
    if tag != "HEAP" or capacity != 65536 or not 0 < free <= capacity or initial_used != 0:
        raise ValueError("unexpected experiment heap")
    tag, baseline, loaded = records[2].split(",")
    baseline, loaded = int(baseline), int(loaded)
    if tag != "PROBE" or baseline <= 0 or loaded < baseline + 4096 or baseline % 4 or loaded % 4:
        raise ValueError("stack probe did not observe known local")
    offset = 3
    stack_bounds = None
    for identifier in range(8):
        if records[offset:offset + 2] != [cases[identifier], f"BEGIN,{identifier}"]:
            raise ValueError("catalogue case/order mismatch")
        offset += 2
        previous = None
        for sample in range(samples):
            phases = []
            for name in ("init", "aggregate"):
                fields = records[offset].split(",")
                offset += 1
                if len(fields) != 12 or fields[:4] != ["PHASE", str(identifier), str(sample), name]:
                    raise ValueError("invalid catalogue phase/order")
                us, req_peak, req_live, occ_peak, occ_live, calls, requested, failures = map(int, fields[4:])
                if (min(us, req_peak, req_live, occ_peak, occ_live, calls, requested, failures) < 0
                        or failures != 0 or calls == 0 or requested == 0
                        or not 0 < req_live <= req_peak <= occ_peak <= free
                        or not req_live <= occ_live <= occ_peak):
                    raise ValueError("invalid heap counters")
                phases.append((req_peak, req_live, occ_peak, occ_live, calls, requested, failures))
            if (phases[1][1] < phases[0][1] or phases[1][3] < phases[0][3]
                    or (previous is not None and phases != previous)):
                raise ValueError("retained catalogue lost or unstable heap counts")
            previous = phases
            fields = records[offset].split(",")
            offset += 1
            if (len(fields) != 8 or fields[:3] != ["WORK", str(identifier), str(sample)]
                    or tuple(map(int, fields[3:])) != work[identifier]):
                raise ValueError("catalogue work differs from host")
            fields = records[offset].split(",")
            offset += 1
            if len(fields) != 9 or fields[:3] != ["STACK", str(identifier), str(sample)]:
                raise ValueError("invalid stack record/order")
            low, top, paint_low, paint_high, untouched, observed = map(int, fields[3:])
            if (not 0 < low < paint_low < paint_high < top or any(x % 4 for x in (low, top, paint_low, paint_high, untouched, observed))
                    or not 256 <= untouched <= paint_high - paint_low
                    or observed != top - paint_low - untouched):
                raise ValueError("invalid/exhausted stack watermark")
            if stack_bounds is not None and stack_bounds != (low, top, paint_low):
                raise ValueError("linker stack bounds changed within run")
            stack_bounds = low, top, paint_low
    return 8, samples
