#!/usr/bin/env python3
"""Reset and capture one benchmark run using espflash; never flashes firmware.

Standard library only. Preserve raw output (including partial failures), stop at
DONE, and fail on panic, early monitor exit, timeout, or incomplete sample matrix.
"""
import argparse
import os
from pathlib import Path
import selectors
import subprocess
import time

import catalogue_capture


def capture(port, elf, output, timeout, expected=None):
    # Validate the external host manifest before opening/resetting hardware.
    if expected is not None:
        parse_expected(expected)
    command = [
        "espflash", "monitor", "--port", port, "--elf", elf,
        "--non-interactive", "--skip-update-check",
    ]
    # Refuse to overwrite an existing measurement. Open before resetting board.
    with Path(output).open("xb") as log:
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        selector = selectors.DefaultSelector()
        selector.register(process.stdout, selectors.EVENT_READ)
        data = bytearray()
        try:
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                if not selector.select(timeout=1):
                    if process.poll() is not None:
                        raise RuntimeError(f"monitor exited {process.returncode}")
                    continue
                chunk = os.read(process.stdout.fileno(), 65536)
                if not chunk:
                    raise RuntimeError("monitor EOF before completion")
                log.write(chunk)
                log.flush()
                data.extend(chunk)
                if b"OVERHEAD_S3_BENCHMARK_FAILED" in data:
                    raise RuntimeError("firmware reported failure; inspect saved log")
                if b"OVERHEAD_S3_BENCHMARK_DONE\n" in data or b"OVERHEAD_S3_BENCHMARK_DONE\r\n" in data:
                    workloads, samples = validate(data.decode(errors="replace"), expected)
                    print(f"Complete: {workloads} workloads × {samples} samples saved to {output}")
                    return
            raise TimeoutError(f"benchmark did not complete in {timeout}s; partial log saved")
        finally:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            selector.close()
            process.stdout.close()


def validate_v1(text):
    if (text.count("OVERHEAD_S3_BENCHMARK_V1") != 1
            or text.count("OVERHEAD_S3_BENCHMARK_DONE") != 1
            or "OVERHEAD_S3_BENCHMARK_FAILED" in text):
        raise ValueError("missing/repeated run markers or firmware failure")
    expected = {}
    for orbit, evaluations, passes in [
        ("LEO", 1497, 7), ("resonant-HEO", 1449, 1), ("GEO", 1441, 0), ("GNSS", 1449, 1)
    ]:
        expected[("state_at", orbit, 1)] = (1441, 0, 0)
        expected[("state_at+ecef+look_angles", orbit, 1)] = (1441, 0, 0)
        expected[("search_satellite", orbit, 1)] = (evaluations, 1, passes)
    expected[("state_at+ecef+look_angles", "mixed-repeated-fixtures", 4)] = (5764, 0, 0)
    expected[("search_satellite", "mixed-repeated-fixtures", 4)] = (5836, 4, 9)
    seen = set()
    for line in text.splitlines():
        if not line.startswith("ROW,"):
            continue
        _, operation, orbit, slots, sample, elapsed, evaluations, searches, passes = line.split(",")
        key = operation, orbit, int(slots)
        sample_key = key, int(sample)
        work = int(evaluations), int(searches), int(passes)
        if (key not in expected or work != expected[key] or int(elapsed) <= 0
                or not 0 <= int(sample) < 5 or sample_key in seen):
            raise ValueError(f"invalid sample: {line}")
        seen.add(sample_key)
    if len(seen) != 70:
        raise ValueError(f"expected 70 samples, got {len(seen)}")
    return 14, 5


def parse_suite(line):
    tag, name, samples, count = line.split(",")
    samples, count = int(samples), int(count)
    if tag != "SUITE" or not name or not 1 <= samples <= 5 or not 1 <= count <= 200:
        raise ValueError("invalid suite header")
    return name, samples, count


