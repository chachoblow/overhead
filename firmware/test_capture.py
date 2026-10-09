"""Offline validation tests; never open a serial port or reset hardware."""
import unittest

from capture import validate


def complete_log():
    rows = ["OVERHEAD_S3_BENCHMARK_V1"]
    for orbit, evaluations, passes in [
        ("LEO", 1497, 7), ("resonant-HEO", 1449, 1), ("GEO", 1441, 0), ("GNSS", 1449, 1),
        ("mixed-repeated-fixtures", 5836, 9),
    ]:
        slots = 4 if orbit == "mixed-repeated-fixtures" else 1
        operations = ["state_at+ecef+look_angles", "search_satellite"]
        if slots == 1:
            operations.append("state_at")
        for operation in operations:
            work = (evaluations, slots, passes) if operation == "search_satellite" else (1441 * slots, 0, 0)
            for sample in range(5):
                rows.append(f"ROW,{operation},{orbit},{slots},{sample},12345,{work[0]},{work[1]},{work[2]}")
    rows.append("OVERHEAD_S3_BENCHMARK_DONE")
    return "\n".join(rows)


class CaptureValidation(unittest.TestCase):
    def test_complete_matrix(self):
        validate(complete_log())

    def test_missing_duplicate_or_bad_sample(self):
        text = complete_log()
        first = next(line for line in text.splitlines() if line.startswith("ROW,"))
        for invalid in [
            text.replace(first, ""), text + "\n" + first,
            text.replace(",1497,1,7", ",1498,1,7"),
            text.replace(",12345,", ",0,"),
            text.replace(",0,12345,", ",5,12345,"),
            text.replace("state_at,LEO", "unknown,LEO"),
        ]:
            with self.subTest(invalid=invalid[:80]), self.assertRaises(ValueError):
                validate(invalid)

    def test_missing_or_repeated_markers_and_panics(self):
        text = complete_log()
        for invalid in [
            text.replace("OVERHEAD_S3_BENCHMARK_DONE", ""),
            text.replace("OVERHEAD_S3_BENCHMARK_V1", ""),
            text + "\nOVERHEAD_S3_BENCHMARK_V1",
            text + "\nOVERHEAD_S3_BENCHMARK_DONE",
            text + "\nOVERHEAD_S3_BENCHMARK_FAILED: panic",
        ]:
            with self.assertRaises(ValueError):
                validate(invalid)


if __name__ == "__main__":
    unittest.main()
