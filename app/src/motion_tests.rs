use super::*;
use crate::projection::View;
use overhead_core::{
    GeodeticPosition, OmmElements,
    catalogue::{Catalogue, CatalogueBuilder, RecordOrigin},
    sgp4::chrono::NaiveDate,
};

fn visual() -> Catalogue {
    let elements: Vec<OmmElements> = serde_json::from_str(include_str!(
        "../../firmware/fixtures/celestrak-visual-20261103.json"
    ))
    .unwrap();
    let mut builder = CatalogueBuilder::new();
    for (record, e) in elements.into_iter().enumerate() {
        builder.push(RecordOrigin { group: 0, record }, e);
    }
    builder.finish().unwrap()
}

fn app(fov: f64) -> RadarApp {
    let observer = GeodeticPosition {
        latitude_rad: 40f64.to_radians(),
        longitude_rad: (-105f64).to_radians(),
        altitude_km: 1.6,
    };
    let view = View::rectangular(fov, 400.0 / 240.0).unwrap();
    let mut app = RadarApp::awaiting_catalogue(observer, view).unwrap();
    app.load_catalogue(visual());
    app
}

fn start() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 11, 3)
        .unwrap()
        .and_hms_opt(2, 0, 0)
        .unwrap()
}

/// Full frames every `step` from `from` for `span`, motion following.
fn run(
    app: &mut RadarApp,
    motion: &mut Motion,
    from: NaiveDateTime,
    span: TimeDelta,
    step: TimeDelta,
) -> NaiveDateTime {
    let mut t = from;
    while t <= from + span {
        app.update(t).unwrap();
        motion.update(app, usize::MAX);
        t += step;
    }
    t - step
}

fn propagated_view(app: &RadarApp, i: usize, t: NaiveDateTime) -> Option<ViewPoint> {
    let entry = &app.entries()[i];
    let ecef = entry.satellite().state_at(t).ok()?.to_ecef().ok()?;
    app.view()
        .project_unclipped(app.camera().project(ecef).ok()?)
}

#[test]
fn trail_step_scales_with_the_lens() {
    assert!((trail_step_s(20.0) - 60.0).abs() < 1e-9);
    assert_eq!(trail_step_s(0.1), 1.0);
    let wide = trail_step_s(100.0);
    assert!((404.0..406.0).contains(&wide), "{wide}");
    assert!(trail_step_s(60.0) > trail_step_s(40.0));
}

#[test]
fn trails_follow_propagated_past_positions() {
    for fov in [100.0, 20.0, 2.0] {
        let mut app = app(fov);
        let mut motion = Motion::new();
        let now = run(
            &mut app,
            &mut motion,
            start(),
            TimeDelta::minutes(30),
            TimeDelta::seconds(5),
        );
        let dt = trail_step_s(fov);
        let selected = app.selected_id();
        let mut trails = 0;
        let mut heads = motion.trails(&app);
        // Each trail matches the propagated past within half a pixel.
        let half_px = 0.5 / 117.0;
        for (i, (entry, result)) in app.objects().enumerate() {
            let Some(head) = result.as_ref().ok().and_then(|s| s.marker(app.view())) else {
                continue;
            };
            if Some(entry.elements().norad_id) == selected {
                continue;
            }
            let Some(trail) = heads.next() else { break };
            assert_eq!(trail.points[0], head);
            trails += 1;
            for (k, p) in trail.points().iter().enumerate().skip(1) {
                let at = now - TimeDelta::milliseconds((k as f64 * dt * 1000.0).round() as i64);
                let exact = propagated_view(&app, i, at).unwrap();
                let err = libm::hypot(exact.east - p.east, exact.north - p.north);
                assert!(err < half_px, "fov {fov} entry {i} step {k}: {err}");
            }
        }
        if fov > 10.0 {
            assert!(trails > 0, "fov {fov}");
            assert!(
                motion.trails(&app).any(|t| t.len == TRAIL_STEPS + 1),
                "fov {fov}"
            );
        }
        assert!(motion.trails(&app).all(|t| t.len >= 2));
    }
}

