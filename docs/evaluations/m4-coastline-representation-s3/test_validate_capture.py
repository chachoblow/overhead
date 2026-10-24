"""Offline parser/oracle negative controls. Never opens a serial port."""
import shutil
import tempfile
import unittest
from pathlib import Path
from validate_capture import HERE, audit, parse


class CaptureAudit(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw = (HERE / "capture-on.log").read_bytes()

    def rejects(self, data):
        with self.assertRaises((AssertionError, ValueError)):
            parse(data, True)

    def test_retained_complete_capture_and_host_bytes(self):
        result = audit(HERE)
        self.assertEqual(result["exact_host_masks"], 48)
        self.assertEqual(result["on"]["rows"], 185)

    def test_missing_duplicate_or_failed_completion(self):
        self.rejects(self.raw.replace(b"OVERHEAD_S3_COAST_REPRESENTATION_DONE", b"TRUNCATED"))
        self.rejects(self.raw + b"\nOVERHEAD_S3_COAST_REPRESENTATION_DONE\n")
        self.rejects(self.raw + b"\nOVERHEAD_S3_COAST_REPRESENTATION_FAILED\n")

    def test_duplicate_missing_or_reordered_sample(self):
        row = next(line for line in self.raw.splitlines(keepends=True) if line.startswith(b"ROW,"))
        self.rejects(self.raw.replace(row, row + row, 1))
        self.rejects(self.raw.replace(row, b"", 1))
        self.rejects(self.raw.replace(b"ROW,sf-tight,Coast,0,", b"ROW,sf-tight,Coast,1,", 1))

    def test_masks_are_ordered_complete_and_checksummed(self):
        line = next(line for line in self.raw.splitlines(keepends=True) if line.startswith(b"MASK,"))
        self.rejects(self.raw.replace(line, b"", 1))
        self.rejects(self.raw.replace(line, line + line, 1))
        self.rejects(self.raw.replace(b"MASK,sf-tight,Coast,0,", b"MASK,sf-tight,Coast,1,", 1))
        prefix, hexdata = line.rsplit(b",", 1)
        changed = prefix + b"," + (b"f" if hexdata[:1] != b"f" else b"0") + hexdata[1:]
        self.rejects(self.raw.replace(line, changed, 1))

    def test_wrong_clock_case_or_missing_rollback_control(self):
        self.rejects(self.raw.replace(b"cpu_mhz=240", b"cpu_mhz=80", 1))
        self.rejects(self.raw.replace(b"CASE,sf-tight,37.774900", b"CASE,sf-tight,38.774900", 1))
        line = next(line for line in self.raw.splitlines(keepends=True) if line.startswith(b"PIXEL_CONTROL,"))
        self.rejects(self.raw.replace(line, b"", 1))

    def test_external_expected_counts_are_not_replaced_by_warmup(self):
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            for name in ["host-expected.csv", "host-masks.zip", "capture-on.log", "capture-off.log"]:
                shutil.copyfile(HERE / name, directory / name)
            path = directory / "host-expected.csv"
            text = path.read_text().replace("sf-tight,Coast,283078817,237,237", "sf-tight,Coast,283078817,238,237")
            path.write_text(text)
            with self.assertRaises(AssertionError):
                audit(directory)


if __name__ == "__main__":
    unittest.main()
