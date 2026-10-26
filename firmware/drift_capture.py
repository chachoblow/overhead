#!/usr/bin/env python3
"""Capture and fit the S3 clock-drift diagnostic; never flashes firmware.

Standard library only. `capture` resets the board via `espflash monitor` and
writes `<host_unix_ns> <line>` for each UART line, refusing to overwrite. Run
`sntp <server>` before/after and record the host offset separately; the fit
uses the host wall clock as the reference. `fit` reports the S3 rate error in
ppm (positive = S3 timer runs fast) and residual spread.
"""
import argparse
import os
from pathlib import Path
import re
import selectors
import subprocess
import time

LINE = re.compile(r"DRIFT seq=(\d+) uptime_us=(\d+)")


def capture(port, elf, output, duration):
    command = ["espflash", "monitor", "--port", port, "--elf", elf,
               "--non-interactive", "--skip-update-check"]
    with Path(output).open("x") as log:
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        selector = selectors.DefaultSelector()
        selector.register(process.stdout, selectors.EVENT_READ)
        pending = b""
        deadline = time.monotonic() + duration
        try:
            while time.monotonic() < deadline:
                if not selector.select(timeout=1):
                    if process.poll() is not None:
                        raise RuntimeError(f"monitor exited {process.returncode}")
                    continue
                chunk = os.read(process.stdout.fileno(), 65536)
                stamp = time.time_ns()
                if not chunk:
                    raise RuntimeError("monitor EOF")
                pending += chunk
                *lines, pending = pending.split(b"\n")
                for raw in lines:
                    text = raw.decode(errors="replace").rstrip("\r")
                    log.write(f"{stamp} {text}\n")
                    if "OVERHEAD_S3_CLOCK_DRIFT_FAILED" in text:
                        raise RuntimeError("firmware reported failure")
                log.flush()
        finally:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
            selector.close()


def parse(path):
    points = []
    for row in Path(path).read_text().splitlines():
        stamp, _, text = row.partition(" ")
        match = LINE.search(text)
        if match:
            points.append((int(stamp) / 1e9, int(match.group(2)) / 1e6, int(match.group(1))))
    seqs = [p[2] for p in points]
    if not seqs or seqs != list(range(seqs[0], seqs[0] + len(seqs))):
        raise ValueError("missing, repeated or reset sequence; split the log")
    return [(h, u) for h, u, _ in points]


OFFSET = re.compile(r"^([+-]\d+\.\d+) \+/- \S+ (\S+)")


def parse_offsets(path):
    """`date -u` lines followed by `sntp` results -> [(unix_s, mean offset_s)].

    sntp's offset is server minus host, so UTC ~= host + offset.
    """
    from datetime import datetime, timezone
    result, stamp, values = [], None, []
    def flush():
        if stamp is not None and values:
            result.append((stamp, sum(values) / len(values)))
    for row in Path(path).read_text().splitlines():
        match = OFFSET.match(row)
        if match:
            values.append(float(match.group(1)))
            continue
        try:
            when = datetime.strptime(row.strip(), "%Y-%m-%dT%H:%M:%SZ")
        except ValueError:
            continue
        flush()
        stamp, values = when.replace(tzinfo=timezone.utc).timestamp(), []
    flush()
    return result


def correct(points, offsets):
    """Add the linearly interpolated host offset; drop points outside its span."""
    out = []
    for host, uptime in points:
        for (t0, o0), (t1, o1) in zip(offsets, offsets[1:]):
            if t0 <= host <= t1:
                out.append((host + o0 + (o1 - o0) * (host - t0) / (t1 - t0), uptime))
                break
    return out


