#!/usr/bin/env python3
"""Reconstruct the candidate M2 pool from pinned source bytes, entirely offline.

No orbit values are changed or reserialized: selected JSON objects are copied
verbatim. Rust's overhead-m2-pool-preflight must verify actual SGP4 paths/searches.
This is NOT the capture-manifest generator. See fixtures/m2-pool/README.md.
"""
import argparse
import datetime as dt
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent / "fixtures" / "m2-pool"


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def raw_records(data):
    text = data.decode("utf-8")
    decoder = json.JSONDecoder(object_pairs_hook=unique_object)
    # Validate the complete document before extracting individual raw objects.
    records = decoder.decode(text)
    if not isinstance(records, list) or not all(isinstance(r, dict) for r in records):
        raise ValueError("expected an array of objects")
    pos = text.index("[") + 1
    result = []
    for record in records:
        while text[pos].isspace():
            pos += 1
        parsed, end = decoder.raw_decode(text, pos)
        assert parsed == record
        result.append((record, text[pos:end]))
        pos = end
        while text[pos].isspace():
            pos += 1
        if text[pos] == ",":
            pos += 1
    return result


def eligible(record, population, start):
    epoch = dt.datetime.fromisoformat(record["EPOCH"]).replace(tzinfo=dt.timezone.utc)
    if abs((start - epoch).total_seconds()) > 48 * 3600:
        return False
    n, e = record["MEAN_MOTION"], record["ECCENTRICITY"]
    # Selection filters only, NOT SGP4 branch assertions. Low-eccentricity LEO,
    # Molniya-like HEO, GPS-like GNSS, and near-geosynchronous GEO respectively.
    return {
        "leo": 12 < n < 18 and 0 <= e < 0.05,
        "heo": 1.89 < n < 2.12 and 0.5 <= e < 1,
        "gnss": 1.89 < n < 2.12 and 0 <= e < 0.5,
        "geo": 0.8 < n < 1.2 and 0 <= e < 0.05,
    }[population]


def generate(root=ROOT):
    acquisition = json.loads((root / "acquisition.json").read_bytes())
    start = dt.datetime.fromisoformat(acquisition["start_utc"])
    selected = {}
    sources = []
    for source in acquisition["sources"]:
        data = (root / source["path"]).read_bytes()
        if len(data) != source["bytes"] or hashlib.sha256(data).hexdigest() != source["sha256"]:
            raise ValueError(f"source hash/length mismatch: {source['path']}")
        records = raw_records(data)
        ids = [r["NORAD_CAT_ID"] for r, _ in records]
        if len(ids) != len(set(ids)):
            raise ValueError(f"duplicate source identities: {source['path']}")
        population = source["population"]
        candidates = sorted(
            ((r, raw, i) for i, (r, raw) in enumerate(records)
             if eligible(r, population, start)),
            key=lambda item: item[0]["NORAD_CAT_ID"],
        )
        count = {"leo": 8, "heo": 2, "gnss": 2, "geo": 4}[population]
        if len(candidates) < count:
            raise ValueError(f"insufficient {population} candidates; review contract")
        selected[population] = [(r, raw, i, source["path"], population)
                                for r, raw, i in candidates[:count]]
        sources.append(dict(path=source["path"], records=len(records),
                            eligible_ids=[r["NORAD_CAT_ID"] for r, _, _ in candidates]))
    deep = [selected[c][i] for i in range(2) for c in ("heo", "gnss", "geo")]
    deep += selected["geo"][2:]
    pool = [entry for pair in zip(selected["leo"], deep) for entry in pair]
    ids = [r["NORAD_CAT_ID"] for r, *_ in pool]
    if len(pool) != 16 or len(set(ids)) != 16:
        raise ValueError("expected 16 distinct identities")
    payload = ("[\n" + ",\n".join(raw for _, raw, *_ in pool) + "\n]\n").encode()
    if len(payload) > 32768:
        raise ValueError("pool exceeds aggregate JSON ceiling; review contract")
    selection = dict(
        schema_version=1, status="candidate-pool-not-capture-manifest",
        start_utc=acquisition["start_utc"], sources=sources,
        pool_bytes=len(payload), pool_sha256=hashlib.sha256(payload).hexdigest(),
        records=[dict(norad_id=r["NORAD_CAT_ID"], name=r["OBJECT_NAME"],
                      epoch_utc=r["EPOCH"] + "Z", population=c,
                      source_path=path, source_record_index=i,
                      record_sha256=hashlib.sha256(raw.encode()).hexdigest())
                 for r, raw, i, path, c in pool],
    )
    return {"pool.json": payload,
            "selection.json": (json.dumps(selection, indent=2) + "\n").encode()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify without writing")
    args = parser.parse_args()
    for name, data in generate().items():
        path = ROOT / name
        if args.check:
            if path.read_bytes() != data:
                raise SystemExit(f"out of date: {path}")
        else:
            path.write_bytes(data)
        print(f"{'verified' if args.check else 'wrote'} {path}")


if __name__ == "__main__":
    main()
