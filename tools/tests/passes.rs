//! Real executable tests. Cloned/mutated ISS records exercise aggregation and
//! error policy, not independent physical accuracy or production budgets.
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

const ISS: &str = include_str!("../../core/tests/fixtures/iss-25544.json");

fn invoke(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_overhead-passes"))
        .args(args)
        .current_dir(std::env::temp_dir())
        .env("TZ", "Pacific/Honolulu")
        .output()
        .unwrap()
}

struct Input {
    directory: PathBuf,
}
impl Input {
    fn new(records: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let directory = std::env::temp_dir().join(format!(
            "overhead-passes-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("omm.json"), records).unwrap();
        fs::write(
            directory.join("manifest.json"),
            r#"{"groups":[{"name":"test","path":"omm.json"}]}"#,
        )
        .unwrap();
        Self { directory }
    }
    fn args(&self) -> Vec<String> {
        [
            self.directory.join("manifest.json").to_str().unwrap(),
            "2026-10-04T12:43:41.833056Z",
            "39.007",
            "-104.883",
            "2.187",
            "60",
            "3000",
            "9000",
        ]
        .map(str::to_owned)
        .to_vec()
    }
    fn two() -> Self {
        let original: Value = serde_json::from_str(ISS).unwrap();
        let mut second = original[0].clone();
        second["NORAD_CAT_ID"] = json!(42);
        Self::new(&json!([original[0], second]).to_string())
    }
}
impl Drop for Input {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn report(output: Output, code: i32) -> String {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}
fn failure(args: &[String]) {
    let out = invoke(args);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert!(!out.stderr.is_empty());
}

#[test]
fn defaults_report_reproducible_brackets_and_ambiguous_arrivals() {
    let input = Input::two();
    let args = input.args();
    let text = report(invoke(&args), 0);
    assert_eq!(text, report(invoke(&args), 0));
    for expected in [
        "Aggregate: complete",
        "Minimum elevation (deg): 10",
        "crossing tolerance (s): 5",
        "Earliest detected arrival candidates: 2 (ordering unresolved",
        "NORAD 42 pass 1:",
        "NORAD 25544 pass 1:",
        "Pass 1 start: upward crossing [",
        "End: downward crossing [",
        "tolerance met",
        "Fetched at (UTC): unknown",
        "brief excursions/gaps",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    assert!(
        text.find("Prediction NORAD 42").unwrap() < text.find("Prediction NORAD 25544").unwrap()
    );
}

#[test]
fn configured_no_arrival_and_window_spanning_underway_are_distinct() {
    let input = Input::new(ISS);
    for threshold in ["90", "-90"] {
        let mut args = input.args();
        args.extend(["60", threshold, "1"].map(str::to_owned));
        let text = report(invoke(&args), 0);
        assert!(text.contains("none found within the searched window"));
        assert!(text.contains("crossing tolerance (s): 1"));
        if threshold == "-90" {
            assert!(text.contains("in progress at window start; actual start unknown"));
            assert!(text.contains("end unknown beyond window"));
        } else {
            assert!(text.contains("passes: 0; upcoming: 0"));
        }
    }
}

#[test]
fn budgets_report_partial_unsearched_and_zero_work_without_false_no_pass() {
    let input = Input::two();
    let mut args = input.args();
    args[7] = "1".into();
    args.extend(["60", "-90", "5"].map(str::to_owned));
    let text = report(invoke(&args), 2);
    for expected in [
        "Aggregate: INCOMPLETE; evaluations: 1",
        "Catalogue-wide next arrival: unknown",
        "NOT a complete no-pass result",
        "Status: incomplete",
        "Status: unsearched",
        "total allowance exhausted",
        "unknown due to interrupted search",
        "Last regular sample: none",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    args[6] = "0".into();
    args[7] = "100".into();
    let text = report(invoke(&args), 2);
    assert_eq!(text.matches("Status: unsearched").count(), 2);
    assert!(text.contains("satellite allowance exhausted"));
    assert!(text.contains("evaluations: 0"));
    args[6] = "100".into();
    args[7] = "0".into();
    assert_eq!(
        report(invoke(&args), 2)
            .matches("Status: unsearched")
            .count(),
        2
    );
}

#[test]
fn interrupted_refinement_keeps_coarse_candidate_and_phase() {
    let input = Input::new(ISS);
    let mut args = input.args();
    // Existing orbital fixture crosses in this minute; no refinement allowance.
    args[1] = "2026-10-04T15:58:00Z".into();
    args[6] = "2".into();
    args.extend(["60", "10", "1"].map(str::to_owned));
    let text = report(invoke(&args), 2);
    for expected in [
        "Earliest detected arrival candidates: 1",
        "tolerance NOT met",
        "during RisingRefinement",
        "Catalogue-wide next arrival: unknown",
        "unknown due to interrupted search",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
}

#[test]
fn orbital_failure_reports_details_and_continues_later_satellites() {
    let original: Value = serde_json::from_str(ISS).unwrap();
    let mut broken = original[0].clone();
    broken["NORAD_CAT_ID"] = json!(42);
    broken["BSTAR"] = json!(1e300);
    let input = Input::new(&json!([broken, original[0]]).to_string());
    let text = report(invoke(&input.args()), 2);
    assert!(text.contains("orbital propagation:"));
    assert!(text.contains("during Sampling"));
    assert!(text.contains("Status: incomplete; evaluations: 1"));
    assert!(text.contains("Status: complete"));
    assert!(text.contains("NORAD 25544 pass 1:"));
    assert!(text.contains("Catalogue-wide next arrival: unknown"));
}

#[test]
fn input_rejections_and_help() {
    for flag in ["--help", "-h"] {
        assert!(report(invoke(&[flag.into()]), 0).contains("Usage:"));
    }
    failure(&[]);
    let input = Input::new(ISS);
    let args = input.args();
    for (index, bad) in [
        (1, "2026-10-04T00:00:00"),
        (1, "2016-12-31T23:59:60Z"),
        (1, "2101-01-01T00:00:00Z"),
        (2, "NaN"),
        (2, "91"),
        (3, "181"),
        (4, "inf"),
        (5, "0"),
        (5, "-1"),
        (5, "0.5"),
        (5, "9223372036854775807"),
        (6, "-1"),
        (7, "18446744073709551616"),
    ] {
        let mut bad_args = args.clone();
        bad_args[index] = bad.into();
        failure(&bad_args);
    }
    let mut tuning = args.clone();
    tuning.extend(["60", "10", "5"].map(str::to_owned));
    for (index, bad) in [(8, "0"), (9, "91"), (9, "NaN"), (10, "0")] {
        let mut bad_args = tuning.clone();
        bad_args[index] = bad.into();
        failure(&bad_args);
    }
    tuning[1] = "2100-12-31T23:59:59Z".into();
    failure(&tuning);
    failure(&args[..7]);
    let mut extra = args.clone();
    extra.push("60".into());
    failure(&extra);
}

#[test]
fn ingestion_warnings_scope_and_fatal_load_atomicity_are_preserved() {
    let original: Value = serde_json::from_str(ISS).unwrap();
    let input = Input::new(&json!([null, original[0]]).to_string());
    let out = invoke(&input.args());
    assert_eq!(out.status.code(), Some(0));
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("warning: skipped")
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("Scope: accepted catalogue only"));
    assert!(text.contains("omm.json record 2"));
    for invalid in ["[null]", "[]", "[", "{}"] {
        fs::write(input.directory.join("omm.json"), invalid).unwrap();
        failure(&input.args());
    }
    fs::remove_file(input.directory.join("omm.json")).unwrap();
    failure(&input.args());
}
