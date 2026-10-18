use super::*;
use embedded_graphics::prelude::Point;
use overhead_app::{Counts, Sample, TrackingError, projection::View};
use overhead_core::sgp4::chrono::{Datelike, NaiveDateTime};
use std::path::PathBuf;

fn replay(mode: &str) -> (RadarApp, Layout) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/weather-gps.json");
    let mut args = vec![
        manifest.to_str().unwrap().to_owned(),
        "2026-10-10T00:00:00Z".into(),
        "39.007".into(),
        "-104.883".into(),
        "2.187".into(),
    ];
    if mode == "wide" {
        args.push("--wide".into());
    } else {
        args.extend(["--viewport".into(), mode.into()]);
    }
    let config = Config::parse(&args).unwrap();
    let loaded = overhead_tools::catalogue::load(&config.manifest).unwrap();
    let mut app = RadarApp::new(loaded.catalogue, config.observer, config.view)
        .unwrap()
        .with_group_categories(loaded.group_categories)
        .with_label_zoom(config.label_zoom);
    app.update(config.start).unwrap();
    (app, config.layout)
}

#[derive(Debug, PartialEq)]
struct AppState {
    time: Option<NaiveDateTime>,
    view: View,
    selected: Option<u64>,
    counts: Counts,
    label_visible: bool,
    samples: Vec<(u64, Result<Sample, TrackingError>)>,
}
impl AppState {
    fn new(app: &RadarApp) -> Self {
        Self {
            time: app.time(),
            view: app.view(),
            selected: app.selected_id(),
            counts: app.counts(),
            label_visible: app.selected_label_visible(),
            samples: app
                .objects()
                .map(|(entry, sample)| (entry.elements().norad_id, sample.clone()))
                .collect(),
        }
    }
}
fn pixels(display: &SimulatorDisplay<BinaryColor>) -> Vec<BinaryColor> {
    (0..240)
        .flat_map(|y| (0..400).map(move |x| display.get_pixel(Point::new(x, y))))
        .collect()
}
fn normal(app: &RadarApp, layout: Layout, warning: bool) -> Vec<BinaryColor> {
    let mut display = SimulatorDisplay::new(overhead_render::DISPLAY_SIZE);
    overhead_render::draw(&mut display, app, layout, warning).unwrap();
    pixels(&display)
}
fn frame(
    presentation: &mut InteractivePresentation,
    app: &RadarApp,
    layout: Layout,
    warning: bool,
    ms: u64,
) -> Vec<BinaryColor> {
    let mut display = SimulatorDisplay::new(overhead_render::DISPLAY_SIZE);
    let state = AppState::new(app);
    presentation
        .draw(
            &mut display,
            app,
            layout,
            warning,
            Duration::from_millis(ms),
        )
        .unwrap();
    assert_eq!(AppState::new(app), state);
    pixels(&display)
}

#[test]
fn wheel_direction_and_large_events() {
    assert_eq!(wheel_steps(1, MouseWheelDirection::Normal), 1);
    assert_eq!(wheel_steps(1, MouseWheelDirection::Flipped), -1);
    assert_eq!(wheel_steps(i32::MIN, MouseWheelDirection::Flipped), 64);
    assert_eq!(wheel_steps(0, MouseWheelDirection::Normal), 0);
}

#[test]
fn v_cycles_only_wide_presentation_without_mutating_app_or_repeating() {
    for mode in ["wide", "square", "circle", "borderless-circle"] {
        let (mut app, layout) = replay(mode);
        let state = AppState::new(&app);
        let mut presentation = InteractivePresentation::new(app.view().viewport(), false);
        let expected = normal(&app, layout, true);
        assert_eq!(frame(&mut presentation, &app, layout, true, 0), expected);
        assert!(presentation.key(&mut app, Keycode::V, true));
        assert!(!presentation.debug);
        assert!(presentation.key(&mut app, Keycode::V, false));
        assert_eq!(presentation.debug, mode == "wide");
        assert_eq!(AppState::new(&app), state);
        let shown = frame(&mut presentation, &app, layout, true, 50);
        if mode == "wide" {
            assert_ne!(shown, expected);
            // Explicit debug start and toggled entry are the same current page.
            let mut startup = InteractivePresentation::new(app.view().viewport(), true);
            assert_eq!(frame(&mut startup, &app, layout, true, 0), shown);
        } else {
            assert_eq!(shown, expected);
        }
        presentation.key(&mut app, Keycode::V, true);
        assert_eq!(presentation.debug, mode == "wide");
        presentation.key(&mut app, Keycode::V, false);
        assert!(!presentation.debug);
        assert_eq!(frame(&mut presentation, &app, layout, true, 100), expected);
        assert_eq!(AppState::new(&app), state);
        assert!(!presentation.key(&mut app, Keycode::Escape, false));
    }
}

