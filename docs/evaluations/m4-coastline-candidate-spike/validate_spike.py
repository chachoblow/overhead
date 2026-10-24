"""Audit host-only candidate counts; never infer S3 performance from host time.

Optionally pass a fresh redirected coast_candidate_spike test log. Counts and
On writes must reproduce; elapsed times are observations, not thresholds.
"""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent
baseline = {}
for line in (ROOT.parent / "m4-coastline-work-review/host-run2.txt").read_text().splitlines():
    cells = line.split(",")
    if len(cells) == 9 and cells[1].isdigit():
        baseline[cells[0]] = (int(cells[5]), int(cells[6]))
assert len(baseline) == 11


def read_run(path):
    text = path.read_text()
    assert "test result: ok. 1 passed; 0 failed;" in text
    rows = {}
    for line in text.splitlines():
        cells = line.split(",")
        name = cells[0]
        if name not in baseline:
            continue
        assert name not in rows and len(cells) == 13
        counts = tuple(map(int, cells[1:11]))
        hidden, outside, front, intervals, original, culled, reuse, roots, strokes, writes = counts
        retained = 1734 - hidden - outside
        assert all(value >= 0 for value in counts)
        assert 0 <= front <= retained <= 1734
        assert 0 <= intervals <= 23377
        assert (original, writes) == baseline[name]
        assert culled == 1734 + 3 * intervals + 24 * roots + 2 * strokes
        midpoints = reuse - (1734 + intervals + retained + 25 * roots)
        assert 0 <= midpoints <= intervals
        assert reuse <= culled < original
        assert all(float(value) > 0 for value in cells[11:])
        rows[name] = counts
    assert list(rows) == list(baseline)
    return rows


first = read_run(ROOT / "host-run1.txt")
second = read_run(ROOT / "host-run2.txt")
assert first == second
if len(sys.argv) > 1:
    assert len(sys.argv) == 2
    assert read_run(Path(sys.argv[1])) == first
print("PASS: eleven scenes reproduce counts and prior Coast On-write controls.")
print("Isolated pixel equality is asserted by the Rust probe, not inferred from write counts.")
print("Host results do not establish target phase costs, numeric completeness or S3 viability.")
