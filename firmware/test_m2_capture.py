"""Synthetic protocol fixtures ONLY: invented memory/timings, not S3 evidence."""
import copy
import unittest
from unittest.mock import patch

import generate_m2
import m2_capture as capture
from prepare_m2_images import frames


def synthetic(mode, frozen, digest):
    lines = [capture.START, f'MANIFEST,{frozen["manifest_id"]},{digest}', f'MODE,{mode}',
             'HEAP,65536,65536,0', 'compiler=rustc synthetic-test-only', capture.HARDWARE]
    diagnostic = mode == 'diagnostic'
    if diagnostic:
        for family, count in frozen['numerical']['rows'].items():
            lines += [f'NUM,{family},{i},Ok(())' for i in range(count)]

    def sample(row, index):
        c = row['case']
        identifier = c['id']
        lines.extend([f'BEGIN,{identifier},{index}', f'ENDPOINT,{identifier},{index}'])
        lines.extend(generate_m2.outcomes(row).splitlines())
        lines.append('ENDPOINT_DONE')
        copied = row['input_json_bytes'] if c['lifetime'] != 'flash-control' else 0
        retained = copied if c['lifetime'] == 'ram-retained' else 0
        phases = [
            ('input-copy', (copied, copied, copied, copied, len(row['groups']) if copied else 0, copied, 0)),
            ('initialization', (copied + 3000, copied + 1000, copied + 3016, copied + 1016, 100, 10000, 0)),
            ('input-release', (copied + 1000, retained + 1000, copied + 1016, retained + 1016, 0, 0, 0)),
        ]
        if row['search']:
            phases.append(('aggregation', (retained + 4000, retained + 2000, retained + 4016, retained + 2016, 30, 6000, 0)))
        last = phases[-1][1]
        phases.append(('destruction', (last[1], 0, last[3], 0, 0, 0, 0)))
        for name, values in phases:
            lines.append(f'PHASE,{name},{0 if diagnostic else 100},' + ','.join(map(str, values)))
        pipe = (max(p[0] for _, p in phases), 0, max(p[2] for _, p in phases), 0,
                sum(p[4] for _, p in phases), sum(p[5] for _, p in phases), 0)
        lines.append('PIPE,whole,0,' + ','.join(map(str, pipe)))
        for g in row['groups']:
            lines.append(f'RAW,{g["index"]},{g["records"] or 0}')
        for g in row['groups']:
            for record in range(g['records'] or 0):
                lines.append(f'RECORD,{g["index"]},{record},{copied + 400},{copied + 416},4,200')
        a = row['accepted'] or []
        s = row['search']
        own = (len(row['diagnostics']['parse']), s['earliest_candidates'] if s else 0, len(a),
               len(row['diagnostics']['core']), sum(len(x['groups']) for x in a),
               len(s['satellites']) if s else 0, s['stored_passes'] if s else 0,
               sum(len(d.get('origins', [])) for d in row['diagnostics']['core']))
        lines.append('OWN,' + ','.join(map(str, own)))
        if diagnostic:
            for name, ids in frozen['stack_captures']['phase_case_ids'].items():
                if identifier in ids:
                    lines.append(f'STACK,{name},4096,65536,4100,60000,50000,11436')
        lines.append(f'SAMPLE_DONE,{identifier},{index}')

    if diagnostic:
        for index, pattern in enumerate(capture.PATTERNS):
            lines.append(f'PROBE,{pattern},1024,9216')
            for row in frozen['cases']:
                sample(row, index)
    else:
        for row in frozen['cases']:
            lines.append(f'WARMUP,{row["case"]["id"]},ok')
            for index in range(3):
                sample(row, index)
    return '\n'.join(lines + [capture.DONE]) + '\n'


class ProtocolTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.frozen, cls.digest = generate_m2.manifest()
        cls.measured = synthetic('measure', cls.frozen, cls.digest)
        cls.diagnostic = synthetic('diagnostic', cls.frozen, cls.digest)

    def validate(self, text, mode=None):
        return capture.validate(text, self.frozen, self.digest, mode)

    def test_complete_synthetic_matrix_and_boot_chatter(self):
        for text, mode in [(self.measured, 'measure'), (self.diagnostic, 'diagnostic')]:
            selected, rows = self.validate('ESP-ROM: boot chatter\n' + text, mode)
            self.assertEqual(selected, mode)
            self.assertEqual(len(rows), 20)
        with patch.object(capture, 'manifest', return_value=(self.frozen, self.digest)):
            capture.validate_set([self.measured] * 2, [self.diagnostic] * 2)

    def test_missing_duplicate_and_reordered_every_record_kind(self):
        for text in [self.measured, self.diagnostic]:
            lines = text.splitlines()
            tags = set()
            for i, line in enumerate(lines):
                tag = line.split(',')[0]
                if tag in tags:
                    continue
                tags.add(tag)
                for changed in (lines[:i] + lines[i+1:], lines[:i] + [line] + lines[i:],
                                lines[:i] + lines[i+1:i+2] + [line] + lines[i+2:]):
                    if changed != lines:
                        with self.subTest(tag=tag), self.assertRaises(ValueError):
                            self.validate('\n'.join(changed))

    def test_corrupt_outcomes_and_unknown_records_fail_closed(self):
        for before, after in [
            ('ACCEPT,6251,', 'ACCEPT,9999,'),
            ('unsupported OMM CENTER_NAME "MARS"', 'unsupported OMM CENTER_NAME "MOON"'),
            ('EccentricityOutOfRange(1.0)', 'EccentricityOutOfRange(2.0)'),
            ('ORIGIN,0,0', 'ORIGIN,0,1'),
            ('WORK,1,', 'WORK,0,'),
            ('FAIL,1,', 'FAIL,0,'),
            ('PHASE,destruction,100,', 'PHASE,destruction,-1,'),
            ('PHASE,initialization,100,', 'PHASE,initialization,0,'),
            ('RAW,0,8', 'RAW,0,0'),
            ('SAMPLE_DONE,1,0', 'SAMPLE_DONE,1,0\nUNKNOWN,1'),
            (capture.DONE, capture.DONE + '\nRAW,0,1'),
            (capture.DONE, 'OVERHEAD_S3_M2_FAILED: OOM\n' + capture.DONE),
            (capture.START, capture.START + '\nESP-ROM: reset'),
        ]:
            self.assertIn(before, self.measured)
            with self.subTest(before=before), self.assertRaises(ValueError):
                self.validate(self.measured.replace(before, after, 1))

    def test_each_memory_field_rejected_when_invalid(self):
        lines = self.measured.splitlines()
        for tag in ('PHASE,input-copy,', 'PHASE,initialization,', 'PHASE,input-release,',
                    'PHASE,aggregation,', 'PHASE,destruction,', 'PIPE,whole,'):
            index = next(i for i, line in enumerate(lines) if line.startswith(tag))
            fields = lines[index].split(',')
            for field in range(3, len(fields)):
                changed = fields.copy()
                changed[field] = '999999'  # Includes forbidden failures and unreleased live bytes.
                mutated = lines.copy()
                mutated[index] = ','.join(changed)
                with self.subTest(tag=tag, field=field), self.assertRaises(ValueError):
                    self.validate('\n'.join(mutated))

    def test_numeric_stack_and_mode_gates(self):
        for before, after in [
            ('NUM,Teme,0,Ok(())', 'NUM,Teme,0,Err(Tolerance)'),
            ('1024,9216', '1024,2048'),
            ('1024,9216', '1024,1000000'),
            ('50000,11436', '0,61436'),
            ('50000,11436', '50000,11440'),
            ('STACK,initialization', 'STACK,input-release'),
            ('PHASE,initialization,0,', 'PHASE,initialization,1,'),
        ]:
            with self.subTest(before=before), self.assertRaises(ValueError):
                self.validate(self.diagnostic.replace(before, after, 1))
        with self.assertRaises(ValueError):
            self.validate(self.measured, 'diagnostic')

    def test_generated_bytes_and_timeouts(self):
        self.assertEqual(generate_m2.DEST.read_text(), generate_m2.generate())
        self.assertGreater(capture.timeout_seconds(self.frozen, 'measure'), 900)
        self.assertGreater(capture.timeout_seconds(self.frozen, 'measure'),
                           capture.timeout_seconds(self.frozen, 'diagnostic'))
        changed = copy.deepcopy(self.frozen)
        changed['cases'][0]['search']['evaluations'] += 10000
        self.assertGreater(capture.timeout_seconds(changed, 'measure'),
                           capture.timeout_seconds(self.frozen, 'measure'))

    def test_frame_inventory_parses_hex_without_claiming_call_chain(self):
        self.assertEqual(frames('42030000 <smoke>:\n42030000: 406136 entry a1, 0x2030\n42030004 <leaf>:\n42030004: 004136 entry a1, 32\n'),
                         [{'symbol': 'smoke', 'entry_bytes': 8240}, {'symbol': 'leaf', 'entry_bytes': 32}])

    def test_cross_boot_differences_cannot_be_hidden_by_internal_repeatability(self):
        # Change every row for case 1 consistently: individually valid but not
        # the same collection observations as the other boot.
        changed = self.measured.replace('RECORD,0,0,400,416,4,200', 'RECORD,0,0,400,416,4,201')
        self.validate(changed)
        with patch.object(capture, 'manifest', return_value=(self.frozen, self.digest)):
            with self.assertRaises(ValueError):
                capture.validate_set([self.measured, changed], [self.diagnostic] * 2)

    def test_freeze_and_generation_failure_precedes_hardware(self):
        with patch.object(capture, 'manifest', side_effect=ValueError('changed source')):
            with patch.object(capture.subprocess, 'Popen') as process:
                with self.assertRaises(ValueError):
                    capture.capture('unused', 'unused', 'unused', 'unused', 'measure')
                process.assert_not_called()


if __name__ == '__main__':
    unittest.main()
