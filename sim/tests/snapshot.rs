//! Real executable tests: no SDL window, network, wall clock, or hardware.
use overhead_app::category::Category;
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
fn wide_empty_status_distinguishes_real_arrows_from_clean_horizon() {
    let scratch = Scratch::new();
    let report = success(scratch.run(&["--demo", "--wide", "--snapshot", "arrows.png"]));
    assert!(
        report.contains("catalogue=16 sky=4 visible=0 failed=0 indicators=4"),
        "{report}"
    );
    assert_snapshot_notices(&scratch, "arrows.png", &["NONE IN VIEW"]);

    let iss: Value =
        serde_json::from_str(include_str!("../../core/tests/fixtures/iss-25544.json")).unwrap();
    scratch.manifest(iss.clone());
    let run = |file: &str| {
        scratch.run(&[
            "catalogue.json",
            "2026-10-04T12:43:41.833056Z",
            "39.007",
            "-104.883",
            "2.187",
            "--wide",
            "--snapshot",
            file,
        ])
    };
    let report = success(run("horizon.png"));
    assert!(
        report.contains("catalogue=1 sky=0 visible=0 failed=0 indicators=0"),
        "{report}"
    );
    assert_snapshot_notices(&scratch, "horizon.png", &["NONE ABOVE HORIZON"]);

    // A known omission prevents the horizon claim, without changing the accepted ISS.
    let mut omitted = iss;
    omitted
        .as_array_mut()
        .unwrap()
        .push(json!({"bad": "record"}));
    scratch.manifest(omitted);
    let output = run("omitted.png");
    assert!(String::from_utf8_lossy(&output.stderr).contains("warning: skipped"));
    assert!(success(output).contains("catalogue=1 sky=0 visible=0 failed=0 indicators=0"));
    assert_snapshot_notices(&scratch, "omitted.png", &["NO MARKERS", "INGEST WARN"]);
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
        GeodeticPosition, ecef_to_geodetic, ecef_to_look_angles,
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
            let physical: Vec<_> = loaded
                .catalogue
                .entries()
                .iter()
                .map(|entry| {
                    let state = entry.satellite().state_at(time).unwrap();
                    let ecef = state.to_ecef().unwrap();
                    (
                        ecef_to_look_angles(ecef, observer).unwrap(),
                        ecef_to_geodetic(ecef).unwrap().altitude_km,
                        state.velocity().iter().map(|v| v * v).sum::<f64>().sqrt(),
                        1440.0 / entry.elements().mean_motion,
                        // Explicit expected roles for these two local manifests,
                        // not the app resolver or the loader's category table.
                        if name == "weather-gps.json" {
                            if entry.groups().contains(&1) {
                                Category::Navigation
                            } else {
                                Category::Weather
                            }
                        } else {
                            Category::Unknown
                        },
                    )
                })
                .collect();
            let sky = physical
                .iter()
                .filter(|(look, _, _, _, _)| look.elevation_rad > 0.0)
                .count();
            for fov in [0.1_f64, 5.0, 20.0, 40.0, 100.0] {
                let mut visible = 0;
                let mut indicators = 0;
                let mut selected_elevation = None;
                let mut selected_altitude = None;
                let mut selected_speed = None;
                let mut selected_period = None;
                let mut selected_category = None;
                for (look, altitude, speed, period, category) in &physical {
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
                        selected_altitude.get_or_insert(*altitude);
                        selected_speed.get_or_insert(*speed);
                        selected_period.get_or_insert(*period);
                        selected_category.get_or_insert(*category);
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
                        physical.len()
                    )),
                    "{report}"
                );
                let expected = scratch.png("wide.png");
                assert_eq!(success(scratch.run(&args)), report);
                assert_eq!(scratch.png("wide.png"), expected);
                assert_snapshot_fields(
                    &scratch,
                    "wide.png",
                    selected_elevation,
                    selected_altitude,
                    selected_speed,
                    selected_period,
                    selected_category,
                );
                if visible == 0 {
                    assert_snapshot_notices(
                        &scratch,
                        "wide.png",
                        &[if indicators > 0 {
                            "NONE IN VIEW"
                        } else {
                            "NONE ABOVE HORIZON"
                        }],
                    );
                }
            }
        }
    }
}