#[test]
fn debug_keeps_tab_zoom_and_replay_live_then_returns_static_without_queued_motion() {
    let (mut app, layout) = replay("wide");
    let mut presentation = InteractivePresentation::new(app.view().viewport(), false);
    let start = app.time().unwrap();
    frame(&mut presentation, &app, layout, false, 0);
    presentation.key(&mut app, Keycode::Tab, false);
    // Enter debug during a real transition, rather than just from a static page.
    let moving = frame(&mut presentation, &app, layout, false, 10);
    assert_ne!(moving, normal(&app, layout, false));
    presentation.key(&mut app, Keycode::V, false);
    let old_id = app.selected_id();
    presentation.key(&mut app, Keycode::Tab, true);
    assert_eq!(app.selected_id(), old_id);
    presentation.key(&mut app, Keycode::Tab, false);
    assert_ne!(app.selected_id(), old_id);
    let original_fov = app.view().fov_deg();
    presentation.key(&mut app, Keycode::Up, true); // Zoom repetition stays active.
    assert!(app.view().fov_deg() < original_fov);
    presentation.key(&mut app, Keycode::Down, false);
    assert!((app.view().fov_deg() - original_fov).abs() < 1e-12);
    app.zoom(wheel_steps(2, MouseWheelDirection::Normal)); // Same mouse path as loop.
    assert!(app.view().fov_deg() < original_fov);
    let old_debug = frame(&mut presentation, &app, layout, true, 50);
    // Independent replay/presentation clocks deliberately advance by different
    // amounts; debug must use current physical UTC, not animation time.
    app.update(replay_time(start, Duration::from_secs(30)).unwrap())
        .unwrap();
    let current_debug = frame(&mut presentation, &app, layout, true, 100);
    assert_ne!(old_debug, current_debug);
    let mut direct = SimulatorDisplay::new(overhead_render::DISPLAY_SIZE);
    overhead_render::debug::draw(&mut direct, &app, true).unwrap();
    assert_eq!(current_debug, pixels(&direct));
    let state = AppState::new(&app);
    presentation.key(&mut app, Keycode::V, false);
    assert_eq!(AppState::new(&app), state);
    let returned = frame(&mut presentation, &app, layout, true, 110);
    assert_eq!(returned, normal(&app, layout, true));
    assert_eq!(frame(&mut presentation, &app, layout, true, 120), returned);
    // New normal-view selections still animate; only hidden history was reset.
    presentation.key(&mut app, Keycode::Tab, false);
    assert_ne!(
        frame(&mut presentation, &app, layout, true, 130),
        normal(&app, layout, true)
    );
    assert_eq!(
        frame(&mut presentation, &app, layout, true, 380),
        normal(&app, layout, true)
    );
}

