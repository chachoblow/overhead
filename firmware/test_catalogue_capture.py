"""Synthetic protocol tests, NOT hardware measurements."""
import unittest
from unittest.mock import patch

from capture import capture, parse_expected, validate


def fixture(samples=3):
    expected = ["OVERHEAD_S3_CATALOGUE_EXPECTED_V1", f"MATRIX,{samples},8"]
    log = ["boot chatter", "OVERHEAD_S3_CATALOGUE_MEMORY_V1", f"MATRIX,{samples},8",
           "compiler=test", "HEAP,65536,65536,0", "PROBE,512,8704"]
    counts = [(252, 1, 1, 4, 1), (5812, 6, 1, 4, 1), (1000, 2, 1, 1, 0), (0, 0, 0, 0, 0),
              (500, 3, 1, 8, 1), (11616, 13, 1, 8, 1), (1000, 2, 1, 1, 0), (0, 0, 0, 0, 0)]
    for identifier, work in enumerate(counts):
        size = 4 if identifier < 4 else 8
        allowance = [200000 * size, 200000 * size, 1000, 0][identifier % 4]
        window = 3600 if identifier % 4 == 0 else 86400
        case = f"CASE,{identifier},{size},{window},{allowance},100"
        values = ",".join(map(str, work))
        expected += [case, f"WORK,{identifier},{values}"]
        log += [case, f"BEGIN,{identifier}"]
        for sample in range(samples):
            log += [f"PHASE,{identifier},{sample},init,100,12000,3000,12500,3100,35,27000,0",
                    f"PHASE,{identifier},{sample},aggregate,0,5000,4500,5500,4600,10,5000,0",
                    f"WORK,{identifier},{sample},{values}",
                    f"STACK,{identifier},{sample},1000,10000,1064,9000,7000,1936"]
    expected += ["OVERHEAD_S3_EXPECTED_DONE"]
    log += ["OVERHEAD_S3_BENCHMARK_DONE", "exit chatter"]
    return "\n".join(expected), "\n".join(log)


class CatalogueCapture(unittest.TestCase):
    def test_complete_and_crlf(self):
        for samples in range(1, 6):
            expected, log = fixture(samples)
            parse_expected(expected)
            self.assertEqual(validate(log, expected), (8, samples))
            self.assertEqual(validate(log.replace("\n", "\r\n"), expected), (8, samples))

    def test_missing_duplicated_or_out_of_order_records(self):
        expected, log = fixture()
        lines = log.splitlines()
        for index, line in enumerate(lines):
            if line.startswith(("OVERHEAD_", "MATRIX,", "HEAP,", "PROBE,", "CASE,", "BEGIN,", "PHASE,", "WORK,", "STACK,")):
                for replacement in [[], [line, line]]:
                    with self.subTest(line=line), self.assertRaises(ValueError):
                        validate("\n".join(lines[:index] + replacement + lines[index + 1:]), expected)
        phase = next(i for i, line in enumerate(lines) if line.startswith("PHASE,"))
        lines[phase], lines[phase + 1] = lines[phase + 1], lines[phase]
        with self.assertRaises(ValueError):
            validate("\n".join(lines), expected)

    def test_corrupt_counts_resources_markers_and_settings(self):
        expected, log = fixture()
        for old, new in [
            ("MATRIX,3,8", "MATRIX,5,8"),
            ("HEAP,65536,65536,0", "HEAP,65536,65536,1"),
            ("HEAP,65536", "HEAP,131072"),
            ("PROBE,512,8704", "PROBE,512,512"),
            ("PROBE,512,8704", "PROBE,0,8704"),
            ("CASE,0,4,3600,800000,100", "CASE,0,4,3600,800000,101"),
            ("init,100,12000", "init,-1,12000"),
            ("12000,3000,12500", "12000,13000,12500"),
            ("12000,3000,12500", "12000,3000,11000"),
            ("12000,3000,12500", "12000,3000,70000"),
            ("35,27000,0", "35,27000,1"),
            ("PHASE,0,0", "PHASE,0,2"),
            ("init,100", "init,not-a-number"),
            ("WORK,0,0,252", "WORK,0,0,253"),
            ("WORK,0,0,252", "WORK,0,0"),
            ("1064,9000,7000,1936", "1064,9000,0,8936"),
            ("1064,9000,7000,1936", "1064,9000,8000,936"),
            ("1064,9000,7000,1936", "1064,9000,7000,2000"),
            ("1064,9000,7000,1936", "1065,9000,6999,1936"),
        ]:
            with self.subTest(new=new), self.assertRaises(ValueError):
                validate(log.replace(old, new, 1), expected)
        for extra in ["OVERHEAD_S3_BENCHMARK_FAILED: panic", "OVERHEAD_S3_BENCHMARK_V2",
                      "ROW,0,0,123", "PHASE,0,0,init,1,2", "OVERHEAD_S3_CATALOGUE_MEMORY_V1"]:
            for invalid in [extra + "\n" + log, log + "\n" + extra]:
                with self.assertRaises(ValueError):
                    validate(invalid, expected)
        with self.assertRaises(ValueError):
            validate(log)

    def test_invalid_manifest_rejected_before_monitor_or_output_file(self):
        expected, log = fixture()
        for bad in [expected.replace("MATRIX,3,8", "MATRIX,0,8"),
                    expected.replace("CASE,0,4", "CASE,1,4"),
                    expected.replace("800000", "900000"),
                    expected.replace("252,1,1,4,1", "252,1,1,4,0"),
                    expected.replace("WORK,3,0,0,0,0,0", "WORK,3,1,0,0,0,0"),
                    expected.replace("OVERHEAD_S3_EXPECTED_DONE", ""),
                    expected + "\nWORK,8,0,0,0,0,0"]:
            with self.subTest(bad=bad[:80]), self.assertRaises(ValueError):
                parse_expected(bad)
            with patch("capture.subprocess.Popen") as process, patch("capture.Path.open") as file:
                with self.assertRaises(ValueError):
                    capture("unused", "unused", "unused", 1, bad)
                process.assert_not_called()
                file.assert_not_called()
        with self.assertRaises(ValueError):
            validate(log, expected.replace("OVERHEAD_S3_CATALOGUE_EXPECTED_V1", "OVERHEAD_S3_EXPECTED_V2"))


if __name__ == "__main__":
    unittest.main()
