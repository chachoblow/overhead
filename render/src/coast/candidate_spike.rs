//! Test-only feasibility candidate: coarse conservative rejection, then reuse
//! of evaluated samples on the SAME 0.2-degree geographic/raster path.
//! Not selected for production. Envelopes have pragmatic f64 margins, not a
//! directed-rounding proof. Unknown means render/refine, never discard.
extern crate std;

use super::*;
use super::{
    cost_preflight::Screen,
    work_review::{Work, cases},
};
use overhead_app::projection::NEAR_PLANE_KM;
use std::{hint::black_box, time::Instant};

#[derive(Clone, Copy)]
struct Evaluated {
    geo: Geo,
    camera: CameraPoint,
    facing: f64,
}

#[derive(Default, Debug, PartialEq)]
struct Counts {
    evaluations: usize,
    hidden: usize,
    outside: usize,
    front: usize,
    intervals: usize,
    roots: usize,
    strokes: usize,
}

fn evaluate(camera: &Camera, geo: Geo, counts: &mut Counts) -> Option<Evaluated> {
    counts.evaluations += 1;
    let (camera, facing) = sample(camera, geo)?;
    Some(Evaluated {
        geo,
        camera,
        facing,
    })
}

#[derive(Debug, PartialEq)]
enum Bound {
    Hidden,
    Outside,
    Front,
    Unknown,
}

struct Context<'a> {
    camera: &'a Camera,
    view: View,
    scale: ViewPoint,
    zero_height: bool,
}

impl<'a> Context<'a> {
    fn new(camera: &'a Camera, view: View, site: GeodeticPosition) -> Self {
        // Use the actual stored lens coefficients, not an atan/tan round trip.
        let scale = view
            .project_unclipped(CameraPoint {
                east_km: 1.0,
                north_km: 1.0,
                depth_km: 1.0,
            })
            .unwrap();
        Self {
            camera,
            view,
            scale,
            zero_height: site.altitude_km == 0.0,
        }
    }

    fn bound(&self, a: Geo, b: Geo, counts: &mut Counts) -> Bound {
        // The deliberately loose analytic envelopes below are restricted to
        // zero-height observers. Other heights retain the full interval path.
        if !self.zero_height {
            return Bound::Unknown;
        }
        let Some(mid) = evaluate(self.camera, interpolate(a, b, 0.5), counts) else {
            return Bound::Unknown;
        };
        let mut dl = b.0 - a.0;
        if dl.abs() > 180.0 {
            dl -= 360.0 * dl.signum();
        }
        let h = (dl.abs() + (b.1 - a.1).abs()).to_radians() * 0.5;
        // |dP/dangle| < 7000 km/rad, |dn/dangle| <= 1 and
        // |camera - P| < 100000 km for these surface observers. Therefore
        // |delta P| <= 7000h; |delta facing| <= 107000h. This bounds the
        // continuous lat/lon interpolant, not merely its three test samples.
        let travel = 7000.0 * h + 1e-6;
        let facing_error = 107000.0 * h + 0.01;
        let p = mid.camera;
        let dmin = p.depth_km - travel;
        if ![travel, facing_error, mid.facing, p.depth_km, dmin]
            .iter()
            .all(|x| x.is_finite())
        {
            return Bound::Unknown;
        }
        if mid.facing + facing_error < 0.0 {
            return Bound::Hidden;
        }
        if dmin < NEAR_PLANE_KM {
            return Bound::Unknown;
        }
        let Some(projected) = self.view.project_unclipped(p) else {
            return Bound::Unknown;
        };
        let error = |coordinate: f64, scale: f64| {
            travel * (p.depth_km + coordinate.abs()) / (p.depth_km * dmin) * scale + 1e-9
        };
        let ex = error(p.east_km, self.scale.east);
        let ey = error(p.north_km, self.scale.north);
        if ![ex, ey].iter().all(|x| x.is_finite()) {
            return Bound::Unknown;
        }
        if projected.east.abs() - ex > 1.0 || projected.north.abs() - ey > 1.0 {
            return Bound::Outside;
        }
        if mid.facing - facing_error > 0.0 {
            Bound::Front
        } else {
            Bound::Unknown
        }
    }
}

fn cached_stroke(
    display: &mut Screen,
    view: View,
    a: Evaluated,
    b: Evaluated,
    counts: &mut Counts,
) -> bool {
    counts.strokes += 1;
    let Some((a, b)) = view
        .project_unclipped(a.camera)
        .zip(view.project_unclipped(b.camera))
    else {
        return false;
    };
    if let Some((start, end)) = clip(a, b) {
        if ![start.east, start.north, end.east, end.north]
            .iter()
            .all(|x| x.is_finite())
        {
            return false;
        }
        let pixel = |p: ViewPoint| {
            Layout::wide().marker(ViewPoint {
                east: p.east.clamp(-1.0, 1.0),
                north: p.north.clamp(-1.0, 1.0),
            })
        };
        Line::new(pixel(start), pixel(end))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(display)
            .unwrap();
    }
    true
}