def fit(points):
    """Least squares uptime = a + b*host; ppm = (b-1)*1e6."""
    n = len(points)
    if n < 3:
        raise ValueError("need at least three samples")
    mh = sum(h for h, _ in points) / n
    mu = sum(u for _, u in points) / n
    sxx = sum((h - mh) ** 2 for h, _ in points)
    b = sum((h - mh) * (u - mu) for h, u in points) / sxx
    residuals = [u - (mu + b * (h - mh)) for h, u in points]
    # Lower envelope: minimum host latency per half of the run, robust to
    # occasional USB/pipe delay that only makes host stamps late.
    half = n // 2
    early = min(points[:half], key=lambda p: p[0] - p[1])
    late = min(points[half:], key=lambda p: p[0] - p[1])
    env = (late[1] - early[1]) / (late[0] - early[0])
    return {
        "samples": n,
        "span_s": points[-1][0] - points[0][0],
        "ppm": (b - 1) * 1e6,
        "envelope_ppm": (env - 1) * 1e6,
        "residual_ms_min": min(residuals) * 1e3,
        "residual_ms_max": max(residuals) * 1e3,
    }


OBS = re.compile(r"SNTP seq=(\d+) server=(\S+) outcome=Observation rtt_us=(\d+) u0_us=(\d+) "
                 r"mid_uptime_us=(\d+) utc_us=(-?\d+)")
CHECK = re.compile(r"SNTP_CHECK seq=(\d+) diff_us=(-?\d+) bound_us=(\d+) within=(\w+)")
OTHER = re.compile(r"SNTP seq=\d+ server=\S+ (?:rtt_us=\d+ )?outcome=(\w+)")


def summarize_sntp(text):
    """Live-SNTP log: U0/RTT spread, consistency checks and UTC-vs-uptime rate."""
    obs = [tuple(int(g) if g.lstrip("-").isdigit() else g for g in m.groups())
           for m in OBS.finditer(text)]
    checks = [(int(m.group(2)), int(m.group(3)), m.group(4) == "true") for m in CHECK.finditer(text)]
    outcomes = {}
    for m in OTHER.finditer(text):
        if m.group(1) != "Observation":
            outcomes[m.group(1)] = outcomes.get(m.group(1), 0) + 1
    if len(obs) < 3:
        raise ValueError("need at least three observations")
    u0 = sorted(o[3] for o in obs)
    rtt = sorted(o[2] for o in obs)
    rate = fit([(o[5] / 1e6, o[4] / 1e6) for o in obs])
    return {
        "observations": len(obs),
        "outcomes": outcomes,
        "u0_ms_min_median_max": (u0[0] / 1e3, u0[len(u0) // 2] / 1e3, u0[-1] / 1e3),
        "rtt_ms_min_median_max": (rtt[0] / 1e3, rtt[len(rtt) // 2] / 1e3, rtt[-1] / 1e3),
        "checks": len(checks),
        "checks_within": sum(c[2] for c in checks),
        "max_abs_diff_ms": max(abs(c[0]) for c in checks) / 1e3 if checks else None,
        "max_diff_over_bound": max(abs(c[0]) / c[1] for c in checks) if checks else None,
        "s3_rate_ppm_vs_utc": rate["ppm"],
        "span_s": rate["span_s"],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="cmd", required=True)
    c = sub.add_parser("capture")
    c.add_argument("--port", required=True)
    c.add_argument("--elf", required=True)
    c.add_argument("--output", required=True)
    c.add_argument("--duration", type=float, required=True, help="seconds")
    f = sub.add_parser("fit")
    f.add_argument("log")
    f.add_argument("--skip", type=int, default=5, help="initial samples to drop")
    f.add_argument("--offsets", help="sntp offset log; correct host stamps to UTC")
    n = sub.add_parser("sntp", help="summarize a live-SNTP UART log")
    n.add_argument("log")
    args = parser.parse_args()
    if args.cmd == "sntp":
        for key, value in summarize_sntp(Path(args.log).read_text(errors="replace")).items():
            print(f"{key}: {value}")
        return
    if args.cmd == "capture":
        capture(args.port, args.elf, args.output, args.duration)
    else:
        points = parse(args.log)[args.skip:]
        if args.offsets:
            points = correct(points, parse_offsets(args.offsets))
        result = fit(points)
        for key, value in result.items():
            print(f"{key}: {value:.4f}" if isinstance(value, float) else f"{key}: {value}")


if __name__ == "__main__":
    main()
