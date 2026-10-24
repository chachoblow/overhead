#!/usr/bin/env python3
"""One authorized representation boot capture; resets, never flashes.

Gate/flash the exact normal ELF first. Retain raw bytes, including failure/partial
logs. This only detects completion; validate the matrix and masks separately.
"""
import argparse
import os
from pathlib import Path
import select
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("port")
    parser.add_argument("elf", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--timeout", type=float, default=240)
    parser.add_argument("--marker", default="OVERHEAD_S3_COAST_REPRESENTATION",
                        help="image prefix, e.g. OVERHEAD_S3_COAST_PREPARED")
    args = parser.parse_args()
    assert args.elf.is_file() and args.timeout > 0
    command = ["espflash", "monitor", "--port", args.port, "--elf", str(args.elf),
               "--non-interactive", "--skip-update-check"]
    start = time.monotonic()
    with args.output.open("xb") as log:
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        try:
            assert process.stdout is not None
            data = bytearray()
            while time.monotonic() - start < args.timeout:
                ready, _, _ = select.select([process.stdout], [], [], 1)
                if ready:
                    chunk = os.read(process.stdout.fileno(), 65536)
                    if not chunk:
                        raise RuntimeError(f"monitor EOF; exit={process.poll()}")
                    log.write(chunk)
                    log.flush()
                    data.extend(chunk)
                    marker = args.marker.encode()
                    if marker + b"_FAILED" in data:
                        raise RuntimeError("target reported failure; partial log retained")
                    if (marker + b"_DONE\n" in data or marker + b"_DONE\r\n" in data):
                        print(f"DONE: {args.output} ({len(data)} bytes, {time.monotonic()-start:.1f}s)")
                        break
                if process.poll() is not None:
                    raise RuntimeError(f"monitor exited {process.returncode} before DONE")
            else:
                raise TimeoutError("no DONE within timeout; partial log retained")
        finally:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


if __name__ == "__main__":
    main()