def parse_expected(text):
    """Strict host-generated contract; target-reported counts are not an oracle."""
    if text.startswith(catalogue_capture.EXPECTED):
        return catalogue_capture.parse_expected(text)
    lines = text.splitlines()
    if (len(lines) < 5 or lines[0] != "OVERHEAD_S3_EXPECTED_V2"
            or lines[-1] != "OVERHEAD_S3_EXPECTED_DONE"):
        raise ValueError("invalid expected-work manifest markers")
    name, samples, count = parse_suite(lines[1])
    if len(lines) != 3 + 2 * count:
        raise ValueError("incomplete expected-work manifest")
    cases, work = {}, {}
    for index in range(count):
        case, result = lines[2 + 2 * index:4 + 2 * index]
        tag, identifier, operation, orbit, slots, age, window, detection, tolerance = case.split(",")
        if (tag != "CASE" or int(identifier) != index
                or operation not in ("state_at", "state_at+ecef+look_angles", "search_satellite")
                or orbit not in ("LEO", "resonant-HEO", "GEO", "GNSS", "mixed-repeated-fixtures")
                or int(slots) not in (1, 4, 16, 64) or int(age) not in (-30, -7, -1, 0, 1, 7, 30)
                or int(window) not in (3600, 86400)):
            raise ValueError(f"invalid expected case: {case}")
        if operation == "search_satellite":
            if int(detection) not in (5, 30, 60) or int(tolerance) not in (250, 5000):
                raise ValueError("invalid search settings")
        elif int(detection) != 0 or int(tolerance) != 0 or int(window) != 86400:
            raise ValueError("invalid tracking settings")
        tag, identifier, evaluations, searches, passes = result.split(",")
        counts = int(evaluations), int(searches), int(passes)
        if (tag != "WORK" or int(identifier) != index or counts[0] <= 0
                or min(counts) < 0 or counts[1] != (int(slots) if operation == "search_satellite" else 0)
                or (operation != "search_satellite" and counts != (1441 * int(slots), 0, 0))):
            raise ValueError(f"invalid expected counts: {result}")
        cases[index], work[index] = case, counts
    return (name, samples, count), cases, work


def validate(text, expected=None):
    if (catalogue_capture.START in text
            or (expected is not None and expected.startswith(catalogue_capture.EXPECTED))):
        return catalogue_capture.validate(text, expected)
    if "OVERHEAD_S3_BENCHMARK_V2" not in text:
        if expected is not None:
            raise ValueError("expected a V2 run for the supplied manifest")
        return validate_v1(text)
    if expected is None:
        raise ValueError("V2 capture requires --expected host manifest")
    suite, cases, work = parse_expected(expected)
    name, samples, count = suite
    if (text.count("OVERHEAD_S3_BENCHMARK_V2") != 1
            or text.count("OVERHEAD_S3_BENCHMARK_DONE") != 1
            or "OVERHEAD_S3_BENCHMARK_V1" in text or "OVERHEAD_S3_BENCHMARK_FAILED" in text):
        raise ValueError("missing/repeated run markers or firmware failure")
    lines = text.splitlines()
    start, end = lines.index("OVERHEAD_S3_BENCHMARK_V2"), lines.index("OVERHEAD_S3_BENCHMARK_DONE")
    if start >= end:
        raise ValueError("run markers out of order")
    # Monitor boot chatter is allowed; benchmark records outside the run aren't.
    prefixes = ("SUITE,", "CASE,", "BEGIN,", "ROW,")
    if any(line.startswith(prefixes) for line in lines[:start] + lines[end + 1:]):
        raise ValueError("benchmark records outside run markers")
    records = [line for line in lines[start + 1:end] if line.startswith(prefixes)]
    if not records or records.pop(0) != f"SUITE,{name},{samples},{count}":
        raise ValueError("suite/sample count differs from host manifest")
    # Enforce CASE -> BEGIN -> all samples for each sequential workload. A reset,
    # duplicate row, truncated run, or mismatched build must not look complete.
    if len(records) != count * (2 + samples):
        raise ValueError("incomplete or duplicated sample matrix")
    for identifier in range(count):
        offset = identifier * (2 + samples)
        if records[offset:offset + 2] != [cases[identifier], f"BEGIN,{identifier}"]:
            raise ValueError("workload metadata/order differs from host manifest")
        for sample, line in enumerate(records[offset + 2:offset + 2 + samples]):
            tag, row_id, row_sample, elapsed, evaluations, searches, passes = line.split(",")
            if (tag != "ROW" or int(row_id) != identifier or int(row_sample) != sample
                    or int(elapsed) <= 0 or (int(evaluations), int(searches), int(passes)) != work[identifier]):
                raise ValueError(f"invalid sample: {line}")
    return count, samples


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", required=True)
    parser.add_argument("--elf", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--timeout", type=int, default=600)
    parser.add_argument("--expected", help="host-generated expected-work manifest (required for V2/catalogue)")
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("timeout must be positive")
    if not Path(args.elf).is_file():
        parser.error("ELF must be an existing file")
    expected = Path(args.expected).read_text() if args.expected else None
    capture(args.port, args.elf, args.output, args.timeout, expected)
