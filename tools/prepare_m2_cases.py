#!/usr/bin/env python3
"""Offline candidate-input archive; never fetch or edit historical sources."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
DEST = ROOT / "tools/fixtures/m2-cases/inputs.json"


def archive(documents):
    if len(documents) != 10 or len({d["key"] for d in documents}) != 10:
        raise ValueError("expected ten distinct documents")
    rows = []
    for doc in documents:
        raw = doc["text"].encode("utf-8")
        if doc["key"] == "malformed":
            if raw != b"[":
                raise ValueError("unexpected malformed document")
            records = None
        else:
            records = json.loads(raw)
            if not isinstance(records, list):
                raise ValueError("not a record array")
        rows.append({**doc, "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest(),
                     "records": None if records is None else len(records),
                     "input_order": None if records is None else [
                         {"norad_id": r["NORAD_CAT_ID"], "epoch": r["EPOCH"]}
                         for r in records]})
    return {"schema_version": 1, "status": "candidate-inputs-not-frozen-manifest", "documents": rows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    output = subprocess.run(
        ["cargo", "run", "--offline", "--locked", "--quiet", "-p", "overhead-tools",
         "--bin", "overhead-m2-preflight", "--", "--inputs"],
        cwd=ROOT, check=True, stdout=subprocess.PIPE).stdout
    data = (json.dumps(archive(json.loads(output)), indent=2, ensure_ascii=False) + "\n").encode()
    if args.check:
        if DEST.read_bytes() != data:
            raise SystemExit("candidate inputs differ; review before regeneration")
        print("candidate inputs match offline regeneration")
    else:
        DEST.write_bytes(data)
        print(f"wrote {DEST.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