#[test]
fn selected_fields_span_real_low_navigation_and_geo_orbits() {
    use overhead_core::{GeodeticPosition, ecef_to_geodetic, ecef_to_look_angles};
    let scratch = Scratch::new();
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/catalogue.json");
    let loaded = overhead_tools::catalogue::load(&manifest).unwrap();
    let mut regimes = [false; 3];
    let mut speed_regimes = [false; 3];
    let mut period_regimes = [false; 3];
    for entry in loaded.catalogue.entries() {
        // Existing real pool elements, each alone and viewed from its subpoint.
        // These deliberate observer sites are test inputs, not device defaults.
        scratch.manifest(json!([entry.elements()]));
        let time = entry.satellite().epoch();
        let state = entry.satellite().state_at(time).unwrap();
        let ecef = state.to_ecef().unwrap();
        let speed = state.velocity().iter().map(|v| v * v).sum::<f64>().sqrt();
        let period = 1440.0 / entry.elements().mean_motion;
        period_regimes[0] |= period < 120.0;
        period_regimes[1] |= (600.0..800.0).contains(&period);
        period_regimes[2] |= period > 1400.0;
        speed_regimes[0] |= speed < 3.5;
        speed_regimes[1] |= (3.5..5.0).contains(&speed);
        speed_regimes[2] |= speed > 6.0;
        let geodetic = ecef_to_geodetic(ecef).unwrap();
        let observer = GeodeticPosition {
            altitude_km: 0.0,
            ..geodetic
        };
        let look = ecef_to_look_angles(ecef, observer).unwrap();
        let altitude = geodetic.altitude_km;
        regimes[0] |= altitude < 2_000.0;
        regimes[1] |= (15_000.0..25_000.0).contains(&altitude);
        regimes[2] |= altitude > 30_000.0;
        let utc = format!("{time}Z").replace(' ', "T");
        let latitude = observer.latitude_rad.to_degrees().to_string();
        let longitude = observer.longitude_rad.to_degrees().to_string();
        let report = success(scratch.run(&[
            "catalogue.json",
            &utc,
            &latitude,
            &longitude,
            "0",
            "--wide",
            "--snapshot",
            "altitude.png",
        ]));
        assert!(
            report.contains("catalogue=1 sky=1 visible=1 failed=0 indicators=0"),
            "{report}"
        );
        assert_snapshot_fields(
            &scratch,
            "altitude.png",
            Some(look.elevation_rad),
            Some(altitude),
            Some(speed),
            Some(period),
            Some(Category::Unknown),
        );
    }
    assert_eq!(period_regimes, [true; 3]);
    assert_eq!(regimes, [true; 3]);
    assert_eq!(speed_regimes, [true; 3]);
}

