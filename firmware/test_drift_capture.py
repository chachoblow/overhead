import tempfile
import unittest
from pathlib import Path

import drift_capture


class FitTest(unittest.TestCase):
    def test_recovers_rate_with_late_host_jitter(self):
        ppm = 12.5
        points = []
        for i in range(2000):
            host = 1_000.0 + i
            jitter = (i * 7919 % 13) * 1e-3  # host stamps only ever late
            points.append((host + jitter, 5.0 + i * (1 + ppm * 1e-6)))
        result = drift_capture.fit(points)
        self.assertAlmostEqual(result["ppm"], ppm, delta=0.2)
        self.assertAlmostEqual(result["envelope_ppm"], ppm, delta=0.2)

    def test_parse_rejects_gap_and_keeps_order(self):
        with tempfile.TemporaryDirectory() as d:
            log = Path(d) / "x.log"
            log.write_text("1000000000 boot\n2000000000 DRIFT seq=1 uptime_us=1000000\n"
                           "3000000000 DRIFT seq=2 uptime_us=2000001\n")
            self.assertEqual(drift_capture.parse(log), [(2.0, 1.0), (3.0, 2.000001)])
            log.write_text("2000000000 DRIFT seq=1 uptime_us=1\n4000000000 DRIFT seq=3 uptime_us=3\n")
            with self.assertRaises(ValueError):
                drift_capture.parse(log)

    def test_offsets_interpolate_and_correct_host_clock(self):
        with tempfile.TemporaryDirectory() as d:
            log = Path(d) / "o.log"
            log.write_text("1970-01-01T00:00:00Z\n+0.010000 +/- 0.03 a 1.2.3.4\n"
                           "+0.012000 +/- 0.03 b ::1\n1970-01-01T00:16:40Z\n"
                           "-0.009000 +/- 0.03 a 1.2.3.4\n")
            offsets = drift_capture.parse_offsets(log)
            self.assertEqual(offsets, [(0.0, 0.011), (1000.0, -0.009)])
            corrected = drift_capture.correct([(500.0, 1.0), (2000.0, 2.0)], offsets)
            self.assertEqual(len(corrected), 1)
            self.assertAlmostEqual(corrected[0][0], 500.001)

    def test_sntp_summary_counts_checks_and_rate(self):
        lines = []
        for i in range(1, 5):
            mid = 13_000_000 + i * 64_000_000
            utc = 1_793_000_000_000_000 + i * 64_000_064  # S3 1 ppm slow vs UTC
            lines.append(f"SNTP seq={i} server=a outcome=Observation rtt_us={20000 + i} "
                         f"u0_us={25000 + i} mid_uptime_us={mid} utc_us={utc} budget_ok=true")
            if i > 1:
                lines.append(f"SNTP_CHECK seq={i} diff_us={(-1) ** i * 1000} bound_us=50000 within=true")
        lines.append("SNTP seq=5 server=a outcome=Timeout")
        result = drift_capture.summarize_sntp("\n".join(lines))
        self.assertEqual((result["observations"], result["checks"], result["checks_within"]), (4, 3, 3))
        self.assertEqual(result["outcomes"], {"Timeout": 1})
        self.assertAlmostEqual(result["s3_rate_ppm_vs_utc"], -1.0, delta=0.01)


if __name__ == "__main__":
    unittest.main()