#[test]
fn trails_stay_exact_under_staggered_frames() {
    let mut app = app(20.0);
    let mut motion = Motion::new();
    let window = TimeDelta::seconds(1);
    let mut t = start();
    let end = t + TimeDelta::minutes(6);
    while t < end {
        app.update_staggered(t, window).unwrap();
        motion.update(&app, usize::MAX);
        t += TimeDelta::milliseconds(50);
    }
    let dt = TimeDelta::seconds(60);
    let selected = app.selected_id();
    let mut trails = motion.trails(&app);
    let mut checked = 0;
    for (i, (entry, result)) in app.objects().enumerate() {
        if Some(entry.elements().norad_id) == selected
            || result
                .as_ref()
                .ok()
                .and_then(|s| s.marker(app.view()))
                .is_none()
        {
            continue;
        }
        let trail = trails.next().unwrap();
        let at = app.sample_times()[i];
        for (k, p) in trail.points().iter().enumerate().skip(1) {
            let exact = propagated_view(&app, i, at - dt * k as i32).unwrap();
            let err = libm::hypot(exact.east - p.east, exact.north - p.north);
            assert!(err < 0.5 / 117.0, "entry {i} step {k}: {err}");
            checked += 1;
        }
    }
    assert!(checked > 0);
}

#[test]
fn no_trails_or_orbit_for_another_frame_catalogue_or_selection() {
    let mut app = app(100.0);
    let mut motion = Motion::new();
    let now = run(
        &mut app,
        &mut motion,
        start(),
        TimeDelta::minutes(10),
        TimeDelta::seconds(10),
    );
    assert!(motion.trails(&app).count() > 0);
    assert!(motion.orbit(&app).any(|p| p.is_some()));
    // A frame the motion state has not followed.
    app.update(now + TimeDelta::seconds(1)).unwrap();
    assert_eq!(motion.trails(&app).count(), 0);
    assert_eq!(motion.orbit(&app).count(), 0);
    motion.update(&app, 0);
    assert!(motion.trails(&app).count() > 0);
    assert!(motion.orbit(&app).any(|p| p.is_some()));
    // A new selection: the old orbit is never drawn for it.
    app.select_next();
    assert!(motion.trails(&app).count() > 0);
    assert_eq!(motion.orbit(&app).count(), 0);
    motion.update(&app, 0);
    assert!(motion.orbit(&app).all(|p| p.is_none()));
    assert!(!motion.orbit_complete(&app));
    // A new catalogue (same entries, same UTC) drops the history.
    app.load_catalogue(visual());
    assert_eq!(motion.trails(&app).count(), 0);
    motion.update(&app, usize::MAX);
    assert_eq!(motion.trails(&app).count(), 0);
    assert!(motion.orbit_complete(&app));
    // No frame: nothing at all, and history starts again afterwards.
    app.invalidate_frame();
    motion.update(&app, usize::MAX);
    assert_eq!(motion.trails(&app).count(), 0);
    app.update(now + TimeDelta::seconds(2)).unwrap();
    motion.update(&app, usize::MAX);
    assert_eq!(motion.trails(&app).count(), 0);
}

#[test]
fn history_survives_a_small_step_back_but_not_a_gap() {
    let mut app = app(20.0);
    let mut motion = Motion::new();
    let now = run(
        &mut app,
        &mut motion,
        start(),
        TimeDelta::minutes(6),
        TimeDelta::seconds(1),
    );
    let before = motion
        .trails(&app)
        .filter(|t| t.len == TRAIL_STEPS + 1)
        .count();
    assert!(before > 0);
    // A 100 ms resync backwards keeps full trails.
    app.update(now - TimeDelta::milliseconds(100)).unwrap();
    motion.update(&app, 0);
    assert_eq!(
        motion
            .trails(&app)
            .filter(|t| t.len == TRAIL_STEPS + 1)
            .count(),
        before
    );
    // A ten-minute gap leaves nothing to interpolate across.
    app.update(now + TimeDelta::minutes(10)).unwrap();
    motion.update(&app, 0);
    assert_eq!(motion.trails(&app).count(), 0);
}

