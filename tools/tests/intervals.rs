//! Full fixed-suite executable coverage; no network, Python, or hardware.
use std::process::Command;

use serde_json::Value;

const BIN: &str = env!("CARGO_BIN_EXE_overhead-evaluate-intervals");

#[test]
fn help_and_invalid_arguments_do_not_run_an_experiment() {
    let help = Command::new(BIN).arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("Usage:"));
    assert!(help.stderr.is_empty());
    let invalid = Command::new(BIN).arg("unexpected").output().unwrap();
    assert_eq!(invalid.status.code(), Some(1));
    assert!(invalid.stdout.is_empty());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("unexpected arguments"));
}

#[test]
fn full_experiment_preserves_mixed_orbit_coverage_and_reports_misses_as_data() {
    let output = Command::new(BIN).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert!(report["scope"].as_str().unwrap().contains("same-model"));
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 15);
    // Historical case expectations, not claims for other epochs or observers.
    let crossings = [14, 8, 4, 2, 2, 2, 0, 0, 1, 2, 2, 2];
    let mut misses = 0;
    for (i, case) in cases.iter().enumerate() {
        let reference = case["reference_crossings"].as_u64().unwrap();
        assert_eq!(
            case["reference_coarser_grid_comparison"]["matched_crossings"],
            reference
        );
        let rows = case["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 42);
        if i < crossings.len() {
            assert_eq!(reference, crossings[i]);
        }
        for row in rows {
            assert!(
                row["evaluations"].as_u64().unwrap()
                    <= report["search_evaluation_guard"].as_u64().unwrap()
            );
            assert!(
                row["max_bracket_width_ms"].as_f64().unwrap()
                    <= row["tolerance_ms"].as_f64().unwrap()
            );
            let comparison = &row["comparison"];
            for key in [
                "unmatched_candidate_crossings",
                "ambiguous_reference_crossings",
                "ambiguous_candidate_crossings",
            ] {
                assert_eq!(comparison[key], 0, "{}: {row}", case["name"]);
            }
            misses += comparison["missed_crossings"].as_u64().unwrap();
            if i < crossings.len() {
                assert_eq!(comparison["matched_crossings"], row["reference_crossings"]);
                // All-above and all-below GEO cases are not interchangeable
                // just because neither has a threshold crossing.
                if i == 6 {
                    assert_eq!(row["passes"], 0);
                }
                if i == 7 {
                    assert_eq!(row["passes"], 1);
                }
            }
        }
    }
    assert!(
        misses > 0,
        "successful evaluation must not hide detection misses"
    );
}
