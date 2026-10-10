import copy
import hashlib
import json
import unittest

from prepare_m2_cases import DEST, archive


class CandidateInputs(unittest.TestCase):
    def setUp(self):
        self.pinned = json.loads(DEST.read_bytes())
        self.docs = [{"key": d["key"], "text": d["text"]}
                     for d in self.pinned["documents"]]

    def test_exact_archive_and_hashes(self):
        self.assertEqual(archive(self.docs), self.pinned)
        for doc in self.pinned["documents"]:
            raw = doc["text"].encode()
            self.assertEqual(doc["bytes"], len(raw))
            self.assertEqual(doc["sha256"], hashlib.sha256(raw).hexdigest())

    def test_reject_missing_duplicate_or_changed_malformed(self):
        with self.assertRaises(ValueError):
            archive(self.docs[:-1])
        duplicate = copy.deepcopy(self.docs)
        duplicate[-1] = duplicate[0]
        with self.assertRaises(ValueError):
            archive(duplicate)
        malformed = copy.deepcopy(self.docs)
        malformed[-1]["text"] = "[]"
        with self.assertRaises(ValueError):
            archive(malformed)

    def test_raw_pool_subset_objects_are_preserved(self):
        from prepare_m2_cases import ROOT
        # The pool extractor already validates lexical object preservation; here
        # assert exact subset construction (not merely parsed numeric equality).
        pool = (ROOT / "tools/fixtures/m2-pool/pool.json").read_text()
        decoder = json.JSONDecoder()
        objects = []
        index = 1
        while True:
            while pool[index].isspace() or pool[index] == ",":
                index += 1
            if pool[index] == "]":
                break
            _, end = decoder.raw_decode(pool, index)
            objects.append(pool[index:end])
            index = end
        by_key = {doc["key"]: doc["text"] for doc in self.docs}
        for size in [5, 9, 12, 16]:
            self.assertEqual(by_key[f"mixed-{size}"], "[\n" + ",\n".join(objects[:size]) + "\n]\n")
        for offset, key in [(0, "leo-heavy"), (1, "deep-space-heavy")]:
            indices = list(range(offset, 16, 2)) + list(range(1-offset, 4, 2))
            self.assertEqual(by_key[key], "[\n" + ",\n".join(objects[i] for i in indices) + "\n]\n")


if __name__ == "__main__":
    unittest.main()