#[test]
fn orbit_fills_nearest_first_within_budget_and_rolls_forward() {
    let mut app = app(100.0);
    let mut motion = Motion::new();
    let t = start();
    app.update(t).unwrap();
    assert!(app.selected_id().is_some());
    assert_eq!(motion.update(&app, 4), 4);
    let points: Vec<_> = motion.orbit(&app).collect();
    assert_eq!(points.len(), ORBIT_SEGMENTS + 1);
    // Only the slots around now are filled so far.
    let filled: Vec<usize> = (0..points.len())
        .filter(|&k| {
            motion.orbit.ring[Orbit::index(motion.orbit.centre - Orbit::HALF + k as i64)].slot
                == motion.orbit.centre - Orbit::HALF + k as i64
        })
        .collect();
    assert_eq!(filled, [79, 80, 81, 82]);
    let mut total = 4;
    while !motion.orbit_complete(&app) {
        total += motion.update(&app, 4);
    }
    assert_eq!(total, ORBIT_SEGMENTS + 1);
    // Steady state: one propagation per slot step as now advances.
    let step = motion.orbit.step;
    let mut later = t;
    let mut rolled = 0;
    for _ in 0..10 {
        later += step;
        app.update(later).unwrap();
        rolled += motion.update(&app, 4);
    }
    assert_eq!(rolled, 10);
    assert!(motion.orbit_complete(&app));
}

#[test]
fn orbit_passes_through_the_selected_marker_and_breaks_behind_the_earth() {
    let mut app = app(100.0);
    let mut motion = Motion::new();
    app.update(start()).unwrap();
    motion.update(&app, usize::MAX);
    let (_, sample) = app.selected().unwrap();
    let marker = app.view().project_unclipped(sample.camera).unwrap();
    let points: Vec<_> = motion.orbit(&app).collect();
    let centre = points[ORBIT_SEGMENTS / 2].unwrap();
    assert!(libm::hypot(centre.east - marker.east, centre.north - marker.north) < 1e-6);
    // A LEO orbit seen from 75,000 km passes behind the Earth for part of it.
    let hidden = points.iter().filter(|p| p.is_none()).count();
    assert!(hidden > 0 && hidden < points.len() / 2, "{hidden}");
}

#[test]
fn cached_trails_match_fresh_ones_and_follow_a_zoom_before_drawing() {
    let mut app = app(60.0);
    let mut motion = Motion::new();
    run(
        &mut app,
        &mut motion,
        start(),
        TimeDelta::minutes(20),
        TimeDelta::seconds(5),
    );
    let cached: Vec<Trail> = motion.trails(&app).collect();
    assert!(!cached.is_empty());
    assert!(motion.trail_cache.iter().any(Option::is_some));
    motion.trail_cache.iter_mut().for_each(|c| *c = None);
    assert_eq!(motion.trails(&app).collect::<Vec<_>>(), cached);
    // Zoomed after the update: trails use the new step, not the cached one.
    motion.update(&app, 0);
    app.zoom(3);
    let zoomed: Vec<Trail> = motion.trails(&app).collect();
    motion.trail_cache.iter_mut().for_each(|c| *c = None);
    assert_eq!(motion.trails(&app).collect::<Vec<_>>(), zoomed);
    motion.update(&app, 0);
    assert_eq!(motion.trails(&app).collect::<Vec<_>>(), zoomed);
}

/// The device draws NO DATA frames before the first catalogue arrives (and
/// after a stored one expires at boot): motion must follow them, not panic.
#[test]
fn frames_without_a_catalogue_are_followed() {
    let observer = GeodeticPosition {
        latitude_rad: 40f64.to_radians(),
        longitude_rad: (-105f64).to_radians(),
        altitude_km: 1.6,
    };
    let view = View::rectangular(20.0, 400.0 / 240.0).unwrap();
    let mut app = RadarApp::awaiting_catalogue(observer, view).unwrap();
    let mut motion = Motion::new();
    let t = run(
        &mut app,
        &mut motion,
        start(),
        TimeDelta::minutes(5),
        TimeDelta::seconds(10),
    );
    assert_eq!(motion.trails(&app).count(), 0);
    assert_eq!(motion.orbit(&app).count(), 0);
    // A catalogue arriving later starts trails as usual.
    app.load_catalogue(visual());
    let end = run(
        &mut app,
        &mut motion,
        t + TimeDelta::seconds(10),
        TimeDelta::minutes(5),
        TimeDelta::seconds(10),
    );
    assert_eq!(app.time(), Some(end));
    assert!(motion.trails(&app).count() > 0);
}
