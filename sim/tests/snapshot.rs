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
        report.contains("catalogue=16 sky=4 visible=0 failed=0"),
        "{report}"
    );
    let expected = scratch.png("frame.png");
    assert_eq!(success(scratch.run(&args)), report);
    assert_eq!(scratch.png("frame.png"), expected);
    success(scratch.run(&["--demo", "--radar-size", "200", "--snapshot", "small.png"]));
    assert_ne!(scratch.png("small.png"), expected);
    let wide = success(scratch.run(&["--demo", "--fov-deg", "100", "--snapshot", "wide.png"]));
    assert!(wide.contains("catalogue=16 sky=4 visible=4 failed=0"));
    assert_ne!(scratch.png("wide.png"), expected);
}

#[test]
fn real_catalogue_zoom_changes_visibility_not_sky_count() {
    let scratch = Scratch::new();
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/weather-gps.json");
    let run = |fov: &str, viewport: &str, file: &str| {
        success(scratch.run(&[
            manifest.to_str().unwrap(),
            "2026-10-10T00:00:00Z",
            "39.007",
            "-104.883",
            "2.187",
            "--fov-deg",
            fov,
            "--viewport",
            viewport,
            "--snapshot",
            file,
        ]))
    };
    // Review counts independently from physical look angles, not by copying
    // the new executable's output or calling the app's projection.
    let loaded = overhead_tools::catalogue::load(&manifest).unwrap();
    let time = overhead_core::sgp4::chrono::NaiveDate::from_ymd_opt(2026, 10, 10)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let observer = overhead_core::GeodeticPosition {
        latitude_rad: 39.007_f64.to_radians(),
        longitude_rad: (-104.883_f64).to_radians(),
        altitude_km: 2.187,
    };
    let looks: Vec<_> = loaded
        .catalogue
        .entries()
        .iter()
        .map(|entry| {
            let ecef = entry.satellite().state_at(time).unwrap().to_ecef().unwrap();
            overhead_core::ecef_to_look_angles(ecef, observer).unwrap()
        })
        .collect();
    assert_eq!(
        looks.iter().filter(|look| look.elevation_rad > 0.0).count(),
        22
    );
    for (fov, file, expected) in [
        (100.0_f64, "wide.png", 22),
        (20.0, "default.png", 6),
        (40.0, "middle.png", 17),
        (5.0, "local.png", 4),
        (0.1, "empty.png", 0),
    ] {
        let reference = looks
            .iter()
            .filter(|look| {
                let depth = 75_000.0 - look.range_km * look.elevation_rad.sin();
                let horizontal = look.range_km * look.elevation_rad.cos();
                look.elevation_rad > 0.0
                    && depth >= 1.0
                    && horizontal / depth <= (fov.to_radians() / 2.0).tan()
            })
            .count();
        assert_eq!(reference, expected);
        for viewport in ["circle", "borderless-circle", "square"] {
            let count = if viewport == "square" {
                looks
                    .iter()
                    .filter(|look| {
                        let depth = 75_000.0 - look.range_km * look.elevation_rad.sin();
                        let horizontal = look.range_km * look.elevation_rad.cos();
                        let azimuth = look.azimuth_rad.unwrap_or(0.0);
                        let span = depth * (fov.to_radians() / 2.0).tan();
                        look.elevation_rad > 0.0
                            && depth >= 1.0
                            && (horizontal * azimuth.sin()).abs() <= span
                            && (horizontal * azimuth.cos()).abs() <= span
                    })
                    .count()
            } else {
                expected
            };
            if viewport == "square" && fov == 40.0 {
                assert!(count > expected);
            }
            let path = if viewport == "circle" {
                file.to_owned()
            } else {
                format!("{viewport}-{file}")
            };
            let report = run(&fov.to_string(), viewport, &path);
            assert!(
                report.contains(&format!("catalogue=105 sky=22 visible={count} failed=0")),
                "{report}"
            );
            let png = scratch.png(&path);
            assert_eq!(run(&fov.to_string(), viewport, &path), report);
            assert_eq!(scratch.png(&path), png);
        }
    }
    // Omitting --viewport must really select square, including corner markers.
    for (fov, file, expected_count) in [("20", "default.png", 6), ("40", "middle.png", 18)] {
        let report = success(scratch.run(&[
            manifest.to_str().unwrap(),
            "2026-10-10T00:00:00Z",
            "39.007",
            "-104.883",
            "2.187",
            "--fov-deg",
            fov,
            "--snapshot",
            "implicit.png",
        ]));
        assert!(report.contains(&format!(
            "catalogue=105 sky=22 visible={expected_count} failed=0"
        )));
        assert_eq!(
            scratch.png("implicit.png"),
            scratch.png(&format!("square-{file}"))
        );
    }
    // At 20 degrees this frame has no corner-only markers; at 40 it does.
    assert_eq!(
        scratch.png("borderless-circle-default.png"),
        scratch.png("square-default.png")
    );
    assert_ne!(
        scratch.png("borderless-circle-middle.png"),
        scratch.png("square-middle.png")
    );
    assert_ne!(
        scratch.png("default.png"),
        scratch.png("borderless-circle-default.png")
    );
    assert_ne!(scratch.png("wide.png"), scratch.png("default.png"));
    assert_ne!(scratch.png("default.png"), scratch.png("local.png"));
    assert_ne!(scratch.png("local.png"), scratch.png("empty.png"));
}

