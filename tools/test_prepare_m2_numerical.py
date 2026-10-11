"""Offline selection/provenance and malformed-reference regression tests."""
import copy
import json
import unittest
from unittest.mock import patch

import prepare_m2_numerical as numerical


class NumericalTests(unittest.TestCase):
    def test_exact_archive_and_selection(self):
        data = numerical.prepare()
        self.assertEqual(data, json.loads(numerical.DEST.read_text()))
        self.assertEqual([(r["norad_id"], r["minutes"]) for r in data["teme"]],
                         [(i, m) for i in numerical.IDS for m in numerical.MINUTES])
        for path, keys in [("coordinates.json", ("rotations", "geodetic")),
                           ("observer.json", ("geometry", "iss_pipeline"))]:
            original = json.loads((numerical.ROOT / "core/tests/fixtures" / path).read_text())
            for key in keys:
                self.assertEqual(data[key], original[key])
        self.assertEqual(len(data["sources"]), 8)

    def test_epoch_and_day_vectors_are_not_interpolated(self):
        data = numerical.prepare()
        for row in data["teme"]:
            source = numerical.ROOT / f"tools/fixtures/m2-numerical/vallado/{row['norad_id']:05}.e"
            vector = numerical.ephemeris(source.read_text())[row["minutes"] * 60]
            self.assertEqual(vector, row["position_km"] + row["velocity_km_s"])
            self.assertEqual(len(row["tle"][0]), 69)
            self.assertEqual(len(row["tle"][1]), 69)

    def test_bad_ephemerides_rejected(self):
        original = (numerical.ROOT / "tools/fixtures/m2-numerical/vallado/00005.e").read_text()
        first = original.split("EphemerisTimePosVel", 1)[1].strip().splitlines()[0]
        for bad in [original.replace("TEME", "J2000"),
                    original.replace("Kilometers", "Meters"),
                    original.replace(first, first + "\n" + first),
                    original.replace("7022.46529266", "nan"),
                    original.replace("7022.46529266", "")]:
            with self.subTest(bad=bad[:30]), self.assertRaises(ValueError):
                numerical.ephemeris(bad)

    def test_missing_required_time_fails_instead_of_computing(self):
        original = numerical.ephemeris
        def missing(text):
            rows = copy.deepcopy(original(text))
            del rows[21600]
            return rows
        with patch.object(numerical, "ephemeris", missing), self.assertRaises(KeyError):
            numerical.prepare()


if __name__ == "__main__":
    unittest.main()