#[test]
fn configured_categories_reach_executable_with_hierarchy_and_no_other_pixel_changes() {
    use embedded_graphics::{pixelcolor::Rgb888, prelude::*};
    use embedded_graphics_simulator::SimulatorDisplay;
    use overhead_core::{OmmElements, Satellite, ecef_to_geodetic, ecef_to_look_angles};
    let scratch = Scratch::new();
    let records: Value =
        serde_json::from_str(include_str!("../../core/tests/fixtures/iss-25544.json")).unwrap();
    scratch.manifest(records.clone());
    let elements: Vec<OmmElements> = serde_json::from_value(records).unwrap();
    let satellite = Satellite::from_elements(elements[0].elements()).unwrap();
    let time = satellite.epoch();
    let state = satellite.state_at(time).unwrap();
    let ecef = state.to_ecef().unwrap();
    let geodetic = ecef_to_geodetic(ecef).unwrap();
    let mut observer = geodetic;
    observer.altitude_km = 0.0;
    let look = ecef_to_look_angles(ecef, observer).unwrap();
    let speed = state.velocity().iter().map(|v| v * v).sum::<f64>().sqrt();
    let utc = format!("{time}Z").replace(' ', "T");
    let lat = observer.latitude_rad.to_degrees().to_string();
    let lon = observer.longitude_rad.to_degrees().to_string();
    let run = |wide: bool| {
        let mut args = vec![
            "catalogue.json",
            &utc,
            &lat,
            &lon,
            "0",
            "--snapshot",
            "category.png",
        ];
        if wide {
            args.push("--wide");
        }
        success(scratch.run(&args))
    };
    run(false);
    let square = scratch.png("category.png");
    run(true);
    let baseline = SimulatorDisplay::<Rgb888>::load_png(scratch.0.join("category.png")).unwrap();
    for (roles, expected) in [
        (vec![None], Category::Unknown),
        (vec![Some("weather")], Category::Weather),
        (vec![Some("navigation")], Category::Navigation),
        (vec![Some("station")], Category::Station),
        (
            vec![Some("weather"), None, Some("navigation")],
            Category::Navigation,
        ),
        (
            vec![Some("navigation"), Some("weather"), None],
            Category::Navigation,
        ),
        (
            vec![Some("navigation"), Some("station"), Some("weather")],
            Category::Station,
        ),
        (
            vec![Some("weather"), Some("station"), Some("navigation")],
            Category::Station,
        ),
        (vec![Some("weather"), Some("weather")], Category::Weather),
    ] {
        let groups: Vec<_> = roles.iter().enumerate().map(|(i, role)| {
            json!({"name":format!("stations-{i}"), "path":"data.json", "category":role})
        }).collect();
        fs::write(
            scratch.0.join("catalogue.json"),
            json!({"groups":groups}).to_string(),
        )
        .unwrap();
        let report = run(true);
        assert!(report.contains("catalogue=1 sky=1 visible=1 failed=0 indicators=0"));
        assert_snapshot_fields(
            &scratch,
            "category.png",
            Some(look.elevation_rad),
            Some(geodetic.altitude_km),
            Some(speed),
            Some(1440.0 / elements[0].elements().mean_motion),
            Some(expected),
        );
        let image = SimulatorDisplay::<Rgb888>::load_png(scratch.0.join("category.png")).unwrap();
        for y in 0..240 {
            for x in 0..400 {
                if !(364..388).contains(&x) || !(10..43).contains(&y) {
                    let point = Point::new(x, y);
                    assert_eq!(image.get_pixel(point), baseline.get_pixel(point));
                }
            }
        }
        run(false);
        assert_eq!(scratch.png("category.png"), square);
    }
}

