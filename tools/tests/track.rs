//! Exercise the real executable: file/argument parsing, reporting, and errors.
use overhead_core::{GeodeticPosition, sgp4};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

const ISS: &str = include_str!("../../core/tests/fixtures/iss-25544.json");
const T0: &str = "2026-10-04T12:43:41.833056Z";

fn fixture() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../core/tests/fixtures/iss-25544.json")
        .to_str()
        .unwrap()
        .into()
}

fn invoke(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_overhead-track"))
        .args(args)
        .env("TZ", "Pacific/Honolulu")
        .output()
        .unwrap()
}

fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    String::from_utf8(output.stdout).unwrap()
}

fn failure(args: &[&str], expected: &str) {
    let output = invoke(args);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains(expected), "{args:?}: {error}");
}

fn field<'a>(report: &'a str, label: &str) -> &'a str {
    report
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{label}: ")))
        .unwrap()
}

fn number(report: &str, label: &str) -> f64 {
    field(report, label).parse().unwrap()
}

#[test]
fn twelve_cli_pipelines_match_independent_skyfield_references() {
    let refs: Value =
        serde_json::from_str(include_str!("../../core/tests/fixtures/observer.json")).unwrap();
    let cases = refs["iss_pipeline"].as_array().unwrap();
    assert_eq!(cases.len(), 12);
    let path = fixture();
    for case in cases {
        let utc = format!("{}Z", case["utc"].as_str().unwrap());
        let observer = &case["observer"];
        let lat = observer["latitude_deg"].to_string();
        let lon = observer["longitude_deg"].to_string();
        let height = observer["altitude_km"].to_string();
        let args = [path.as_str(), &utc, &lat, &lon, &height];
        let report = success(invoke(&args));
        assert_eq!(
            report,
            success(invoke(&args)),
            "report must be reproducible"
        );
        assert_eq!(field(&report, "Satellite"), "ISS (ZARYA)");
        assert_eq!(field(&report, "NORAD ID"), "25544");
        assert_eq!(field(&report, "Element epoch (UTC)"), T0);
        assert_eq!(field(&report, "Requested time (UTC)"), utc);
        let range = number(&report, "Slant range (km)");
        let az = number(&report, "Azimuth (deg clockwise from true north)");
        let el = number(&report, "Elevation (deg)");
        assert!(
            (range - case["range_km"].as_f64().unwrap()).abs() < 0.1,
            "{report}"
        );
        let az_error =
            (az - case["azimuth_deg"].as_f64().unwrap() + 180.0).rem_euclid(360.0) - 180.0;
        assert!(az_error.abs() < 0.01, "{report}");
        assert!(
            (el - case["elevation_deg"].as_f64().unwrap()).abs() < 0.01,
            "{report}"
        );
        // Reporting check: geodetic units/order reconstruct the printed ECEF.
        // Independent transform accuracy is separately covered by core tests.
        let geo = GeodeticPosition {
            latitude_rad: number(&report, "Satellite WGS-84 latitude (deg)").to_radians(),
            longitude_rad: number(&report, "Satellite WGS-84 longitude (deg east)").to_radians(),
            altitude_km: number(&report, "Satellite WGS-84 ellipsoidal altitude (km)"),
        };
        let printed: Vec<f64> = field(&report, "ECEF position (km; GMST-only)")
            .split_whitespace()
            .map(|s| s.parse().unwrap())
            .collect();
        assert_eq!(printed.len(), 3);
        for (actual, expected) in geo.to_ecef().unwrap().km().iter().zip(printed) {
            assert!((actual - expected).abs() < 1e-6);
        }
    }
}

#[test]
fn help_and_wrong_argument_counts() {
    for flag in ["--help", "-h"] {
        assert!(success(invoke(&[flag])).contains("Usage:"));
    }
    failure(&[], "expected five arguments");
    failure(&[&fixture(), T0, "0", "0"], "expected five arguments");
    failure(
        &[&fixture(), T0, "0", "0", "0", "extra"],
        "expected five arguments",
    );
}

#[test]
fn invalid_times_and_observers_fail_without_reports() {
    let path = fixture();
    for time in [
        "2026-10-04T12:43:41.833056",
        "2026-10-04T12:43:41+01:00",
        "2026-02-30T00:00:00Z",
        "1956-12-31T23:59:59Z",
        "2101-01-01T00:00:00Z",
        "2016-12-31T23:59:60Z",
        "2016-12-31T23:59:60.5Z",
        "garbageZ",
    ] {
        failure(&[&path, time, "0", "0", "0"], "UTC");
    }
    for (lat, lon, height, error) in [
        ("91", "0", "0", "latitude"),
        ("-91", "0", "0", "latitude"),
        ("0", "181", "0", "longitude"),
        ("0", "-181", "0", "longitude"),
        ("NaN", "0", "0", "finite"),
        ("0", "inf", "0", "finite"),
        ("0", "0", "-inf", "finite"),
        ("0", "0", "bad", "height"),
    ] {
        failure(&[&path, T0, lat, lon, height], error);
    }
}

