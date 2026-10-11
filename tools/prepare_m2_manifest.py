#!/usr/bin/env python3
"""Freeze/check the bounded M2 annex offline. Renewal requires full preflight."""
import argparse
import hashlib
import json
from pathlib import Path

from prepare_m2_numerical import prepare as numerical_references

ROOT = Path(__file__).resolve().parents[1]
DEST = ROOT / "tools/fixtures/m2-cases/manifest.json"


def build():
    report = json.loads((ROOT / "docs/evaluations/m2-host-preflight.json").read_text())
    archive = json.loads((ROOT / "tools/fixtures/m2-cases/inputs.json").read_text())
    numerical = json.loads((ROOT / "tools/fixtures/m2-numerical/references.json").read_text())
    if numerical != numerical_references():
        raise ValueError("numerical source/selection mismatch")
    documents = {d["key"]: d for d in archive["documents"]}
    if len(documents) != 10:
        raise ValueError("document count changed")
    cases = []
    if [r["case"]["id"] for r in report["cases"]] != list(range(1, 21)):
        raise ValueError("exact case order must remain 01–20")
    for row in report["cases"]:
        case = row["case"]
        groups = row["groups"]
        if not 1 <= len(groups) <= 4 or case["groups"] != [g["document"] for g in groups]:
            raise ValueError("invalid group selection")
        records = total_bytes = 0
        for i, group in enumerate(groups):
            document = documents[group["document"]]
            raw = document["text"].encode()
            if group["index"] != i or group["sha256"] != hashlib.sha256(raw).hexdigest():
                raise ValueError("group origin/hash mismatch")
            if group["bytes"] != len(raw) or group["records"] != document["records"]:
                raise ValueError("group size mismatch")
            total_bytes += len(raw)
            records += document["records"] or 0
        accepted = len(row["accepted"] or [])
        if not (records == row["input_records"] <= 32 and
                total_bytes == row["input_json_bytes"] <= 32768 and accepted <= 16):
            raise ValueError("case counts/ceilings mismatch")
        if (case["id"] == 16) != (row["accepted"] is None and row["search"] is None):
            raise ValueError("unexpected failed case")
        # Architecture-dependent requested-memory values stay in the hashed host
        # annex, never become expected target heap occupancy/capacities.
        cases.append({k: v for k, v in row.items() if k != "requested_memory"})
    paths = {
        ".gitattributes", "Cargo.lock", "core/Cargo.toml", "firmware/Cargo.toml", "firmware/Cargo.lock",
        "core/tests/propagate.rs", "core/tests/coordinates.rs", "core/tests/observer.rs",
        "firmware/build.rs", "firmware/build_numerical.rs", "firmware/src/numerical.rs",
        "firmware/examples/numerical_expected.rs", "tools/Cargo.toml",
        "tools/src/bin/overhead-m2-preflight.rs", "tools/src/bin/overhead-m2-pool-preflight.rs",
        "tools/prepare_m2_cases.py", "tools/prepare_m2_pool.py", "tools/prepare_m2_numerical.py",
        "tools/prepare_m2_manifest.py", "tools/test_prepare_m2_numerical.py",
        "tools/test_prepare_m2_manifest.py", "tools/tests/m2_preflight.rs",
        "tools/fixtures/m2-cases/inputs.json", "tools/fixtures/m2-numerical/references.json",
        "tools/fixtures/m2-numerical/README.md", "tools/fixtures/catalogue-costs.tle",
        "docs/evaluations/m2-host-preflight.json", "docs/evaluations/m2-pool-suitability.json",
        "docs/evaluations/m2-frozen-manifest.md",
        "docs/decisions/0019-bounded-m2-evidence-preparation.md",
        "docs/decisions/0020-m2-capture-manifest-freeze.md",
    }
    for pattern in ["core/src/**/*.rs", "core/tests/fixtures/*", "tools/src/bin/m2_preflight/*.rs",
                    "tools/fixtures/m2-numerical/vallado/*", "tools/fixtures/m2-pool/*"]:
        paths.update(str(p.relative_to(ROOT)) for p in ROOT.glob(pattern) if p.is_file())
    sources = []
    for path in sorted(paths):
        raw = (ROOT / path).read_bytes()
        sources.append({"path": path, "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()})
    return {
        "schema_version": 1, "manifest_id": "m2-bounded-20-v1", "frozen_date_utc": "2026-10-11",
        "status": "frozen-preparation-not-target-evidence", "sources": sources,
        "internal_heap_bytes": 65536,
        "experimental_ceilings": {"groups": 4, "input_records": 32,
                                  "input_json_bytes": 32768, "accepted_satellites": 16},
        "search_controls": {k: report[k] for k in ["threshold_deg", "detection_s",
                              "crossing_tolerance_s", "per_satellite_allowance"]},
        "measurement_captures": {"boots": 2, "warmups_per_case_per_boot": 1,
                                 "samples_per_case_per_boot": 3, "total_measured_samples": 120},
        "stack_captures": {"boots": 2, "patterns": ["0xA5A5A5A5", "0x5A5A5A5A"],
                           "observations_per_case_pattern_per_boot": 1,
                           "phase_case_ids": {
                               "input-copy": [17, 18, 19, 20],
                               "initialization": list(range(1, 21)),
                               "aggregation": [i for i in range(1, 21) if i != 16],
                               "destruction": list(range(1, 21))}},
        "numerical": {"reference_path": "tools/fixtures/m2-numerical/references.json",
                      "checker_path": "firmware/src/numerical.rs",
                      "rows": {"Teme": 12, "Rotation": 9, "Geodetic": 13, "Geometry": 27,
                               "IssPipeline": 12, "VanguardPipeline": 2},
                      "total_rows": 75, "tolerances": numerical["strict_less_than_tolerances"]},
        "target_representation_contract": "docs/evaluations/m2-frozen-manifest.md",
        "cases": cases,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--freeze", action="store_true", help="explicitly renew after reviewed preflight")
    args = parser.parse_args()
    data = (json.dumps(build(), indent=2, ensure_ascii=False) + "\n").encode()
    if args.check:
        if DEST.read_bytes() != data:
            raise SystemExit("frozen manifest mismatch; stop and renew reviewed preflight, do not silently refresh")
        print("frozen M2 manifest matches inputs, numerical selection, host annex and source hashes")
    else:
        DEST.write_bytes(data)
        print(f"froze {DEST.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