fn assert_snapshot_fields(
    scratch: &Scratch,
    path: &str,
    elevation: Option<f64>,
    altitude: Option<f64>,
    speed: Option<f64>,
    period: Option<f64>,
    category: Option<Category>,
) {
    use embedded_graphics::{
        mono_font::{MonoTextStyle, ascii::FONT_6X10},
        pixelcolor::{BinaryColor, Rgb888},
        prelude::*,
        text::Text,
    };
    use embedded_graphics_simulator::SimulatorDisplay;
    let image = SimulatorDisplay::<Rgb888>::load_png(scratch.0.join(path)).unwrap();
    let ink = Rgb888::new(32, 32, 32); // Existing LcdWhite theme, two states only.
    let paper = Rgb888::new(245, 245, 245);
    for y in 0..240 {
        for x in 0..400 {
            assert!([ink, paper].contains(&image.get_pixel(Point::new(x, y))));
        }
    }
    let mut reference = SimulatorDisplay::<BinaryColor>::new(Size::new(400, 240));
    reference.clear(BinaryColor::Off).unwrap();
    reference
        .draw_iter((12..=227).map(|y| Pixel(Point::new(352, y), BinaryColor::On)))
        .unwrap();
    if let Some(category) = category {
        Text::new(
            "Cat",
            Point::new(367, 17),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        )
        .draw(&mut reference)
        .unwrap();
        for y in 0..24 {
            for x in 0..24 {
                if category_reference_ink(category, x, y) {
                    Pixel(Point::new(364 + x, 19 + y), BinaryColor::On)
                        .draw(&mut reference)
                        .unwrap();
                }
            }
        }
    }
    if let Some(elevation) = elevation {
        Text::new(
            "Elev",
            Point::new(364, 64),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        )
        .draw(&mut reference)
        .unwrap();
        // Physical elevation was calculated independently from the app above.
        // This reference also avoids the renderer's octant table, integer
        // angular clipping and Bresenham accumulator: classify a rounded-sqrt
        // ring, choose its nearest polar angle, then round major-axis slopes.
        let on_ring = |x: i32, y: i32| {
            let minor = x.abs().min(y.abs());
            let major = x.abs().max(y.abs());
            minor <= 8 && major == f64::from(121 - minor * minor).sqrt().round() as i32
        };
        let (ex, ey) = (0..=11)
            .flat_map(|x| (0..=11).map(move |y| (x, y)))
            .filter(|&(x, y)| on_ring(x, y))
            .min_by(|&(ax, ay), &(bx, by)| {
                let a = f64::from(ay).atan2(f64::from(ax));
                let b = f64::from(by).atan2(f64::from(bx));
                (a - elevation)
                    .abs()
                    .total_cmp(&(b - elevation).abs())
                    .then_with(|| a.total_cmp(&b))
            })
            .unwrap();
        let angle = f64::from(ey).atan2(f64::from(ex));
        for y in -11..=11 {
            for x in -11..=11 {
                let a = f64::from(y).atan2(f64::from(x));
                let arc = on_ring(x, y) && !(a > 0.0 && a < angle);
                let radial = ey > 0
                    && ((y == 0 && (0..=11).contains(&x))
                        || ((0..=ex).contains(&x)
                            && (0..=ey).contains(&y)
                            && if ex >= ey {
                                y == (2 * x * ey + ex) / (2 * ex)
                            } else {
                                x == (2 * y * ex + ey) / (2 * ey)
                            }));
                if arc || radial {
                    Pixel(Point::new(376 + x, 80 - y), BinaryColor::On)
                        .draw(&mut reference)
                        .unwrap();
                }
            }
        }
    }
    if let Some(altitude) = altitude {
        Text::new(
            "Alt",
            Point::new(367, 111),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        )
        .draw(&mut reference)
        .unwrap();
        // Independent inverse thresholds and rounded-sqrt classification,
        // not the production log mapping or integer octant traversal.
        let radius = (2..=8).fold(9, |radius, lower| {
            let boundary = 400.0 * (101_f64.powf((8.5 - lower as f64) / 7.0) - 1.0);
            if altitude > boundary {
                radius.min(lower)
            } else {
                radius
            }
        });
        for y in -11_i32..=11 {
            for x in -11_i32..=11 {
                let minor = x.abs().min(y.abs());
                let major = x.abs().max(y.abs());
                if [radius, 11].into_iter().any(|r| {
                    minor <= r && major == f64::from(r * r - minor * minor).sqrt().round() as i32
                }) {
                    Pixel(Point::new(376 + x, 127 + y), BinaryColor::On)
                        .draw(&mut reference)
                        .unwrap();
                }
            }
        }
    }
    if let Some(speed) = speed {
        Text::new(
            "Spd",
            Point::new(367, 158),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        )
        .draw(&mut reference)
        .unwrap();
        // Independent ring classification / nearest-angle choice, without the
        // production octant traversal or km/s midpoint comparisons. Input is
        // independently propagated TEME speed, not app state or screen motion.
        let mut ring: Vec<_> = (-11_i32..=11)
            .flat_map(|y| (-11_i32..=11).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let minor = x.abs().min(y.abs());
                let major = x.abs().max(y.abs());
                major == f64::from(121 - minor * minor).sqrt().round() as i32
            })
            .map(|(x, y)| {
                (
                    (x as f64)
                        .atan2(-f64::from(y))
                        .rem_euclid(std::f64::consts::TAU),
                    x,
                    y,
                )
            })
            .collect();
        ring.sort_by(|a, b| a.0.total_cmp(&b.0));
        if speed > 0.0 {
            let sweep = speed.clamp(0.0, 12.0) / 12.0 * std::f64::consts::TAU;
            let endpoint = ring
                .iter()
                .min_by(|a, b| {
                    (a.0 - sweep)
                        .abs()
                        .total_cmp(&(b.0 - sweep).abs())
                        .then_with(|| a.0.total_cmp(&b.0))
                })
                .unwrap()
                .0;
            for (angle, x, y) in ring {
                if angle <= endpoint {
                    Pixel(Point::new(376 + x, 174 + y), BinaryColor::On)
                        .draw(&mut reference)
                        .unwrap();
                }
            }
        }
    }
    if let Some(period) = period {
        Text::new(
            "Per",
            Point::new(367, 205),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        )
        .draw(&mut reference)
        .unwrap();
        // Element-derived input, independent of app state. Host math and
        // absolute screen points rather than production libm/row bitsets.
        let turns = 2.5 - 1.5 * (period.clamp(90.0, 1440.0) / 90.0).ln() / 16_f64.ln();
        for step in 0..=4096 {
            let u = step as f64 / 4096.0;
            let r = 1.5 + 7.5 * u;
            let a = -std::f64::consts::FRAC_PI_2 + std::f64::consts::TAU * turns * (u - 1.0);
            Pixel(
                Point::new(
                    (376.0 + r * a.cos()).round() as i32,
                    (220.0 + r * a.sin()).round() as i32,
                ),
                BinaryColor::On,
            )
            .draw(&mut reference)
            .unwrap();
        }
    }
    for y in 0..240 {
        for x in 352..400 {
            let point = Point::new(x, y);
            let expected = if reference.get_pixel(point) == BinaryColor::On {
                ink
            } else {
                paper
            };
            assert_eq!(
                image.get_pixel(point),
                expected,
                "strip pixel {point:?}, elevation={elevation:?}, altitude={altitude:?}, speed={speed:?}, period={period:?}"
            );
        }
    }
}

