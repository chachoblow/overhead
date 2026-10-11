"""Synthetic report extraction tests; no target measurements or serial access."""
import csv
import io
import json
import unittest
from unittest.mock import patch

import summarize_m2_capture as report
from generate_m2 import manifest
from test_m2_capture import synthetic


class SummaryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.frozen, cls.digest = manifest()
        cls.measure = synthetic('measure', cls.frozen, cls.digest)
        cls.diagnostic = synthetic('diagnostic', cls.frozen, cls.digest)

    def summarize(self, measures=None):
        return report.summarize(measures or [self.measure] * 2, [self.diagnostic] * 2)

    def test_exact_matrix_and_expected_absent_aggregation(self):
        output = self.summarize()
        summary = json.loads(output['summary.json'])
        self.assertEqual(summary['timed_samples'], 120)
        self.assertEqual(summary['stack_observations'], 252)
        self.assertEqual(summary['numerical_rows_per_diagnostic_boot'], 75)
        self.assertEqual([c['timed_samples'] for c in summary['cases']], [6] * 20)
        self.assertIsNone(summary['cases'][15]['phase_times']['aggregation'])
        samples = list(csv.DictReader(io.StringIO(output['samples.csv'])))
        self.assertEqual(len(samples), 120)
        self.assertEqual([row['aggregation_us'] for row in samples if row['case_id'] == '16'], [''] * 6)
        self.assertTrue(all(row['pipeline_requested_live'] == row['pipeline_occupied_live'] == '0' for row in samples))

    def test_phase_timings_are_not_summed_or_confused_with_pipeline_peaks(self):
        slower = self.measure.replace('PHASE,aggregation,100,', 'PHASE,aggregation,300,')
        summary = json.loads(self.summarize([self.measure, slower])['summary.json'])
        row = summary['cases'][0]
        self.assertEqual(row['phase_times']['aggregation'], {'min_us': 100, 'median_us': 200.0, 'max_us': 300})
        self.assertEqual(row['pipeline']['occupied_peak'], 4016)
        self.assertEqual(row['collections']['entries_length'], 8)
        self.assertNotIn('entries_capacity', row['collections'])

    def test_stack_rows_preserve_boot_pattern_phase_and_depth(self):
        output = self.summarize()
        rows = list(csv.DictReader(io.StringIO(output['stack.csv'])))
        self.assertEqual({(row['boot'], row['pattern']) for row in rows},
                         {('1', '0xA5A5A5A5'), ('1', '0x5A5A5A5A'), ('2', '0xA5A5A5A5'), ('2', '0x5A5A5A5A')})
        self.assertEqual(len([row for row in rows if row['phase'] == 'input-copy']), 16)
        self.assertTrue(all(row['observed_bytes'] == '11436' for row in rows))
        self.assertFalse(any(row['case_id'] == '16' and row['phase'] == 'aggregation' for row in rows))

    def test_invalid_or_partial_logs_never_become_reports(self):
        with self.assertRaises(ValueError):
            self.summarize([self.measure[:-100], self.measure])
        with patch.object(report, 'validate_set', side_effect=ValueError('failed gate')):
            with patch.object(report, 'samples') as parse:
                with self.assertRaises(ValueError):
                    self.summarize()
                parse.assert_not_called()


if __name__ == '__main__':
    unittest.main()
