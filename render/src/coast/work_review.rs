//! Offline source-level work accounting for the unchanged comparison renderer.
//! Not an S3 profiler or a candidate algorithm. Keep this model in sync with
//! `interval`, `crossing` and `stroke`; output equality alone cannot prove counts.
extern crate std;

use super::*;
use crate::coast::cost_preflight::{Screen, observer};
use overhead_core::{
    OmmElements,
    catalogue::{CatalogueBuilder, RecordOrigin},
};
use std::{hint::black_box, time::Instant, vec::Vec};

#[derive(Default)]
pub(super) struct Work {
    intervals: usize,
    hidden: usize,
    roots: usize,
    strokes: usize,
}

impl Work {
    // Counts source-level Camera::coast_point evaluations on successful paths,
    // not emitted calls/instructions (the compiler may inline/share operations).
    pub(super) fn samples(&self) -> usize {
        3 * self.intervals + ROOT_STEPS * self.roots + 2 * self.strokes
    }

    pub(super) fn classify(&mut self, a: bool, mid: bool, b: bool) {
        self.intervals += 1;
        match (a, mid, b) {
            (true, true, true) => self.strokes += 1,
            (true, false, true) => {
                self.roots += 2;
                self.strokes += 2;
            }
            (false, false, false) => self.hidden += 1,
            (false, true, false) => {
                self.roots += 2;
                self.strokes += 1;
            }
            _ => {
                self.roots += 1;
                self.strokes += 1;
            }
        }
    }
}

#[test]
fn successful_interval_work_model_covers_all_visibility_branches() {
    for (a, mid, b, hidden, roots, strokes) in [
        (false, false, false, 1, 0, 0),
        (false, false, true, 0, 1, 1),
        (false, true, false, 0, 2, 1),
        (false, true, true, 0, 1, 1),
        (true, false, false, 0, 1, 1),
        (true, false, true, 0, 2, 2),
        (true, true, false, 0, 1, 1),
        (true, true, true, 0, 0, 1),
    ] {
        let mut work = Work::default();
        work.classify(a, mid, b);
        assert_eq!(
            (work.hidden, work.roots, work.strokes),
            (hidden, roots, strokes)
        );
        assert_eq!(work.samples(), 3 + 24 * roots + 2 * strokes);
    }
}

pub(super) fn cases() -> [(&'static str, GeodeticPosition, f64); 11] {
    // Exact rounded target inputs, not the host preflight's unrounded solvers.
    // Keep aligned with firmware/src/coast_measurement.rs::CASES.
    let elements: Vec<OmmElements> =
        serde_json::from_str(include_str!("../../../core/tests/fixtures/iss-25544.json")).unwrap();
    let mut builder = CatalogueBuilder::new();
    builder.push(
        RecordOrigin {
            group: 0,
            record: 0,
        },
        elements.into_iter().next().unwrap(),
    );
    let catalogue = builder.finish().unwrap();
    let satellite = catalogue.entries()[0].satellite();
    let selected_site = overhead_core::ecef_to_geodetic(
        satellite
            .state_at(satellite.epoch())
            .unwrap()
            .to_ecef()
            .unwrap(),
    )
    .unwrap();
    [
        ("sf-tight", observer(37.7749, -122.4194), 0.1),
        ("sf-1", observer(37.7749, -122.4194), 1.0),
        ("sf-5", observer(37.7749, -122.4194), 5.0),
        ("sf-100", observer(37.7749, -122.4194), 100.0),
        ("colorado-20", observer(39.007, -104.883), 20.0),
        ("polar-20", observer(89.9, 0.0), 20.0),
        ("dateline-5", observer(0.0, 179.9), 5.0),
        ("limb-20", observer(1.364900, -156.229044), 20.0),
        ("limb-100", observer(1.364900, -156.229044), 100.0),
        ("viewport-tight", observer(1.364900, 119.225793), 0.1),
        (
            "iss-ground-20",
            GeodeticPosition {
                altitude_km: 0.0,
                ..selected_site
            },
            20.0,
        ),
    ]
}

#[test]
#[ignore = "explicit offline source-work review; host timing is not S3 evidence"]
fn coast_work_review() {
    let layout = Layout::wide();
    std::println!(
        "case,intervals,hidden,roots,strokes,coast_point_evals,on_writes,validation_median_us,coast_median_us"
    );
    for (name, site, fov) in cases() {
        assert!(valid(ASSET));
        let camera = Camera::new(site).unwrap();
        let view = View::rectangular(fov, layout.aspect_ratio()).unwrap();
        let mut work = Work::default();
        let mut replay = Screen::new();
        for part in 0..PARTS {
            let start = u32_at(ASSET, COORD_BYTES + part * 4).unwrap();
            let end = u32_at(ASSET, COORD_BYTES + (part + 1) * 4).unwrap();
            for index in start..end - 1 {
                let a = point(ASSET, index).unwrap();
                let b = point(ASSET, index + 1).unwrap();
                let count = steps(a, b);
                for step in 0..count {
                    let first = interpolate(a, b, step as f64 / count as f64);
                    let last = interpolate(a, b, (step + 1) as f64 / count as f64);
                    let front = |at| sample(&camera, at).unwrap().1 > 0.0;
                    work.classify(
                        front(first),
                        front(interpolate(first, last, 0.5)),
                        front(last),
                    );
                    // The model assumes successful paths. Exercise each actual
                    // interval and compare traversal/output against full draw.
                    assert!(interval(&mut replay, &camera, view, layout, first, last).unwrap());
                }
            }
        }
        assert_eq!(work.intervals, MAX_TOTAL_STEPS);
        let mut control = Screen::new();
        assert!(draw(&mut control, site, view, layout, ASSET).unwrap());
        assert_eq!(replay.pixels, control.pixels);
        assert_eq!(replay.on_writes, control.on_writes);

        // Warm paths above; validation is batched to reduce clock overhead.
        // Full draw includes validation. These are host observations only.
        let mut validation_us = [0.0_f64; 5];
        let mut coast_us = [0.0_f64; 5];
        for i in 0..5 {
            let start = Instant::now();
            for _ in 0..100 {
                assert!(black_box(valid(black_box(ASSET))));
            }
            validation_us[i] = start.elapsed().as_secs_f64() * 1e6 / 100.0;
            let mut screen = Screen::new(); // blanking excluded, like target Coast
            let start = Instant::now();
            assert!(
                black_box(draw(
                    black_box(&mut screen),
                    black_box(site),
                    black_box(view),
                    layout,
                    black_box(ASSET)
                ))
                .unwrap()
            );
            coast_us[i] = start.elapsed().as_secs_f64() * 1e6;
            black_box(screen);
        }
        validation_us.sort_by(f64::total_cmp);
        coast_us.sort_by(f64::total_cmp);
        std::println!(
            "{name},{},{},{},{},{},{},{:.1},{:.1}",
            work.intervals,
            work.hidden,
            work.roots,
            work.strokes,
            work.samples(),
            control.on_writes,
            validation_us[2],
            coast_us[2]
        );
    }
}