// Review glyphs expressed as local geometry, independent of packed renderer masks.
fn category_reference_ink(category: Category, x: i32, y: i32) -> bool {
    match category {
        Category::Navigation => {
            (x == 11 && (2..=21).contains(&y)) || (y == 11 && (2..=21).contains(&x))
        }
        Category::Station => {
            ((y == 8 || y == 15) && (9..=14).contains(&x))
                || ((x == 9 || x == 14) && (8..=15).contains(&y))
                || (y == 11 && ((2..=9).contains(&x) || (14..=21).contains(&x)))
        }
        Category::Weather => match y {
            5 => (8..=11).contains(&x),
            6 => x == 7 || x == 12,
            7 => x == 6 || x == 13,
            8 => x == 6 || (13..=16).contains(&x),
            9 => x == 5 || x == 6 || x == 17,
            10 => x == 3 || x == 4 || x == 18,
            11 => x == 3 || (18..=20).contains(&x),
            12..=15 => x == 2 || x == 21,
            16 => x == 3 || x == 20,
            17 => (4..=19).contains(&x),
            _ => false,
        },
        Category::Unknown => match y {
            3 => (10..=13).contains(&x),
            4 => x == 9 || x == 14,
            5..=6 => x == 8 || x == 15,
            7..=8 => x == 15,
            9..=10 => x == 14,
            11 => x == 13,
            12 => x == 12,
            13..=16 => x == 11,
            19..=20 => x == 11 || x == 12,
            _ => false,
        },
    }
}

