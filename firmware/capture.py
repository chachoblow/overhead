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


def capture(port, elf, output, timeout):
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
                if b"OVERHEAD_S3_BENCHMARK_DONE" in data:
                    validate(data.decode(errors="replace"))
                    print(f"Complete: 14 workloads × 5 samples saved to {output}")
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


def validate(text):
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


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", required=True)
    parser.add_argument("--elf", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--timeout", type=int, default=600)
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("timeout must be positive")
    if not Path(args.elf).is_file():
        parser.error("ELF must be an existing file")
    capture(args.port, args.elf, args.output, args.timeout)
