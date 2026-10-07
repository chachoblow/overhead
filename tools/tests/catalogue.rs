//! Exercise the real offline executable, including raw-record parsing and
//! failure atomicity. Synthetic mutations test policy, not orbital accuracy.
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

const ISS: &str = include_str!("../../core/tests/fixtures/iss-25544.json");

fn object() -> Value {
    serde_json::from_str::<Value>(ISS).unwrap()[0].clone()
}

fn invoke(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_overhead-catalogue"))
        .args(args)
        .current_dir(std::env::temp_dir())
        .env("TZ", "Pacific/Honolulu")
        .output()
        .unwrap()
}

fn success(output: Output) -> (String, String) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

fn failure(output: Output, expected: &str) -> String {
    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "failed load published partial stdout"
    );
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains(expected), "expected {expected:?}: {error}");
    error
}

struct Input {
    directory: PathBuf,
    manifest: String,
}

impl Input {
    fn new(groups: Value) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let directory = std::env::temp_dir().join(format!(
            "overhead-catalogue-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let manifest = directory
            .join("catalogue.json")
            .to_str()
            .unwrap()
            .to_owned();
        fs::write(&manifest, json!({"groups": groups}).to_string()).unwrap();
        Self {
            directory,
            manifest,
        }
    }

    fn one() -> Self {
        let input = Self::new(json!([{"name": "stations", "path": "stations.json"}]));
        input.write("stations.json", ISS);
        input
    }

    fn write(&self, name: &str, data: &str) {
        fs::write(self.directory.join(name), data).unwrap();
    }

    fn run(&self) -> Output {
        invoke(&[&self.manifest])
    }
}

impl Drop for Input {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn help_arguments_and_unreadable_manifest() {
    for flag in ["--help", "-h"] {
        let (report, errors) = success(invoke(&[flag]));
        assert!(report.contains("Usage:"));
        assert!(errors.is_empty());
    }
    failure(invoke(&[]), "expected one manifest argument");
    failure(invoke(&["a", "b"]), "expected one manifest argument");
    let input = Input::one();
    fs::remove_file(&input.manifest).unwrap();
    failure(input.run(), "cannot read manifest");
}

#[test]
fn checked_in_example_runs_from_an_unrelated_working_directory() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/catalogue.json");
    let (report, errors) = success(invoke(&[path.to_str().unwrap()]));
    assert!(errors.is_empty());
    assert!(report.contains("Catalogue: 1 satellites"));
    assert!(report.contains("NORAD ID: 25544"));
    assert!(report.contains("Groups: iss-fixture"));
}

#[test]
fn relative_paths_and_unknown_provenance_are_explicit_and_reproducible() {
    let input = Input::one();
    let output = success(input.run());
    assert_eq!(output, success(input.run()));
    let (report, errors) = output;
    assert!(errors.is_empty());
    assert!(report.contains("Element epoch (UTC): 2026-10-04T12:43:41.833056Z"));
    assert!(report.contains("Selected from: group \"stations\", stations.json record 1"));
    assert!(report.contains("Source URL: unknown"));
    assert!(report.contains("Fetched at (UTC): unknown"));
}

#[test]
fn merge_uses_element_epoch_not_fetch_time_and_retains_group_membership() {
    let input = Input::new(json!([
        {"name": "stations", "path": "a.json", "fetched_at": "2026-10-07T00:00:00Z"},
        {"name": "brightest", "path": "b.json", "fetched_at": "2026-10-06T00:00:00Z", "source_url": "https://example.invalid/brightest"}
    ]));
    input.write("a.json", ISS);
    let mut newer = object();
    newer["EPOCH"] = json!("2026-10-05T00:00:00");
    newer["OBJECT_NAME"] = json!("Newer orbit");
    input.write("b.json", &json!([newer, newer]).to_string());
    let (report, errors) = success(input.run());
    assert!(errors.is_empty());
    assert!(report.contains("Catalogue: 1 satellites"));
    assert!(report.contains("Groups: stations, brightest"));
    assert!(report.contains("Satellite: Newer orbit"));
    assert!(report.contains("Selected from: group \"brightest\", b.json record 1"));
    assert!(report.contains("Fetched at (UTC): 2026-10-06T00:00:00Z"));
    assert!(report.contains("Source URL: https://example.invalid/brightest"));
}

#[test]
fn invalid_records_are_skipped_and_do_not_evict_a_valid_record() {
    let input = Input::new(json!([
        {"name": "valid", "path": "a.json"}, {"name": "invalid", "path": "b.json"}
    ]));
    input.write("a.json", ISS);
    let mut bad_orbit = object();
    bad_orbit["MEAN_MOTION"] = json!(0);
    bad_orbit["EPOCH"] = json!("2026-10-06T00:00:00");
    let mut bad_metadata = object();
    bad_metadata["REF_FRAME"] = json!("GCRF");
    let mut bad_epoch = object();
    bad_epoch["EPOCH"] = json!("2101-01-01T00:00:00");
    input.write(
        "b.json",
        &json!([bad_orbit, bad_metadata, null, {}, bad_epoch]).to_string(),
    );
    let (report, errors) = success(input.run());
    assert!(report.contains("Catalogue: 1 satellites"));
    assert!(report.contains("Groups: valid\n"));
    assert_eq!(errors.matches("warning: skipped").count(), 5);
    assert!(errors.contains("mean motion 0"));
    assert!(errors.contains("REF_FRAME"));
    assert!(errors.contains("epoch year 2101"));
    for record in 1..=5 {
        assert!(
            errors.contains(&format!("b.json record {record}")),
            "{errors}"
        );
    }
}

#[test]
fn raw_record_parsing_preserves_duplicate_metadata_and_orbital_keys() {
    let input = Input::one();
    let good = serde_json::to_string(&object()).unwrap();
    for prefix in [
        "\"REF_FRAME\":\"GCRF\",\"REF_FRAME\":\"TEME\",",
        "\"REF_FRAME\":\"TEME\",\"REF_FRAME\":\"TEME\",",
        "\"CENTER_NAME\":null,",
        "\"TIME_SYSTEM\":42,",
        "\"MEAN_ELEMENT_THEORY\":\"DSST\",",
        "\"MEAN_MOTION\":15.48731752,",
    ] {
        let bad = format!("{{{prefix}{}", &good[1..]);
        input.write("stations.json", &format!("[{bad},{good}]"));
        let (report, errors) = success(input.run());
        assert!(report.contains("record 2"));
        assert_eq!(errors.matches("warning: skipped").count(), 1, "{errors}");
        assert!(errors.contains("stations.json record 1"));
    }
}

#[test]
fn same_epoch_conflict_omits_only_that_satellite_and_identifies_both_inputs() {
    let input = Input::new(json!([
        {"name": "a", "path": "a.json"}, {"name": "b", "path": "b.json"}
    ]));
    let mut other = object();
    other["NORAD_CAT_ID"] = json!(42);
    input.write("a.json", &json!([object(), other]).to_string());
    let mut different = object();
    different["MEAN_ANOMALY"] = json!(140);
    input.write("b.json", &json!([different]).to_string());
    let (report, errors) = success(input.run());
    assert!(report.contains("Catalogue: 1 satellites"));
    assert!(report.contains("NORAD ID: 42"));
    assert!(!report.contains("NORAD ID: 25544"));
    assert!(errors.contains("omitted NORAD 25544"));
    assert!(errors.contains("conflicting orbital values at newest valid epoch"));
    assert!(errors.contains("a.json record 1"));
    assert!(errors.contains("b.json record 1"));
}

#[test]
fn equivalent_numeric_records_do_not_conflict_over_names_or_json_formatting() {
    let input = Input::one();
    let mut alternate = object();
    alternate["OBJECT_NAME"] = json!("Renamed");
    alternate["MEAN_MOTION"] = json!("15.48731752");
    alternate["COMMENT"] = json!("ignored");
    input.write(
        "stations.json",
        &serde_json::to_string_pretty(&json!([object(), alternate])).unwrap(),
    );
    let (report, errors) = success(input.run());
    assert!(errors.is_empty());
    assert!(report.contains("Catalogue: 1 satellites"));
    assert!(report.contains("Satellite: ISS (ZARYA)"));
    assert!(report.contains("stations.json record 1"));
}

#[test]
fn failed_later_document_never_publishes_a_partial_catalogue() {
    let input = Input::new(json!([
        {"name": "good", "path": "a.json"}, {"name": "broken", "path": "b.json"}
    ]));
    input.write("a.json", ISS);
    failure(input.run(), "cannot read group \"broken\"");
    for bad in [
        "not json",
        "{}",
        "null",
        "[",
        "[{},]",
        "[] trailing",
        "[{\"bad\":]",
    ] {
        input.write("b.json", bad);
        failure(
            input.run(),
            "invalid OMM JSON document for group \"broken\"",
        );
    }
    // A truncated document is fatal even if it begins with a valid record.
    input.write("b.json", &format!("[{},", object()));
    failure(input.run(), "invalid OMM JSON document");
}

#[test]
fn empty_groups_are_allowed_but_empty_results_fail_with_rejection_details() {
    let input = Input::new(json!([
        {"name": "empty", "path": "a.json"}, {"name": "other", "path": "b.json"}
    ]));
    input.write("a.json", "[]");
    input.write("b.json", ISS);
    success(input.run());
    input.write("b.json", "[]");
    failure(input.run(), "no usable satellites");
    input.write("b.json", "[null]");
    let error = failure(input.run(), "no usable satellites");
    assert!(error.contains("b.json record 1"));
    let mut bad = object();
    bad["MEAN_MOTION"] = json!(0);
    input.write("b.json", &json!([bad]).to_string());
    let error = failure(input.run(), "no usable satellites");
    assert!(error.contains("mean motion 0"));
    let mut conflict = object();
    conflict["MEAN_ANOMALY"] = json!(140);
    input.write("b.json", &json!([object(), conflict]).to_string());
    let error = failure(input.run(), "no usable satellites");
    assert!(error.contains("omitted NORAD 25544"));
}

#[test]
fn malformed_or_ambiguous_manifest_configuration_fails() {
    let input = Input::one();
    for bad in [
        "not json".to_owned(),
        "[]".to_owned(),
        "{}".to_owned(),
        json!({"groups": [], "typo": true}).to_string(),
        json!({"groups": [{"name": "a", "path": "stations.json", "fetched": "unknown"}]})
            .to_string(),
        "{\"groups\":[],\"groups\":[]}".to_owned(),
    ] {
        input.write("catalogue.json", &bad);
        failure(input.run(), "invalid manifest");
    }
    for groups in [
        json!([]),
        json!([{"name": " ", "path": "stations.json"}]),
        json!([{"name": "a", "path": ""}]),
        json!([{"name": "a", "path": "stations.json", "source_url": " "}]),
        json!([{"name": "a", "path": "stations.json"}, {"name": "a", "path": "stations.json"}]),
    ] {
        input.write("catalogue.json", &json!({"groups": groups}).to_string());
        failure(input.run(), "error:");
    }
}

#[test]
fn fetch_times_are_explicit_utc_and_never_substituted_for_element_epoch() {
    let input = Input::one();
    for time in [
        "garbage",
        "2026-10-06T00:00:00",
        "2026-10-06T00:00:00+00:00",
        "2026-02-30T00:00:00Z",
        "1956-12-31T00:00:00Z",
        "2101-01-01T00:00:00Z",
        "2016-12-31T23:59:60Z",
        "2016-12-31T23:59:60.5Z",
    ] {
        input.write(
            "catalogue.json",
            &json!({"groups": [{"name": "a", "path": "stations.json", "fetched_at": time}]})
                .to_string(),
        );
        failure(input.run(), "invalid fetched_at");
    }
    for time in [Value::Null, json!("2026-10-06T08:00:00.123Z")] {
        input.write(
            "catalogue.json",
            &json!({"groups": [{"name": "a", "path": "stations.json", "fetched_at": time}]})
                .to_string(),
        );
        let (report, errors) = success(input.run());
        assert!(errors.is_empty());
        assert!(report.contains("Element epoch (UTC): 2026-10-04T12:43:41.833056Z"));
        let expected = time.as_str().unwrap_or("unknown");
        assert!(report.contains(&format!("Fetched at (UTC): {expected}")));
    }
}
