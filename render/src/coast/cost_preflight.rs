//! Opt-in host cost probe; `cargo test -p overhead-render --release coast_cost_preflight -- --ignored --nocapture`.
//! Not an S3 benchmark or a numerical visibility test.
extern crate std;

use super::*;
use crate::{DISPLAY_SIZE, draw_frame};
use core::convert::Infallible;
use overhead_app::{RadarApp, projection::Viewport};
use overhead_core::{
    OmmElements,
    catalogue::{CatalogueBuilder, RecordOrigin},
};
use std::{hint::black_box, time::Instant, vec::Vec};

struct Screen {
    pixels: [[u8; 400]; 240],
    writes: usize,
    on_writes: usize,
}

impl Screen {
    fn new() -> Self {
        Self {
            pixels: [[0; 400]; 240],
            writes: 0,
            on_writes: 0,
        }
    }
    fn lit(&self) -> usize {
        self.pixels.iter().flatten().filter(|&&v| v != 0).count()
    }
}
impl OriginDimensions for Screen {
    fn size(&self) -> Size {
        DISPLAY_SIZE
    }
}
impl DrawTarget for Screen {
    type Color = BinaryColor;
    type Error = Infallible;
    fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(
        &mut self,
        pixels: I,
    ) -> Result<(), Self::Error> {
        for Pixel(p, c) in pixels {
            assert!((0..400).contains(&p.x) && (0..240).contains(&p.y));
            let on = usize::from(c == BinaryColor::On);
            self.pixels[p.y as usize][p.x as usize] = on as u8;
            self.writes += 1;
            self.on_writes += on;
        }
        Ok(())
    }
}

fn observer(lat: f64, lon: f64) -> GeodeticPosition {
    GeodeticPosition {
        latitude_rad: lat.to_radians(),
        longitude_rad: lon.to_radians(),
        altitude_km: 0.0,
    }
}

fn app(site: GeodeticPosition, view: View) -> RadarApp {
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
    let time = catalogue.entries()[0].satellite().epoch();
    let mut app = RadarApp::new(catalogue, site, view).unwrap();
    app.update(time).unwrap();
    app
}

// One deterministic exported long segment, with observers solved for the
// midpoint's camera-facing limb and horizontal viewport edge respectively.
// These are work stress cases, NOT a proof about every near-boundary run.
fn boundary_sites(layout: Layout) -> (GeodeticPosition, GeodeticPosition, usize) {
    let fov = 0.1;
    let view = View::rectangular(fov, layout.aspect_ratio()).unwrap();
    for part in 0..PARTS {
        let start = u32_at(ASSET, COORD_BYTES + part * 4).unwrap();
        let end = u32_at(ASSET, COORD_BYTES + (part + 1) * 4).unwrap();
        for index in start..end - 1 {
            let a = point(ASSET, index).unwrap();
            let b = point(ASSET, index + 1).unwrap();
            if index % 23 != 0 || steps(a, b) < 5 || a.1.abs() > 60.0 {
                continue;
            }
            let mid = interpolate(a, b, 0.5);
            let site =
                |offset: f64| observer(mid.1, (mid.0 + offset + 180.0).rem_euclid(360.0) - 180.0);
            let facing = |offset| sample(&Camera::new(site(offset)).unwrap(), mid).unwrap().1;
            if facing(180.0) >= 0.0 {
                continue;
            }
            let (mut lo, mut hi) = (0.0, 180.0);
            for _ in 0..32 {
                let m = (lo + hi) / 2.0;
                if facing(m) > 0.0 {
                    lo = m;
                } else {
                    hi = m;
                }
            }
            let limb = site((lo + hi) / 2.0 - 0.2);
            let east = |offset| {
                let camera = Camera::new(site(offset)).unwrap();
                view.project_unclipped(sample(&camera, mid).unwrap().0)
                    .unwrap()
                    .east
                    .abs()
            };
            let (mut lo, mut hi) = (0.0, 3.0);
            if east(lo) >= 1.0 || east(hi) <= 1.0 {
                continue;
            }
            for _ in 0..32 {
                let m = (lo + hi) / 2.0;
                if east(m) < 1.0 {
                    lo = m;
                } else {
                    hi = m;
                }
            }
            return (limb, site((lo + hi) / 2.0 - 0.08), index);
        }
    }
    panic!("no suitable exported segment for boundary probe");
}

fn run(screen: &mut Screen, app: &RadarApp, mode: &str) {
    let layout = Layout::wide();
    match mode {
        "coast" => assert!(super::draw(screen, app.observer(), app.view(), layout, ASSET).unwrap()),
        "frame+coast" => draw_frame(screen, app, layout, false, None, Some(ASSET)).unwrap(),
        "frame-only" => draw_frame(screen, app, layout, false, None, None).unwrap(),
        "frame-invalid" => {
            draw_frame(screen, app, layout, false, None, Some(&ASSET[..10])).unwrap()
        }
        _ => unreachable!(),
    }
}

#[test]
#[ignore = "explicit, potentially slow release host measurement"]
fn coast_cost_preflight() {
    let layout = Layout::wide();
    let (limb, edge, segment) = boundary_sites(layout);
    std::println!(
        "boundary exported segment index={segment}; limb lat={:.6} lon={:.6}; viewport lat={:.6} lon={:.6}",
        limb.latitude_rad.to_degrees(),
        limb.longitude_rad.to_degrees(),
        edge.latitude_rad.to_degrees(),
        edge.longitude_rad.to_degrees()
    );
    std::println!(
        "checked asset bytes={}; parts={PARTS}; vertices={POINTS}; segments={}; interpolation intervals={MAX_TOTAL_STEPS}; root iterations/crossing={ROOT_STEPS}",
        ASSET.len(),
        POINTS - PARTS
    );
    std::println!("case,mode,min_ms,median_ms,max_ms,pixel_writes,on_writes,final_lit");
    for (name, site, fov) in [
        ("sf-tight", observer(37.7749, -122.4194), 0.1),
        ("sf-1", observer(37.7749, -122.4194), 1.0),
        ("sf-5", observer(37.7749, -122.4194), 5.0),
        ("sf-100", observer(37.7749, -122.4194), 100.0),
        ("colorado-20", observer(39.007, -104.883), 20.0),
        ("polar-20", observer(89.9, 0.0), 20.0),
        ("dateline-5", observer(0.0, 179.9), 5.0),
        ("limb-20", limb, 20.0),
        ("limb-100", limb, 100.0),
        ("viewport-tight", edge, 0.1),
    ] {
        let view = View::rectangular(fov, layout.aspect_ratio()).unwrap();
        let app = app(site, view);
        assert_eq!(app.view().viewport(), Viewport::Rectangle);
        for mode in ["coast", "frame+coast", "frame-only", "frame-invalid"] {
            let mut screen = Screen::new();
            run(&mut screen, &app, mode); // warm-up, excluded
            let mut samples = [0.0_f64; 5];
            for ms in &mut samples {
                let start = Instant::now();
                run(black_box(&mut screen), black_box(&app), black_box(mode));
                *ms = start.elapsed().as_secs_f64() * 1000.0;
                black_box(screen.lit());
            }
            samples.sort_by(f64::total_cmp);
            let mut counted = Screen::new();
            run(&mut counted, &app, mode); // excluded write accounting
            std::println!(
                "{name},{mode},{:.3},{:.3},{:.3},{},{},{}",
                samples[0],
                samples[2],
                samples[4],
                counted.writes,
                counted.on_writes,
                counted.lit()
            );
        }
    }
}