#[test]
fn failure_counts_causes_and_normal_notices_survive_debug_round_trips() {
    use overhead_core::{
        GeodeticPosition, OmmElements,
        catalogue::{CatalogueBuilder, RecordOrigin},
        ecef_to_geodetic,
    };
    let original: serde_json::Value =
        serde_json::from_str(include_str!("../../core/tests/fixtures/iss-25544.json")).unwrap();
    for mixed in [false, true] {
        let mut builder = CatalogueBuilder::new();
        let elements: Vec<OmmElements> = serde_json::from_value(original.clone()).unwrap();
        let satellite = overhead_core::Satellite::from_elements(elements[0].elements()).unwrap();
        let time = satellite.epoch();
        let subpoint =
            ecef_to_geodetic(satellite.state_at(time).unwrap().to_ecef().unwrap()).unwrap();
        if mixed {
            builder.push(
                RecordOrigin {
                    group: 0,
                    record: 0,
                },
                elements.into_iter().next().unwrap(),
            );
        }
        let mut fault = original.clone();
        fault[0]["NORAD_CAT_ID"] = serde_json::json!(99999);
        fault[0]["MEAN_MOTION"] = serde_json::json!(1e300);
        let elements: Vec<OmmElements> = serde_json::from_value(fault).unwrap();
        builder.push(
            RecordOrigin {
                group: 0,
                record: 1,
            },
            elements.into_iter().next().unwrap(),
        );
        let layout = Layout::wide();
        let mut app = RadarApp::new(
            builder.finish().unwrap(),
            GeodeticPosition {
                altitude_km: 0.0,
                ..subpoint
            },
            View::rectangular(20.0, layout.aspect_ratio()).unwrap(),
        )
        .unwrap();
        app.update(time).unwrap();
        assert_eq!(app.counts().failed, 1);
        assert_eq!(app.counts().visible, usize::from(mixed));
        let state = AppState::new(&app);
        let expected = normal(&app, layout, true);
        let mut presentation = InteractivePresentation::new(app.view().viewport(), false);
        assert_eq!(frame(&mut presentation, &app, layout, true, 0), expected);
        presentation.key(&mut app, Keycode::V, false);
        assert_ne!(frame(&mut presentation, &app, layout, true, 50), expected);
        presentation.key(&mut app, Keycode::V, false);
        assert_eq!(frame(&mut presentation, &app, layout, true, 100), expected);
        assert_eq!(AppState::new(&app), state);
    }
}

#[test]
fn debug_empty_and_invalid_frames_recover_without_stale_normal_notices_or_fields() {
    let (mut app, layout) = replay("wide");
    let start = app.time().unwrap();
    let mut presentation = InteractivePresentation::new(app.view().viewport(), true);
    let selected = frame(&mut presentation, &app, layout, false, 0);
    for _ in 0..64 {
        presentation.key(&mut app, Keycode::Up, false);
    }
    assert_eq!(app.counts().visible, 0);
    assert!(app.counts().above_horizon > 0);
    assert!(app.selected_id().is_some()); // Still tracked above the horizon.
    let empty = frame(&mut presentation, &app, layout, true, 10);
    assert_ne!(selected, empty);
    presentation.key(&mut app, Keycode::V, false);
    assert_eq!(
        frame(&mut presentation, &app, layout, true, 20),
        normal(&app, layout, true)
    );
    presentation.key(&mut app, Keycode::V, false);
    // NO FRAME is a renderer/app control, not a persistent host-clock screen.
    assert!(app.update(start.with_year(2101).unwrap()).is_err());
    let invalid = frame(&mut presentation, &app, layout, true, 30);
    assert_ne!(empty, invalid);
    presentation.key(&mut app, Keycode::V, false);
    assert_eq!(
        frame(&mut presentation, &app, layout, true, 40),
        normal(&app, layout, true)
    );
    presentation.key(&mut app, Keycode::V, false);
    app.update(start).unwrap();
    for _ in 0..64 {
        presentation.key(&mut app, Keycode::Down, false);
    }
    assert!(app.selected_id().is_some());
    frame(&mut presentation, &app, layout, false, 50);
    presentation.key(&mut app, Keycode::V, false);
    assert_eq!(
        frame(&mut presentation, &app, layout, false, 60),
        normal(&app, layout, false)
    );
    // Even two toggles in a single event batch discard, rather than queue motion.
    presentation.key(&mut app, Keycode::V, false);
    presentation.key(&mut app, Keycode::Tab, false);
    presentation.key(&mut app, Keycode::V, false);
    assert_eq!(
        frame(&mut presentation, &app, layout, false, 70),
        normal(&app, layout, false)
    );
}