fn assert_snapshot_notices(scratch: &Scratch, path: &str, lines: &[&str]) {
    use embedded_graphics::{
        mono_font::{MonoTextStyle, ascii::FONT_6X10},
        pixelcolor::{BinaryColor, Rgb888},
        prelude::*,
        text::Text,
    };
    use embedded_graphics_simulator::SimulatorDisplay;
    assert!(!lines.is_empty());
    if lines.iter().any(|line| {
        matches!(
            *line,
            "NO MARKERS" | "NO FRAME" | "NONE IN VIEW" | "NONE ABOVE HORIZON"
        )
    }) {
        assert_snapshot_fields(scratch, path, None, None, None, None, None);
    }
    let image = SimulatorDisplay::<Rgb888>::load_png(scratch.0.join(path)).unwrap();
    let mut reference = SimulatorDisplay::<BinaryColor>::new(Size::new(400, 240));
    reference.clear(BinaryColor::Off).unwrap();
    let top = 213 - (lines.len() as i32 - 1) * 13;
    let right = 14 + lines.iter().map(|line| line.len()).max().unwrap() as i32 * 6;
    for (row, line) in lines.iter().enumerate() {
        Text::new(
            line,
            Point::new(12, top + 9 + row as i32 * 13),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        )
        .draw(&mut reference)
        .unwrap();
    }
    for y in top..227 {
        for x in 10..right {
            let point = Point::new(x, y);
            let expected = if reference.get_pixel(point) == BinaryColor::On {
                Rgb888::new(32, 32, 32)
            } else {
                Rgb888::new(245, 245, 245)
            };
            assert_eq!(image.get_pixel(point), expected, "notice pixel {point:?}");
        }
    }
    // Below the period slot there is no ink. Selected fields are checked
    // separately, including alongside warnings.
    for y in 233..240 {
        for x in 352..400 {
            let expected = if x == 352 && y <= 227 {
                Rgb888::new(32, 32, 32)
            } else {
                Rgb888::new(245, 245, 245)
            };
            assert_eq!(image.get_pixel(Point::new(x, y)), expected);
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
        vec!["--demo", "--debug", "--snapshot", "bad.png"],
        vec!["--demo", "--wide", "--debug", "--snapshot", "bad.png"],
        vec!["--demo", "--snapshot", "bad.png", "--debug", "--wide"],
        vec![
            "--demo",
            "--wide",
            "--debug",
            "--debug",
            "--snapshot",
            "bad.png",
        ],
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
    assert!(help.contains("--debug") && help.contains("v = toggle radar/debug"));
    for (args, error) in [
        (vec!["--demo", "--debug"], "--debug requires --wide"),
        (
            vec!["--demo", "--wide", "--debug", "--debug"],
            "repeated option: --debug",
        ),
        (
            vec!["--demo", "--wide", "--debug", "--snapshot", "bad.png"],
            "--debug cannot be combined with --snapshot",
        ),
    ] {
        let output = scratch.run(&args);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains(error));
        assert!(!scratch.0.join("bad.png").exists());
    }
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
    assert_snapshot_notices(&scratch, "error.png", &["NO MARKERS", "TRACK ERROR"]);
    // All three notices together, using an explicit synthetic bad record.
    let mut omitted: Value =
        serde_json::from_str(include_str!("../../core/tests/fixtures/iss-25544.json")).unwrap();
    omitted[0]["MEAN_MOTION"] = json!(1e300);
    omitted
        .as_array_mut()
        .unwrap()
        .push(json!({"bad":"record"}));
    scratch.manifest(omitted);
    let output = scratch.run(&wide_args);
    assert!(String::from_utf8_lossy(&output.stderr).contains("warning: skipped"));
    assert!(success(output).contains("catalogue=1 sky=0 visible=0 failed=1 indicators=0"));
    assert_snapshot_notices(
        &scratch,
        "error.png",
        &["NO MARKERS", "TRACK ERROR", "INGEST WARN"],
    );

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

#[test]
fn host_report_retains_original_names_and_every_complete_failure_cause() {
    let scratch = Scratch::new();
    let mut records: Value =
        serde_json::from_str(include_str!("../../core/tests/fixtures/iss-25544.json")).unwrap();
    // Deliberately long/unsupported text exercises the host side of 0047's
    // overflow guidance; the orbital input is unchanged for the selected ISS.
    let name = format!("{} λ ORIGINAL TAIL", "LONG NAME ".repeat(20));
    records[0]["OBJECT_NAME"] = json!(name);
    for id in [99999, 99998] {
        let mut failed = records[0].clone();
        failed["NORAD_CAT_ID"] = json!(id);
        failed["MEAN_MOTION"] = json!(1e300);
        records.as_array_mut().unwrap().push(failed);
    }
    records
        .as_array_mut()
        .unwrap()
        .push(json!({"bad":"record"}));
    scratch.manifest(records);
    let loaded = overhead_tools::catalogue::load(&scratch.0.join("catalogue.json")).unwrap();
    let time = loaded.catalogue.entries()[0].satellite().epoch();
    let output = scratch.run(&[
        "catalogue.json",
        "2026-10-04T12:43:41.833056Z",
        "0.000017409",
        "-85.180357315",
        "0",
        "--wide",
        "--snapshot",
        "normal.png",
    ]);
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();
    assert!(stderr.contains(&format!("Satellite: {name}\n")));
    assert!(stderr.contains("warning: skipped"));
    for entry in &loaded.catalogue.entries()[1..] {
        let cause = entry.satellite().state_at(time).unwrap_err();
        assert!(stderr.contains(&format!(
            "NORAD {}: propagation: {cause}\n",
            entry.elements().norad_id
        )));
    }
    assert!(success(output).contains("catalogue=3 sky=1 visible=1 failed=2"));
    assert_snapshot_notices(&scratch, "normal.png", &["TRACK ERROR", "INGEST WARN"]);
}

#[test]
fn wide_warning_snapshots_preserve_valid_markers_and_clear_after_clean_load() {
    use embedded_graphics::{pixelcolor::Rgb888, prelude::*};
    use embedded_graphics_simulator::SimulatorDisplay;
    let scratch = Scratch::new();
    let records: Value =
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
    let run = |file| {
        scratch.run(&[
            "catalogue.json",
            &utc,
            &latitude,
            &longitude,
            "0",
            "--wide",
            "--snapshot",
            file,
        ])
    };
    scratch.manifest(records.clone());
    assert!(
        success(run("healthy.png")).contains("catalogue=1 sky=1 visible=1 failed=0 indicators=0")
    );
    let state = satellite.state_at(time).unwrap();
    let ecef = state.to_ecef().unwrap();
    let look = overhead_core::ecef_to_look_angles(
        ecef,
        overhead_core::GeodeticPosition {
            altitude_km: 0.0,
            ..observer
        },
    )
    .unwrap();
    let altitude = overhead_core::ecef_to_geodetic(ecef).unwrap().altitude_km;
    let speed = state.velocity().iter().map(|v| v * v).sum::<f64>().sqrt();
    assert_snapshot_fields(
        &scratch,
        "healthy.png",
        Some(look.elevation_rad),
        Some(altitude),
        Some(speed),
        Some(1440.0 / elements[0].elements().mean_motion),
        Some(Category::Unknown),
    );
    let healthy_png = scratch.png("healthy.png");
    let healthy = SimulatorDisplay::<Rgb888>::load_png(scratch.0.join("healthy.png")).unwrap();
    for (failure, omission, lines) in [
        (true, false, vec!["TRACK ERROR"]),
        (true, true, vec!["TRACK ERROR", "INGEST WARN"]),
        (false, true, vec!["INGEST WARN"]),
    ] {
        let mut changed = records.clone();
        if failure {
            let mut bad = records[0].clone();
            bad["NORAD_CAT_ID"] = json!(99999);
            bad["MEAN_MOTION"] = json!(1e300);
            changed.as_array_mut().unwrap().push(bad);
        }
        if omission {
            changed
                .as_array_mut()
                .unwrap()
                .push(json!({"bad":"record"}));
        }
        scratch.manifest(changed);
        let output = run("warned.png");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(stderr.contains("NORAD 99999: propagation:"), failure);
        assert_eq!(stderr.contains("warning: skipped"), omission);
        assert!(success(output).contains(&format!(
            "catalogue={} sky=1 visible=1 failed={} indicators=0",
            1 + usize::from(failure),
            usize::from(failure),
        )));
        assert_snapshot_notices(&scratch, "warned.png", &lines);
        assert_snapshot_fields(
            &scratch,
            "warned.png",
            Some(look.elevation_rad),
            Some(altitude),
            Some(speed),
            Some(1440.0 / elements[0].elements().mean_motion),
            Some(Category::Unknown),
        );
        let warned = SimulatorDisplay::<Rgb888>::load_png(scratch.0.join("warned.png")).unwrap();
        let top = 213 - (lines.len() as i32 - 1) * 13;
        for y in 0..240 {
            for x in 0..400 {
                if !(10..80).contains(&x) || !(top..227).contains(&y) {
                    let point = Point::new(x, y);
                    assert_eq!(warned.get_pixel(point), healthy.get_pixel(point));
                }
            }
        }
    }
    // Ingestion recovery is a new clean load, not a timer or live reload.
    scratch.manifest(records);
    assert!(
        success(run("recovered.png")).contains("catalogue=1 sky=1 visible=1 failed=0 indicators=0")
    );
    assert_eq!(scratch.png("recovered.png"), healthy_png);
}
