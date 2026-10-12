//! Real executable tests: no SDL window, network, wall clock, or hardware.
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "overhead-sim-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_overhead-sim"))
            .args(args)
            .current_dir(&self.0)
            .env("TZ", "Pacific/Honolulu")
            .env("SDL_VIDEODRIVER", "not-a-real-driver")
            .output()
            .unwrap()
    }
    fn png(&self, path: &str) -> Vec<u8> {
        let data = fs::read(self.0.join(path)).unwrap();
        assert_eq!(&data[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(u32::from_be_bytes(data[16..20].try_into().unwrap()), 400);
        assert_eq!(u32::from_be_bytes(data[20..24].try_into().unwrap()), 240);
        data
    }
    fn manifest(&self, records: Value) {
        fs::write(
            self.0.join("data.json"),
            serde_json::to_vec(&records).unwrap(),
        )
        .unwrap();
        fs::write(
            self.0.join("catalogue.json"),
            r#"{"groups":[{"name":"test","path":"data.json"}]}"#,
        )
        .unwrap();
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn demo_is_deterministic_from_an_unrelated_directory_without_sdl() {
    let scratch = Scratch::new();
    let args = ["--demo", "--snapshot", "frame.png"];
    let report = success(scratch.run(&args));
    assert!(
        report.contains("catalogue=16 sky=4 visible=4 failed=0"),
        "{report}"
    );
    let expected = scratch.png("frame.png");
    assert_eq!(success(scratch.run(&args)), report);
    assert_eq!(scratch.png("frame.png"), expected);
    success(scratch.run(&["--demo", "--radar-size", "200", "--snapshot", "small.png"]));
    assert_ne!(scratch.png("small.png"), expected);
}

#[test]
fn real_catalogue_zoom_changes_visibility_not_sky_count() {
    let scratch = Scratch::new();
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/weather-gps.json");
    let run = |radius: &str, file: &str| {
        success(scratch.run(&[
            manifest.to_str().unwrap(),
            "2026-10-10T00:00:00Z",
            "39.007",
            "-104.883",
            "2.187",
            "--radius-km",
            radius,
            "--snapshot",
            file,
        ]))
    };
    assert!(run("8000", "wide.png").contains("catalogue=105 sky=22 visible=22 failed=0"));
    assert!(run("2500", "local.png").contains("catalogue=105 sky=22 visible=6 failed=0"));
    assert!(run("50", "empty.png").contains("catalogue=105 sky=22 visible=0 failed=0"));
    assert_ne!(scratch.png("wide.png"), scratch.png("local.png"));
    assert_ne!(scratch.png("local.png"), scratch.png("empty.png"));
}

#[test]
fn malformed_inputs_fail_before_creating_a_snapshot() {
    let scratch = Scratch::new();
    for args in [
        vec!["--demo", "--radius-km", "NaN", "--snapshot", "bad.png"],
        vec![
            "absent.json",
            "2026-10-10T00:00:00Z",
            "0",
            "0",
            "0",
            "--snapshot",
            "bad.png",
        ],
        vec!["--demo", "--snapshot", "missing-directory/bad.png"],
    ] {
        let output = scratch.run(&args);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    assert!(!scratch.0.join("bad.png").exists());
    assert!(success(scratch.run(&["--help"])).contains("Usage:"));
    assert_eq!(scratch.run(&[]).status.code(), Some(1));
    scratch.manifest(json!([]));
    let output = scratch.run(&[
        "catalogue.json",
        "2026-10-10T00:00:00Z",
        "0",
        "0",
        "0",
        "--snapshot",
        "bad.png",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("no usable satellites"));
    assert!(!scratch.0.join("bad.png").exists());
}

#[test]
fn propagation_errors_and_ingestion_warnings_remain_explicit() {
    let scratch = Scratch::new();
    let mut records: Value =
        serde_json::from_str(include_str!("../../core/tests/fixtures/iss-25544.json")).unwrap();
    // Same finite-but-overflowing SGP4 control covered by core regression tests.
    records[0]["MEAN_MOTION"] = json!(1e300);
    scratch.manifest(records);
    let args = [
        "catalogue.json",
        "2026-10-04T12:43:41.833056Z",
        "39.007",
        "-104.883",
        "2.187",
        "--snapshot",
        "error.png",
    ];
    let output = scratch.run(&args);
    assert!(String::from_utf8_lossy(&output.stderr).contains("NORAD 25544: propagation:"));
    assert!(success(output).contains("catalogue=1 sky=0 visible=0 failed=1"));
    scratch.png("error.png");

    let mut records: Value =
        serde_json::from_str(include_str!("../../core/tests/fixtures/iss-25544.json")).unwrap();
    records
        .as_array_mut()
        .unwrap()
        .push(json!({"bad":"record"}));
    scratch.manifest(records);
    let output = scratch.run(&args);
    assert!(String::from_utf8_lossy(&output.stderr).contains("warning: skipped"));
    assert!(success(output).contains("catalogue=1"));
}