#[test]
fn wide_snapshots_match_independent_geometry_at_multiple_times_and_zooms() {
    use overhead_core::{
        GeodeticPosition, ecef_to_look_angles,
        sgp4::chrono::{Duration, NaiveDate},
    };
    let scratch = Scratch::new();
    let observer = GeodeticPosition {
        latitude_rad: 39.007_f64.to_radians(),
        longitude_rad: (-104.883_f64).to_radians(),
        altitude_km: 2.187,
    };
    let start = NaiveDate::from_ymd_opt(2026, 10, 10)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    for name in ["catalogue.json", "weather-gps.json"] {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join(name);
        let loaded = overhead_tools::catalogue::load(&manifest).unwrap();
        for seconds in [0, 30] {
            let time = start + Duration::seconds(seconds);
            let utc = format!("{time}Z").replace(' ', "T");
            let looks: Vec<_> = loaded
                .catalogue
                .entries()
                .iter()
                .map(|entry| {
                    let ecef = entry.satellite().state_at(time).unwrap().to_ecef().unwrap();
                    ecef_to_look_angles(ecef, observer).unwrap()
                })
                .collect();
            let sky = looks.iter().filter(|look| look.elevation_rad > 0.0).count();
            for fov in [0.1_f64, 5.0, 20.0, 40.0, 100.0] {
                let mut visible = 0;
                let mut indicators = 0;
                let mut selected_elevation = None;
                for look in &looks {
                    let up = look.range_km * look.elevation_rad.sin();
                    let depth = 75_000.0 - up;
                    if look.elevation_rad <= 0.0 || depth < 1.0 {
                        continue;
                    }
                    let horizontal = look.range_km * look.elevation_rad.cos();
                    let azimuth = look.azimuth_rad.unwrap_or(0.0);
                    // Independent pixel focal length: same 117px vertical half-
                    // extent as square, with the added horizontal area at 173px.
                    let focal_length = 117.0 / (fov.to_radians() / 2.0).tan();
                    let x = horizontal * azimuth.sin() / depth * focal_length;
                    let y = horizontal * azimuth.cos() / depth * focal_length;
                    if x.abs() <= 173.0 && y.abs() <= 117.0 {
                        visible += 1;
                        // Snapshot selection is the first visible NORAD ID.
                        selected_elevation.get_or_insert(look.elevation_rad);
                    } else {
                        indicators += 1;
                    }
                }
                let fov = fov.to_string();
                let args = [
                    manifest.to_str().unwrap(),
                    &utc,
                    "39.007",
                    "-104.883",
                    "2.187",
                    "--wide",
                    "--fov-deg",
                    &fov,
                    "--snapshot",
                    "wide.png",
                ];
                let report = success(scratch.run(&args));
                assert!(
                    report.contains(&format!(
                        "catalogue={} sky={sky} visible={visible} failed=0 indicators={indicators}",
                        looks.len()
                    )),
                    "{report}"
                );
                let expected = scratch.png("wide.png");
                assert_eq!(success(scratch.run(&args)), report);
                assert_eq!(scratch.png("wide.png"), expected);
                assert_snapshot_elevation(&scratch, "wide.png", selected_elevation);
            }
        }
    }
}