#[test]
fn boundary_locations_and_pre_epoch_time_are_accepted() {
    for (lat, lon, height) in [("90", "180", "0"), ("-90", "-180", "-0.4")] {
        let report = success(invoke(&[
            &fixture(),
            "2026-10-04T12:42:41.833056Z",
            lat,
            lon,
            height,
        ]));
        assert_eq!(number(&report, "Minutes since element epoch (min)"), -1.0);
    }
    success(invoke(&[&fixture(), "2026-10-04T12:43:41Z", "0", "0", "0"]));
}

// No additional test dependency: each test owns a unique temporary directory.
struct TempFile {
    directory: PathBuf,
    path: String,
}
impl TempFile {
    fn new(contents: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let directory = std::env::temp_dir().join(format!(
            "overhead-track-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("input.json");
        fs::write(&path, contents).unwrap();
        Self {
            directory,
            path: path.to_str().unwrap().into(),
        }
    }
}
impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn file_json_cardinality_and_element_errors_are_reported() {
    let temp = TempFile::new(ISS);
    fs::remove_file(&temp.path).unwrap();
    failure(&[&temp.path, T0, "0", "0", "0"], "cannot read");
    let mut invalid: Value = serde_json::from_str(ISS).unwrap();
    invalid[0]["MEAN_MOTION"] = json!(0);
    let original: Value = serde_json::from_str(ISS).unwrap();
    for (data, error) in [
        ("not json".to_string(), "invalid OMM JSON"),
        ("{}".to_string(), "invalid OMM JSON"),
        ("[]".to_string(), "found 0"),
        (json!([original[0], original[0]]).to_string(), "found 2"),
        (invalid.to_string(), "invalid elements"),
    ] {
        let temp = TempFile::new(&data);
        failure(&[&temp.path, T0, "0", "0", "0"], error);
    }
}

#[test]
fn incompatible_omm_metadata_fails_without_reports() {
    for (field, unsupported) in [
        ("CENTER_NAME", "MARS"),
        ("REF_FRAME", "GCRF"),
        ("TIME_SYSTEM", "TAI"),
        ("MEAN_ELEMENT_THEORY", "DSST"),
    ] {
        let mut data: Value = serde_json::from_str(ISS).unwrap();
        data[0][field] = json!(unsupported);
        let temp = TempFile::new(&data.to_string());
        failure(&[&temp.path, T0, "0", "0", "0"], field);
    }
}

#[test]
fn explicit_supported_omm_metadata_preserves_the_report() {
    let baseline = success(invoke(&[&fixture(), T0, "0", "0", "0"]));
    let mut data: Value = serde_json::from_str(ISS).unwrap();
    for (field, value) in [
        ("CENTER_NAME", "EARTH"),
        ("REF_FRAME", "TEME"),
        ("TIME_SYSTEM", "UTC"),
        ("MEAN_ELEMENT_THEORY", "SGP4"),
    ] {
        data[0][field] = json!(value);
    }
    let temp = TempFile::new(&data.to_string());
    assert_eq!(success(invoke(&[&temp.path, T0, "0", "0", "0"])), baseline);
}

#[test]
fn non_finite_propagation_fails_before_coordinate_conversion() {
    for field in ["MEAN_MOTION", "BSTAR"] {
        let mut data: Value = serde_json::from_str(ISS).unwrap();
        data[0][field] = json!(1e300);
        let temp = TempFile::new(&data.to_string());
        failure(&[&temp.path, T0, "0", "0", "0"], "propagation failed");
    }
}

#[test]
fn unsupported_element_epochs_fail_without_reports() {
    for epoch in [
        "1956-12-31T23:59:59.999999999",
        "2101-01-01T00:00:00",
        "2016-12-31T23:59:60.5",
    ] {
        let mut data: Value = serde_json::from_str(ISS).unwrap();
        data[0]["EPOCH"] = json!(epoch);
        let temp = TempFile::new(&data.to_string());
        failure(&[&temp.path, T0, "0", "0", "0"], "invalid elements");
    }
}

#[test]
fn propagation_failure_is_reported_without_partial_output() {
    // Vallado 33334 fails at its epoch; same reference as core's divergence test.
    let elements = sgp4::Elements::from_tle(
        None,
        b"1 33334U 78066F   06174.85818871  .00000620  00000-0  10000-3 0  6806",
        b"2 33334  68.4714 236.1303 5602877 123.7484 302.5767  0.00001000 67521",
    )
    .unwrap();
    let utc = format!("{}T{}Z", elements.datetime.date(), elements.datetime.time());
    let temp = TempFile::new(&serde_json::to_string(&[elements]).unwrap());
    failure(&[&temp.path, &utc, "0", "0", "0"], "propagation failed");
}
