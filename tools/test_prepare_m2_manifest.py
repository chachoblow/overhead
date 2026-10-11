"""Frozen annex fails closed on changed case selection or inputs."""
import copy
import json
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

import prepare_m2_manifest as manifest


class ManifestTests(unittest.TestCase):
    def test_exact_frozen_archive(self):
        self.assertEqual(manifest.build(), json.loads(manifest.DEST.read_text()))

    def test_changed_cases_rejected(self):
        path = manifest.ROOT / "docs/evaluations/m2-host-preflight.json"
        original = json.loads(path.read_text())
        read_text = Path.read_text
        mutations = [
            lambda r: r["cases"].reverse(),
            lambda r: r["cases"][0].update(input_json_bytes=32769),
            lambda r: r["cases"][0]["groups"][0].update(sha256="0" * 64),
            lambda r: r["cases"][0]["case"].update(groups=["malformed"]),
            lambda r: r["cases"][15].update(search={}),
        ]
        for mutate in mutations:
            changed = copy.deepcopy(original)
            mutate(changed)
            def read(p, *args, **kwargs):
                return json.dumps(changed) if p == path else read_text(p, *args, **kwargs)
            with patch.object(Path, "read_text", read), self.assertRaises(ValueError):
                manifest.build()

    def test_changed_source_is_not_silently_accepted(self):
        original = Path.read_bytes
        path = manifest.ROOT / "firmware/src/numerical.rs"
        def read(p, *args, **kwargs):
            raw = original(p, *args, **kwargs)
            return raw + b"\n" if p == path else raw
        with patch.object(Path, "read_bytes", read):
            self.assertNotEqual(manifest.build(), json.loads(manifest.DEST.read_text()))

    def test_cli_is_working_directory_independent(self):
        result = subprocess.run([sys.executable, str(manifest.ROOT / "tools/prepare_m2_manifest.py"), "--check"],
                                cwd="/tmp", capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
