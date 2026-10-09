//! Verify benchmark workload/accounting/schema, never machine-dependent speed.
use serde_json::Value;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_overhead-benchmark");

#[test]
fn help_and_invalid_options_do_not_produce_measurements() {
    for flag in ["--help", "-h"] {
        let output = Command::new(BIN).arg(flag).output().unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
    }
    for args in [
        vec!["unexpected"],
        vec!["--bogus", "1"],
        vec!["--samples"],
        vec!["--samples", "0"],
        vec!["--samples", "51"],
        vec!["--samples", "-1"],
        vec!["--samples", "no"],
        vec!["--min-sample-ms", "0"],
        vec!["--min-sample-ms", "1001"],
        vec!["--min-sample-ms", "NaN"],
        vec!["--samples", "1", "--samples", "2"],
    ] {
        let output = Command::new(BIN).args(&args).output().unwrap();
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn full_matrix_completes_and_mixed_work_is_the_sum_of_its_slots() {
    let output = Command::new(BIN)
        .args([
            "--samples",
            "1",
            "--min-sample-ms",
            "1",
            "--label",
            "test run",
        ])
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["options"]["label"], "test run");
    let inputs = report["inputs"].as_array().unwrap();
    let ids: Vec<_> = inputs
        .iter()
        .map(|input| input["norad_id"].as_u64().unwrap())
        .collect();
    assert_eq!(ids, [25544, 8195, 24208, 28129]);
    let rows = report["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 62);
    for row in rows {
        let timing = &row["timing"];
        assert_eq!(timing["elapsed_ns"].as_array().unwrap().len(), 1);
        let iterations = timing["iterations_per_sample"].as_u64().unwrap();
        assert!(iterations.is_power_of_two());
        let elapsed = timing["elapsed_ns"][0].as_u64().unwrap();
        let median = timing["median_ns_per_invocation"].as_f64().unwrap();
        assert_eq!(median, elapsed as f64 / iterations as f64);
        assert_eq!(
            timing["min_ns_per_invocation"],
            timing["median_ns_per_invocation"]
        );
        assert_eq!(
            timing["max_ns_per_invocation"],
            timing["median_ns_per_invocation"]
        );
        let work = &timing["work_per_invocation"];
        let slots = row["slots"].as_u64().unwrap();
        if row["operation"] == "search_satellite" {
            assert_eq!(work["searches"], slots);
            let regular =
                row["window_s"].as_u64().unwrap() / row["detection_s"].as_u64().unwrap() + 1;
            assert!(work["state_evaluations"].as_u64().unwrap() >= regular * slots);
            assert!(work["state_evaluations"].as_u64().unwrap() <= 200_000 * slots);
        } else {
            assert_eq!(row["tracking_ticks"], 1441);
            assert_eq!(work["state_evaluations"], 1441 * slots);
            assert_eq!(work["searches"], 0);
            assert_eq!(work["passes"], 0);
        }
    }
    for class in ["LEO", "resonant-HEO", "GEO", "GNSS"] {
        let searches: Vec<_> = rows
            .iter()
            .filter(|r| r["orbit_class"] == class && r["operation"] == "search_satellite")
            .collect();
        assert_eq!(searches.len(), 12);
        for window in [3600, 86400] {
            for interval in [5, 30, 60] {
                let pair: Vec<_> = searches
                    .iter()
                    .filter(|r| r["window_s"] == window && r["detection_s"] == interval)
                    .collect();
                assert_eq!(pair.len(), 2);
                assert_eq!(pair[0]["tolerance_ms"], 250);
                assert_eq!(pair[1]["tolerance_ms"], 5000);
                let a = &pair[0]["timing"]["work_per_invocation"];
                let b = &pair[1]["timing"]["work_per_invocation"];
                assert_eq!(a["passes"], b["passes"]);
                assert!(
                    a["state_evaluations"].as_u64().unwrap()
                        >= b["state_evaluations"].as_u64().unwrap()
                );
            }
        }
    }
    let mixed: Vec<_> = rows
        .iter()
        .filter(|r| r["orbit_class"] == "mixed-repeated-fixtures")
        .collect();
    assert_eq!(mixed.len(), 6);
    for (pair, size) in mixed.chunks(2).zip([4, 16, 64]) {
        assert!(pair.iter().all(|r| r["slots"] == size));
        for row in pair {
            for field in ["state_evaluations", "searches", "passes"] {
                let sum: u64 = rows
                    .iter()
                    .filter(|r| {
                        r["slots"] == 1
                            && r["operation"] == row["operation"]
                            && r["window_s"] == row["window_s"]
                            && r["detection_s"] == row["detection_s"]
                            && r["tolerance_ms"] == row["tolerance_ms"]
                    })
                    .map(|r| r["timing"]["work_per_invocation"][field].as_u64().unwrap())
                    .sum();
                assert_eq!(row["timing"]["work_per_invocation"][field], sum * size / 4);
            }
        }
    }
}
