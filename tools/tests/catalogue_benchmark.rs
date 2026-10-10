//! Run allocation measurements in a child process, never the parallel test harness.
//! No speed assertions, and no assumption host byte sizes equal S3 byte sizes.
use serde_json::Value;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_overhead-catalogue-benchmark");

#[test]
fn cli_rejects_invalid_options_without_partial_measurements() {
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
        vec!["--samples", "1", "--samples", "2"],
        vec!["--label"],
        vec!["--label", "a", "--label", "b"],
        vec!["--help", "--samples", "1"],
    ] {
        let output = Command::new(BIN).args(&args).output().unwrap();
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

fn number(value: &Value) -> u64 {
    value.as_u64().unwrap()
}

#[test]
fn complete_partial_and_unsearched_catalogues_have_stable_work_and_heap() {
    let output = Command::new(BIN)
        .args(["--samples", "2", "--label", "test run"])
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
    assert_eq!(report["start_utc"], "2006-06-26T00:00:00Z");
    let inputs = report["inputs"].as_array().unwrap();
    let ids: Vec<_> = inputs.iter().map(|i| number(&i["norad_id"])).collect();
    assert_eq!(ids, [6251, 8195, 28129, 24208, 28057, 9880, 14128, 28626]);
    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), 8);
    for input in inputs {
        assert!(input["age_at_start_s"].as_f64().unwrap().abs() < 2.0 * 86400.0);
    }
    let rows = report["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 8);
    // Deterministic same-model work, not an independent orbital accuracy oracle.
    let expected = [
        (252, 1),
        (5812, 6),
        (1000, 2),
        (0, 0),
        (500, 3),
        (11616, 13),
        (1000, 2),
        (0, 0),
    ];
    for (index, (row, (evaluations, passes))) in rows.iter().zip(expected).enumerate() {
        let size = if index < 4 { 4 } else { 8 };
        assert_eq!(row["satellites"], size);
        assert_eq!(row["norad_ids"], serde_json::json!(&ids[..size]));
        assert!(number(&row["input_json_bytes"]) > 0);
        let samples = row["samples"].as_array().unwrap();
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0]["work"], samples[1]["work"]);
        for sample in samples {
            let work = &sample["work"];
            assert_eq!(work["satellites"], size);
            assert_eq!(work["evaluations"], evaluations);
            assert_eq!(work["passes"], passes);
            assert_eq!(work["complete"], index % 4 < 2);
            assert_eq!(
                work["searched"],
                match index % 4 {
                    2 => 1,
                    3 => 0,
                    _ => size,
                }
            );
            let init = &sample["initialization"]["heap"];
            let search = &sample["aggregation"]["heap"];
            for phase in ["initialization", "aggregation"] {
                let heap = &sample[phase]["heap"];
                assert_eq!(heap, &samples[0][phase]["heap"]);
                assert!(sample[phase]["elapsed_ns"].is_u64());
                assert!(number(&heap["allocation_calls"]) > 0);
                assert!(number(&heap["peak_extra_bytes"]) >= number(&heap["retained_extra_bytes"]));
                assert!(number(&heap["requested_bytes"]) >= number(&heap["peak_extra_bytes"]));
            }
            assert_eq!(
                number(&sample["pipeline_peak_extra_bytes"]),
                number(&init["peak_extra_bytes"]).max(
                    number(&init["retained_extra_bytes"]) + number(&search["peak_extra_bytes"])
                )
            );
            assert_eq!(
                number(&sample["combined_retained_bytes"]),
                number(&init["retained_extra_bytes"]) + number(&search["retained_extra_bytes"])
            );
        }
    }
    // Zero work still retains one unsearched report per accepted satellite.
    assert!(
        number(&rows[7]["samples"][0]["aggregation"]["heap"]["retained_extra_bytes"])
            > number(&rows[3]["samples"][0]["aggregation"]["heap"]["retained_extra_bytes"])
    );
}