fn assert_snapshot_elevation(scratch: &Scratch, path: &str, elevation: Option<f64>) {
    use embedded_graphics::{pixelcolor::Rgb888, prelude::*};
    use embedded_graphics_simulator::SimulatorDisplay;
    let image = SimulatorDisplay::<Rgb888>::load_png(scratch.0.join(path)).unwrap();
    let ink = Rgb888::new(32, 32, 32); // Existing LcdWhite theme, two states only.
    let paper = Rgb888::new(245, 245, 245);
    for y in 0..240 {
        for x in 0..400 {
            assert!([ink, paper].contains(&image.get_pixel(Point::new(x, y))));
        }
    }
    assert_eq!(
        image.get_pixel(Point::new(375, 7)) == ink,
        elevation.is_some()
    );
    for row in 0..24 {
        for col in 0..24 {
            let angle = (23.5 - f64::from(row)).atan2(f64::from(col) + 0.5);
            assert_eq!(
                image.get_pixel(Point::new(364 + col, 60 + row)) == ink,
                elevation.is_some_and(|e| angle <= e),
                "glyph column={col}, row={row}, elevation={elevation:?}"
            );
        }
    }
    if elevation.is_none() {
        for y in 0..153 {
            for x in 352..400 {
                assert_eq!(image.get_pixel(Point::new(x, y)), paper);
            }
        }
    }
}

#[test]
fn wide_selected_label_uses_id_not_catalogue_name() {
    // Real ISS orbit at its epoch, viewed from its subpoint. Only identity/name
    // metadata varies; synthetic IDs are renderer checks, not new fixtures.
    let scratch = Scratch::new();
    let mut records: Value =
        serde_json::from_str(include_str!("../../core/tests/fixtures/iss-25544.json")).unwrap();
    let elements: Vec<overhead_core::OmmElements> =
        serde_json::from_value(records.clone()).unwrap();
    let satellite = overhead_core::Satellite::from_elements(elements[0].elements()).unwrap();
    let time = satellite.epoch();
    let observer =
        overhead_core::ecef_to_geodetic(satellite.state_at(time).unwrap().to_ecef().unwrap())
            .unwrap();
    let utc = format!("{time}Z").replace(' ', "T");
    let latitude = observer.latitude_rad.to_degrees().to_string();
    let longitude = observer.longitude_rad.to_degrees().to_string();
    let args = [
        "catalogue.json",
        &utc,
        &latitude,
        &longitude,
        "0",
        "--wide",
        "--snapshot",
        "label.png",
    ];
    let mut images = Vec::new();
    for id in [25544, 38771, 100530, 1_234_567] {
        records[0]["NORAD_CAT_ID"] = json!(id);
        records[0]["OBJECT_NAME"] = json!("ISS (ZARYA)");
        scratch.manifest(records.clone());
        let report = success(scratch.run(&args));
        assert!(report.contains("catalogue=1 sky=1 visible=1 failed=0 indicators=0"));
        let image = scratch.png("label.png");
        assert_eq!(success(scratch.run(&args)), report);
        assert_eq!(scratch.png("label.png"), image);
        // Even a misleading or unfamiliar full name cannot change the label.
        records[0]["OBJECT_NAME"] = json!("UNFAMILIAR OBJECT");
        scratch.manifest(records.clone());
        assert_eq!(success(scratch.run(&args)), report);
        assert_eq!(scratch.png("label.png"), image);
        assert!(images.iter().all(|previous| previous != &image));
        images.push(image);
    }
    // Exact boundaries and startup in the hysteresis interval. At an unchanged
    // frame/lens, overriding thresholds changes only label presentation.
    for (fov, hide, show) in [("60", "60", "50"), ("65", "65", "55"), ("70", "80", "60")] {
        let mut configured = args.to_vec();
        configured.extend(["--fov-deg", fov]);
        let report = success(scratch.run(&configured));
        let default = scratch.png("label.png");
        configured.extend(["--label-hide-deg", hide, "--label-show-deg", show]);
        assert_eq!(success(scratch.run(&configured)), report);
        let overridden = scratch.png("label.png");
        assert_ne!(overridden, default);
        assert_eq!(success(scratch.run(&configured)), report);
        assert_eq!(scratch.png("label.png"), overridden);
    }
    // At overview zoom, changing identity no longer changes the image: the
    // label is absent even though the selected marker remains visible.
    let mut overview = args.to_vec();
    overview.extend(["--fov-deg", "100"]);
    let mut previous = None;
    for id in [25544, 100530] {
        records[0]["NORAD_CAT_ID"] = json!(id);
        scratch.manifest(records.clone());
        let report = success(scratch.run(&overview));
        assert!(report.contains("catalogue=1 sky=1 visible=1 failed=0 indicators=0"));
        let image = scratch.png("label.png");
        if let Some(previous) = previous {
            assert_eq!(image, previous);
        }
        previous = Some(image);
    }
}