fn root(camera: &Camera, a: Evaluated, b: Evaluated, counts: &mut Counts) -> Option<Evaluated> {
    counts.roots += 1;
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..ROOT_STEPS {
        let mid = (lo + hi) * 0.5;
        if (evaluate(camera, interpolate(a.geo, b.geo, mid), counts)?.facing > 0.0)
            == (a.facing > 0.0)
        {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    evaluate(camera, interpolate(a.geo, b.geo, (lo + hi) * 0.5), counts)
}

fn cached_interval(
    screen: &mut Screen,
    context: &Context<'_>,
    a: Evaluated,
    b: Evaluated,
    front: bool,
    counts: &mut Counts,
) -> Option<bool> {
    if front {
        return Some(cached_stroke(screen, context.view, a, b, counts));
    }
    let mid = evaluate(context.camera, interpolate(a.geo, b.geo, 0.5), counts)?;
    let stroke = |screen: &mut Screen, a, b, counts: &mut Counts| {
        cached_stroke(screen, context.view, a, b, counts)
    };
    match (a.facing > 0.0, mid.facing > 0.0, b.facing > 0.0) {
        (true, true, true) => Some(stroke(screen, a, b, counts)),
        (true, false, true) => {
            let left = root(context.camera, a, mid, counts)?;
            let right = root(context.camera, mid, b, counts)?;
            Some(stroke(screen, a, left, counts) && stroke(screen, right, b, counts))
        }
        (false, false, false) => Some(true),
        (false, true, false) => {
            let left = root(context.camera, a, mid, counts)?;
            let right = root(context.camera, mid, b, counts)?;
            Some(stroke(screen, left, right, counts))
        }
        _ => {
            let crossing = root(context.camera, a, b, counts)?;
            Some(if a.facing > 0.0 {
                stroke(screen, a, crossing, counts)
            } else {
                stroke(screen, crossing, b, counts)
            })
        }
    }
}

fn candidate_segment(
    screen: &mut Screen,
    context: &Context<'_>,
    a: Geo,
    b: Geo,
    counts: &mut Counts,
) -> Option<bool> {
    let front = match context.bound(a, b, counts) {
        Bound::Hidden => {
            counts.hidden += 1;
            return Some(true);
        }
        Bound::Outside => {
            counts.outside += 1;
            return Some(true);
        }
        Bound::Front => {
            counts.front += 1;
            true
        }
        Bound::Unknown => false,
    };
    let n = steps(a, b);
    counts.intervals += n;
    let mut previous = evaluate(context.camera, interpolate(a, b, 0.0), counts)?;
    for step in 0..n {
        let next = evaluate(
            context.camera,
            interpolate(a, b, (step + 1) as f64 / n as f64),
            counts,
        )?;
        if !cached_interval(screen, context, previous, next, front, counts)? {
            return Some(false);
        }
        previous = next;
    }
    Some(true)
}

fn segments(mut visit: impl FnMut(usize, Geo, Geo)) {
    for part in 0..PARTS {
        let start = u32_at(ASSET, COORD_BYTES + part * 4).unwrap();
        let end = u32_at(ASSET, COORD_BYTES + (part + 1) * 4).unwrap();
        for index in start..end - 1 {
            visit(
                index,
                point(ASSET, index).unwrap(),
                point(ASSET, index + 1).unwrap(),
            );
        }
    }
}

fn candidate_draw(screen: &mut Screen, site: GeodeticPosition, view: View) -> Counts {
    // Deliberately retain original asset validation in every timed invocation.
    assert!(valid(ASSET));
    let camera = Camera::new(site).unwrap();
    let context = Context::new(&camera, view, site);
    let mut counts = Counts::default();
    segments(|_, a, b| {
        assert_eq!(
            candidate_segment(screen, &context, a, b, &mut counts),
            Some(true)
        )
    });
    counts
}

#[test]
fn candidate_spike_retains_unknown_and_crossing_segments() {
    for (name, site, fov, a, b) in [
        // Endpoints outside opposite viewport edges must not be culled.
        (
            "viewport",
            cost_preflight::observer(0.0, 0.0),
            0.1,
            (-2.0, 0.0),
            (2.0, 0.0),
        ),
        (
            "dateline",
            cost_preflight::observer(0.0, 180.0),
            0.1,
            (179.0, 0.0),
            (-179.0, 0.0),
        ),
        (
            "limb",
            cost_preflight::observer(0.0, 0.0),
            20.0,
            (80.0, 0.0),
            (90.0, 0.0),
        ),
        (
            "pole",
            cost_preflight::observer(90.0, 0.0),
            0.1,
            (0.0, 89.0),
            (180.0, 89.0),
        ),
    ] {
        let camera = Camera::new(site).unwrap();
        let view = View::rectangular(fov, Layout::wide().aspect_ratio()).unwrap();
        let context = Context::new(&camera, view, site);
        assert!(
            matches!(
                context.bound(a, b, &mut Counts::default()),
                Bound::Unknown | Bound::Front
            ),
            "{name}"
        );
        let mut baseline = Screen::new();
        let n = steps(a, b);
        for k in 0..n {
            assert!(
                interval(
                    &mut baseline,
                    &camera,
                    view,
                    Layout::wide(),
                    interpolate(a, b, k as f64 / n as f64),
                    interpolate(a, b, (k + 1) as f64 / n as f64)
                )
                .unwrap()
            );
        }
        let mut actual = Screen::new();
        assert_eq!(
            candidate_segment(&mut actual, &context, a, b, &mut Counts::default()),
            Some(true)
        );
        assert_eq!(actual.pixels, baseline.pixels, "{name}");
        assert_eq!(actual.on_writes, baseline.on_writes, "{name}");
    }
    let mut elevated = cost_preflight::observer(0.0, 0.0);
    elevated.altitude_km = 2.187;
    let camera = Camera::new(elevated).unwrap();
    let context = Context::new(&camera, View::new(20.0).unwrap(), elevated);
    assert_eq!(
        context.bound((170.0, 0.0), (175.0, 0.0), &mut Counts::default()),
        Bound::Unknown
    );
}

#[test]
#[ignore = "explicit host candidate feasibility experiment; no target viability claim"]
fn coast_candidate_spike() {
    std::println!(
        "case,hidden_segments,outside_segments,front_segments,kept_intervals,baseline_evals,cull_only_evals,reuse_evals,roots,strokes,on_writes,baseline_median_us,candidate_median_us"
    );
    for (name, site, fov) in cases() {
        let view = View::rectangular(fov, Layout::wide().aspect_ratio()).unwrap();
        let camera = Camera::new(site).unwrap();
        let context = Context::new(&camera, view, site);
        let mut baseline_work = Work::default();
        let mut retained_work = Work::default();
        let mut isolated = Screen::new();
        let mut actual = Screen::new();
        // Isolated equality: a neighboring coast cannot mask incorrectly
        // rejected ink. Every one of 1734 segments is checked in every scene.
        segments(|index, a, b| {
            isolated.pixels.fill([0; 400]);
            isolated.on_writes = 0;
            actual.pixels.fill([0; 400]);
            actual.on_writes = 0;
            let bound = context.bound(a, b, &mut Counts::default());
            let n = steps(a, b);
            for k in 0..n {
                let first = interpolate(a, b, k as f64 / n as f64);
                let last = interpolate(a, b, (k + 1) as f64 / n as f64);
                let fa = sample(&camera, first).unwrap().1 > 0.0;
                let fm = sample(&camera, interpolate(first, last, 0.5)).unwrap().1 > 0.0;
                let fb = sample(&camera, last).unwrap().1 > 0.0;
                baseline_work.classify(fa, fm, fb);
                if matches!(bound, Bound::Unknown | Bound::Front) {
                    retained_work.classify(fa, fm, fb);
                }
                assert!(
                    interval(&mut isolated, &camera, view, Layout::wide(), first, last).unwrap()
                );
            }
            assert_eq!(
                candidate_segment(&mut actual, &context, a, b, &mut Counts::default()),
                Some(true)
            );
            assert!(
                actual.pixels == isolated.pixels,
                "isolated pixel mismatch: {name} segment={index}"
            );
            assert_eq!(
                actual.on_writes, isolated.on_writes,
                "isolated writes: {name} segment={index}"
            );
        });
        let mut baseline = Screen::new();
        assert!(draw(&mut baseline, site, view, Layout::wide(), ASSET).unwrap());
        let mut candidate = Screen::new();
        let expected_counts = candidate_draw(&mut candidate, site, view);
        assert!(candidate.pixels == baseline.pixels, "whole coast: {name}");
        assert_eq!(candidate.on_writes, baseline.on_writes);
        let mut baseline_us = [0.0_f64; 5];
        let mut candidate_us = [0.0_f64; 5];
        for i in 0..5 {
            // Blank outside timing. Alternate order to reduce ordering bias.
            let mut run = |candidate_first: bool| {
                let mut screen = Screen::new();
                let start = Instant::now();
                if candidate_first {
                    let counts =
                        candidate_draw(black_box(&mut screen), black_box(site), black_box(view));
                    candidate_us[i] = start.elapsed().as_secs_f64() * 1e6;
                    assert_eq!(counts, expected_counts);
                } else {
                    assert!(
                        black_box(draw(
                            black_box(&mut screen),
                            black_box(site),
                            black_box(view),
                            Layout::wide(),
                            black_box(ASSET)
                        ))
                        .unwrap()
                    );
                    baseline_us[i] = start.elapsed().as_secs_f64() * 1e6;
                }
                assert_eq!(screen.pixels, baseline.pixels);
                black_box(screen);
            };
            run(i % 2 == 0);
            run(i % 2 != 0);
        }
        baseline_us.sort_by(f64::total_cmp);
        candidate_us.sort_by(f64::total_cmp);
        std::println!(
            "{name},{},{},{},{},{},{},{},{},{},{},{:.1},{:.1}",
            expected_counts.hidden,
            expected_counts.outside,
            expected_counts.front,
            expected_counts.intervals,
            baseline_work.samples(),
            retained_work.samples() + 1734,
            expected_counts.evaluations,
            expected_counts.roots,
            expected_counts.strokes,
            candidate.on_writes,
            baseline_us[2],
            candidate_us[2]
        );
    }
}
