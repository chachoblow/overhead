"""Offline validation tests; never open a serial port or reset hardware."""
import unittest

from capture import parse_expected, validate


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


def v2_fixture(samples=3):
    # Representative manifest format; Rust tests own workload enumeration/counts.
    metadata = "CASE,0,search_satellite,resonant-HEO,1,-30,86400,60,5000"
    header = f"SUITE,ages-heo,{samples},1"
    expected = "\n".join([
        "OVERHEAD_S3_EXPECTED_V2", header, metadata,
        "WORK,0,1449,1,1", "OVERHEAD_S3_EXPECTED_DONE",
    ])
    log = "\n".join([
        "monitor boot chatter", "OVERHEAD_S3_BENCHMARK_V2", header,
        "compiler=test", metadata, "BEGIN,0",
        *(f"ROW,0,{sample},12345,1449,1,1" for sample in range(samples)),
        "OVERHEAD_S3_BENCHMARK_DONE", "monitor exit chatter",
    ])
    return expected, log


class CaptureV2Validation(unittest.TestCase):
    def test_valid_samples_and_crlf(self):
        for samples in range(1, 6):
            expected, log = v2_fixture(samples)
            self.assertEqual(validate(log, expected), (1, samples))
            self.assertEqual(validate(log.replace("\n", "\r\n"), expected), (1, samples))

    def test_requires_matching_manifest_and_version(self):
        expected, log = v2_fixture()
        for text, manifest in [
            (log, None), (complete_log(), expected),
            (log.replace("SUITE,ages-heo", "SUITE,ages-leo"), expected),
            (log.replace("SUITE,ages-heo,3", "SUITE,ages-heo,5"), expected),
            (log.replace(",-30,86400", ",30,86400"), expected),
            (log.replace(",60,5000", ",60,250"), expected),
            (log + "\nOVERHEAD_S3_BENCHMARK_V1", expected),
        ]:
            with self.subTest(text=text[:80]), self.assertRaises(ValueError):
                validate(text, manifest)

    def test_missing_duplicate_reordered_or_corrupt_records(self):
        expected, log = v2_fixture()
        lines = log.splitlines()
        for line in lines:
            if line.startswith(("SUITE,", "CASE,", "BEGIN,", "ROW,", "OVERHEAD_")):
                for invalid in [log.replace(line, ""), log.replace(line, line + "\n" + line)]:
                    with self.subTest(line=line), self.assertRaises(ValueError):
                        validate(invalid, expected)
        for invalid in [
            log.replace("BEGIN,0", "BEGIN,1"),
            log.replace("ROW,0,0,12345", "ROW,0,0,0"),
            log.replace("ROW,0,0,12345", "ROW,0,3,12345"),
            log.replace("ROW,0,0,12345", "ROW,1,0,12345"),
            log.replace("1449,1,1", "1450,1,1"),
            log.replace("ROW,0,0,12345", "ROW,0,0,not-a-number"),
            log.replace("ROW,0,0,12345", "ROW,0,0"),
            log + "\nOVERHEAD_S3_BENCHMARK_FAILED: panic",
            "ROW,0,0,12345,1449,1,1\n" + log,
            log + "\nROW,0,0,12345,1449,1,1",
            log.replace("BEGIN,0\nROW,0,0,12345,1449,1,1", "ROW,0,0,12345,1449,1,1\nBEGIN,0"),
            log.replace("OVERHEAD_S3_BENCHMARK_V2", "TEMP")
                .replace("OVERHEAD_S3_BENCHMARK_DONE", "OVERHEAD_S3_BENCHMARK_V2")
                .replace("TEMP", "OVERHEAD_S3_BENCHMARK_DONE"),
        ]:
            with self.subTest(invalid=invalid[:80]), self.assertRaises(ValueError):
                validate(invalid, expected)

    def test_malformed_manifests(self):
        expected, _ = v2_fixture()
        for invalid in [
            "", expected.replace("OVERHEAD_S3_EXPECTED_DONE", ""),
            expected.replace("SUITE,ages-heo,3,1", "SUITE,ages-heo,0,1"),
            expected.replace("SUITE,ages-heo,3,1", "SUITE,ages-heo,3,2"),
            expected.replace("WORK,0,1449,1,1", "WORK,0,1449,0,1"),
            expected.replace("WORK,0,1449,1,1", "WORK,0,-1,1,1"),
            expected.replace("CASE,0", "CASE,1"),
            expected.replace("WORK,0", "WORK,1"),
            expected.replace(",-30,", ",-31,"),
            expected.replace(",60,5000", ",60,0"),
            expected.replace("search_satellite", "unknown"),
            expected + "\nextra",
        ]:
            with self.subTest(invalid=invalid[:80]), self.assertRaises(ValueError):
                parse_expected(invalid)


if __name__ == "__main__":
    unittest.main()
