"""Offline integrity/regeneration tests for the candidate source pool."""
import datetime as dt
import json
import shutil
import tempfile
import unittest
from pathlib import Path

from prepare_m2_pool import ROOT, eligible, generate, raw_records


class PoolTests(unittest.TestCase):
    def test_pinned_outputs_reproduce_exactly(self):
        for name, data in generate().items():
            self.assertEqual((ROOT / name).read_bytes(), data, name)

    def test_selected_objects_are_verbatim_and_distinct(self):
        generated = generate()
        pool = raw_records(generated["pool.json"])
        selection = json.loads(generated["selection.json"])
        self.assertEqual(len(pool), 16)
        self.assertEqual(len({r["NORAD_CAT_ID"] for r, _ in pool}), 16)
        self.assertLessEqual(len(generated["pool.json"]), 32768)
        for (record, raw), metadata in zip(pool, selection["records"]):
            source = raw_records((ROOT / metadata["source_path"]).read_bytes())
            self.assertEqual((record, raw), source[metadata["source_record_index"]])

    def test_changed_source_fails_hash_gate(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "pool"
            shutil.copytree(ROOT, root)
            path = root / "sources" / "heo.json"
            path.write_bytes(path.read_bytes() + b"\n")
            with self.assertRaisesRegex(ValueError, "hash/length mismatch"):
                generate(root)

    def test_full_document_and_duplicate_keys_checked(self):
        for data in [b'[{"id":1,"id":2}]', b'[{}] trailing', b'[{},', b'{}', b'[1]']:
            with self.subTest(data=data), self.assertRaises(ValueError):
                raw_records(data)
        self.assertEqual(raw_records(b' [ { "id": 1 },\n{"id":2} ]\n'),
                         [({"id": 1}, '{ "id": 1 }'), ({"id": 2}, '{"id":2}')])

    def test_signed_age_boundary_and_orbit_filters(self):
        start = dt.datetime(2026, 10, 10, tzinfo=dt.timezone.utc)
        record = dict(EPOCH="2026-10-08T00:00:00", MEAN_MOTION=14, ECCENTRICITY=0.001)
        self.assertTrue(eligible(record, "leo", start))
        record["EPOCH"] = "2026-10-07T23:59:59.999999"
        self.assertFalse(eligible(record, "leo", start))
        record["EPOCH"] = "2026-10-12T00:00:00"
        self.assertTrue(eligible(record, "leo", start))
        record["EPOCH"] = "2026-10-12T00:00:00.000001"
        self.assertFalse(eligible(record, "leo", start))
        record["EPOCH"] = "2026-10-10T00:00:00"
        record["MEAN_MOTION"] = 2.0
        record["ECCENTRICITY"] = 0.7
        self.assertTrue(eligible(record, "heo", start))
        self.assertFalse(eligible(record, "gnss", start))
        self.assertFalse(eligible(record, "leo", start))
        record["ECCENTRICITY"] = 0.01
        self.assertTrue(eligible(record, "gnss", start))
        self.assertFalse(eligible(record, "heo", start))
        record["MEAN_MOTION"] = 1.0
        self.assertTrue(eligible(record, "geo", start))


if __name__ == "__main__":
    unittest.main()
