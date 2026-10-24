"""Offline audit of retained host work counts against the existing S3 capture.

Does not build, access hardware, or assign target time to individual operations.
Optionally pass a freshly redirected coast_work_review test log to compare it
with the retained counts (host timing may differ).
"""
from pathlib import Path
import statistics
import sys

ROOT = Path(__file__).resolve().parent
CASES = ["sf-tight", "sf-1", "sf-5", "sf-100", "colorado-20", "polar-20",
         "dateline-5", "limb-20", "limb-100", "viewport-tight", "iss-ground-20"]


def read_work(path):
    rows = {}
    for line in path.read_text().splitlines():
        cells = line.split(",")
        if cells[0] not in CASES:
            continue
        assert len(cells) == 9, line
        name = cells[0]
        assert name not in rows, name
        intervals, hidden, roots, strokes, evaluations, writes = map(int, cells[1:7])
        assert intervals == 23_377
        assert 0 <= hidden <= intervals
        assert evaluations == 3 * intervals + 24 * roots + 2 * strokes
        assert writes > 0
        assert all(float(value) > 0 for value in cells[7:])
        rows[name] = (intervals, hidden, roots, strokes, evaluations, writes)
    assert list(rows) == CASES
    return rows


first = read_work(ROOT / "host-run1.txt")
second = read_work(ROOT / "host-run2.txt")
assert first == second
if len(sys.argv) > 1:
    assert len(sys.argv) == 2
    assert read_work(Path(sys.argv[1])) == first
raw = (ROOT.parent / "m4-coastline-s3/capture-on-boot1.log").read_text().splitlines()
print("case,coast_point_evaluations,target_coast_median_us,coast_on_writes")
for name in CASES:
    rows = [line.split(",") for line in raw if line.startswith(f"ROW,{name},Coast,")]
    assert [int(row[3]) for row in rows] == list(range(5))
    assert all(int(row[6]) == int(row[7]) == first[name][5] for row in rows)
    elapsed = statistics.median(int(row[4]) for row in rows)
    print(f"{name},{first[name][4]},{elapsed},{first[name][5]}")
print("PASS: two host count runs agree; all 11 on-write counts match retained S3 Coast rows.")
print("Counts model successful source paths, not measured target instruction counts or phase times.")