#[test]
fn malformed_inputs_fail_before_creating_a_snapshot() {
    let scratch = Scratch::new();
    for args in [
        vec!["--demo", "--fov-deg", "NaN", "--snapshot", "bad.png"],
        vec!["--demo", "--viewport", "triangle", "--snapshot", "bad.png"],
        vec![
            "--demo",
            "--wide",
            "--viewport",
            "square",
            "--snapshot",
            "bad.png",
        ],
        vec!["--demo", "--wide", "--wide", "--snapshot", "bad.png"],
        vec!["--demo", "--label-hide-deg", "80", "--snapshot", "bad.png"],
        vec![
            "--demo",
            "--wide",
            "--label-hide-deg",
            "60",
            "--snapshot",
            "bad.png",
        ],
        vec![
            "--demo",
            "--wide",
            "--label-show-deg",
            "NaN",
            "--snapshot",
            "bad.png",
        ],
        vec!["--demo", "--radius-km", "8000", "--snapshot", "bad.png"],
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
    let removed = scratch.run(&["--demo", "--radius-km", "8000"]);
    assert!(
        String::from_utf8_lossy(&removed.stderr).contains("--radius-km was removed; use --fov-deg")
    );
    let help = success(scratch.run(&["--help"]));
    assert!(help.contains("Usage:") && help.contains("--fov-deg"));
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
fn unrepresentable_camera_geometry_is_a_fatal_configuration_error() {
    let scratch = Scratch::new();
    scratch.manifest(
        serde_json::from_str(include_str!("../../core/tests/fixtures/iss-25544.json")).unwrap(),
    );
    let output = scratch.run(&[
        "catalogue.json",
        "2026-10-10T00:00:00Z",
        "0",
        "0",
        "-75000",
        "--snapshot",
        "bad-camera.png",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("observer height cannot form a finite Earth view")
    );
    assert!(output.stdout.is_empty());
    assert!(!scratch.0.join("bad-camera.png").exists());
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
    let mut wide_args = args.to_vec();
    wide_args.push("--wide");
    let output = scratch.run(&wide_args);
    assert!(String::from_utf8_lossy(&output.stderr).contains("NORAD 25544: propagation:"));
    assert!(success(output).contains("catalogue=1 sky=0 visible=0 failed=1 indicators=0"));
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
    let output = scratch.run(&wide_args);
    assert!(String::from_utf8_lossy(&output.stderr).contains("warning: skipped"));
    assert!(success(output).contains("catalogue=1"));
    scratch.png("error.png");
}
