//! Host-only screen-effect controls for selected and all exported segments.
//! Analytic derivative envelopes are independent of the renderer's 0.2°
//! sampling. f64 margins are pragmatic, NOT directed-rounding certification.
//! The short-run arithmetic audit below rounds algebra outward but explicitly
//! leaves shared transcendental/camera inputs uncertified.
extern crate std;

use super::*;
use overhead_app::projection::Viewport;
use std::{collections::BTreeSet, vec::Vec};

mod rounded_projection;

struct Ink([[bool; 400]; 240]);
impl OriginDimensions for Ink {
    fn size(&self) -> Size {
        crate::DISPLAY_SIZE
    }
}
impl DrawTarget for Ink {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;
    fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(
        &mut self,
        pixels: I,
    ) -> Result<(), Self::Error> {
        for Pixel(p, color) in pixels {
            assert!((0..400).contains(&p.x) && (0..240).contains(&p.y));
            self.0[p.y as usize][p.x as usize] = color == BinaryColor::On;
        }
        Ok(())
    }
}

fn site(lat: f64, lon: f64) -> GeodeticPosition {
    GeodeticPosition {
        latitude_rad: lat.to_radians(),
        longitude_rad: lon.to_radians(),
        altitude_km: 0.0,
    }
}

#[derive(Default)]
struct Counts {
    hidden: usize,
    outside: usize,
    pixels: BTreeSet<(i32, i32)>,
    possible_pixels: BTreeSet<(i32, i32)>,
    unbounded_unknown: usize,
    unknown: usize,
    unknown_fraction: f64,
    nodes: usize,
}

enum Bound {
    Hidden,
    Outside,
    Pixel(Point),
    Possible(Point, Point),
    Unknown,
}

// For zero-height observers and points, |dECEF/dlon|,|dECEF/dlat| < 7000
// km/rad on WGS-84; |camera - surface| < 100000 km and |dn/dangle| <= 1.
// Thus, for any point on the parameter subinterval, |delta P| <= 7000*h,
// |delta facing| <= 107000*h, with h = (|dlon|+|dlat|)*half-width.
// Ratio error uses D >= Dmid - |delta P|. Inflate for host arithmetic;
// these margins are not a machine-checked f64/rounding bound.
struct Study<'a> {
    camera: &'a Camera,
    view: View,
    layout: Layout,
    a: Geo,
    b: Geo,
}

fn enclosure(study: &Study<'_>, lo: f64, hi: f64) -> Bound {
    let Study {
        camera,
        view,
        layout,
        a,
        b,
    } = *study;
    let mut dlon = b.0 - a.0;
    if dlon.abs() > 180.0 {
        dlon -= 360.0 * dlon.signum();
    }
    let h = (dlon.abs() + (b.1 - a.1).abs()).to_radians() * (hi - lo) * 0.5;
    let travel = 7000.0 * h + 1e-6;
    let facing_error = 107000.0 * h + 0.01;
    let mid = interpolate(a, b, (lo + hi) * 0.5);
    let Some((p, facing)) = sample(camera, mid) else {
        return Bound::Unknown;
    };
    if facing + facing_error < 0.0 {
        return Bound::Hidden;
    }
    let dmin = p.depth_km - travel;
    if dmin < overhead_app::projection::NEAR_PLANE_KM {
        return Bound::Unknown;
    }
    let Some(projected) = view.project_unclipped(p) else {
        return Bound::Unknown;
    };
    let vertical = (view.fov_deg().to_radians() * 0.5).tan();
    let e_err = travel * (p.depth_km + p.east_km.abs())
        / (p.depth_km * dmin * vertical * layout.aspect_ratio())
        + 1e-9;
    let n_err = travel * (p.depth_km + p.north_km.abs()) / (p.depth_km * dmin * vertical) + 1e-9;
    if ![e_err, n_err].iter().all(|v| v.is_finite()) {
        return Bound::Unknown;
    }
    if projected.east.abs() - e_err > 1.0 || projected.north.abs() - n_err > 1.0 {
        return Bound::Outside;
    }
    // Retain possible screen cells even when visibility is uncertain. Unknown
    // is never silently reclassified as hidden; it may have existing ink.
    let lower = layout.marker(ViewPoint {
        east: (projected.east - e_err).clamp(-1.0, 1.0),
        north: (projected.north - n_err).clamp(-1.0, 1.0),
    });
    let upper = layout.marker(ViewPoint {
        east: (projected.east + e_err).clamp(-1.0, 1.0),
        north: (projected.north + n_err).clamp(-1.0, 1.0),
    });
    if facing - facing_error <= 0.0
        || projected.east.abs() + e_err >= 1.0
        || projected.north.abs() + n_err >= 1.0
    {
        return Bound::Possible(lower, upper);
    }
    if lower != upper
        || !layout.contains(lower, Viewport::Rectangle)
        || lower.x < 4
        || lower.x >= 348
        || lower.y < 4
        || lower.y >= 236
    {
        return Bound::Possible(lower, upper);
    }
    Bound::Pixel(lower)
}

fn record_possible(result: &mut Counts, a: Point, b: Point) {
    for y in a.y.min(b.y)..=a.y.max(b.y) {
        for x in a.x.min(b.x)..=a.x.max(b.x) {
            result.possible_pixels.insert((x, y));
        }
    }
}

fn ink_at(actual: &Ink, x: i32, y: i32) -> bool {
    (0..400).contains(&x) && (0..240).contains(&y) && actual.0[y as usize][x as usize]
}

fn covered(a: Point, b: Point, actual: &Ink, radius: i32) -> bool {
    (a.y.min(b.y)..=a.y.max(b.y)).all(|y| {
        (a.x.min(b.x)..=a.x.max(b.x)).all(|x| {
            (-radius..=radius).any(|dy| (-radius..=radius).any(|dx| ink_at(actual, x + dx, y + dy)))
        })
    })
}

fn visit(
    study: &Study<'_>,
    lo: f64,
    hi: f64,
    depth: u8,
    result: &mut Counts,
    coverage: Option<(&Ink, i32)>,
) {
    result.nodes += 1;
    // Finite study budget; uncertain leaves NEVER become hidden/certified.
    if result.nodes > 200_000 {
        result.unknown += 1;
        result.unbounded_unknown += 1;
        result.unknown_fraction += hi - lo;
        return;
    }
    let bound = enclosure(study, lo, hi);
    if let (Bound::Possible(a, b), Some((actual, radius))) = (&bound, coverage)
        && covered(*a, *b, actual, radius)
    {
        // Facing/visibility stays unknown; ALL possibly visible screen cells
        // are demonstrably already covered by ink. No need to refine further.
        result.unknown += 1;
        result.unknown_fraction += hi - lo;
        record_possible(result, *a, *b);
        return;
    }
    match bound {
        Bound::Hidden => result.hidden += 1,
        Bound::Outside => result.outside += 1,
        Bound::Pixel(p) => {
            result.pixels.insert((p.x, p.y));
        }
        unresolved @ (Bound::Unknown | Bound::Possible(_, _)) if depth >= 22 => {
            result.unknown += 1;
            result.unknown_fraction += hi - lo;
            if let Bound::Possible(a, b) = unresolved {
                record_possible(result, a, b);
            } else {
                result.unbounded_unknown += 1;
            }
        }
        Bound::Unknown | Bound::Possible(_, _) => {
            let mid = (lo + hi) * 0.5;
            visit(study, lo, mid, depth + 1, result, coverage);
            visit(study, mid, hi, depth + 1, result, coverage);
        }
    }
}

fn isolated(study: &Study<'_>) -> Ink {
    let mut actual = Ink([[false; 400]; 240]);
    let n = steps(study.a, study.b);
    for step in 0..n {
        assert!(
            interval(
                &mut actual,
                study.camera,
                study.view,
                study.layout,
                interpolate(study.a, study.b, step as f64 / n as f64),
                interpolate(study.a, study.b, (step + 1) as f64 / n as f64)
            )
            .unwrap()
        );
    }
    actual
}

fn missing_pixels(pixels: &BTreeSet<(i32, i32)>, actual: &Ink, radius: i32) -> Vec<(i32, i32)> {
    pixels
        .iter()
        .copied()
        .filter(|&(x, y)| {
            !(-radius..=radius)
                .any(|dy| (-radius..=radius).any(|dx| ink_at(actual, x + dx, y + dy)))
        })
        .collect()
}

fn missing(result: &Counts, actual: &Ink, radius: i32) -> Vec<(i32, i32)> {
    missing_pixels(&result.pixels, actual, radius)
}

fn compare(study: &Study<'_>) -> (Counts, Vec<(i32, i32)>) {
    let actual = isolated(study);
    let mut result = Counts::default();
    visit(study, 0.0, 1.0, 0, &mut result, None);
    let absent = missing(&result, &actual, 3);
    (result, absent)
}

#[test]
fn conservative_export_segment_screen_effect_study() {
    // Stratified long segment used by the cost preflight's reproducible
    // boundary observer solver; deliberately not a whole-asset claim.
    let index = 46;
    let a = point(ASSET, index).unwrap();
    let b = point(ASSET, index + 1).unwrap();
    assert!(steps(a, b) >= 5);
    let mid = interpolate(a, b, 0.5);
    let site_at = |offset: f64| site(mid.1, (mid.0 + offset + 180.0).rem_euclid(360.0) - 180.0);
    let layout = Layout::wide();
    let tight = View::rectangular(0.1, layout.aspect_ratio()).unwrap();
    let facing = |offset| {
        sample(&Camera::new(site_at(offset)).unwrap(), mid)
            .unwrap()
            .1
    };
    assert!(facing(0.0) > 0.0 && facing(180.0) < 0.0);
    let (mut low, mut high) = (0.0, 180.0);
    for _ in 0..32 {
        let m = (low + high) * 0.5;
        if facing(m) > 0.0 {
            low = m;
        } else {
            high = m;
        }
    }
    let limb = site_at((low + high) * 0.5 - 0.2);
    let east = |offset| {
        let camera = Camera::new(site_at(offset)).unwrap();
        tight
            .project_unclipped(sample(&camera, mid).unwrap().0)
            .unwrap()
            .east
            .abs()
    };
    let (mut low, mut high) = (0.0, 3.0);
    assert!(east(low) < 1.0 && east(high) > 1.0);
    for _ in 0..32 {
        let m = (low + high) * 0.5;
        if east(m) < 1.0 {
            low = m;
        } else {
            high = m;
        }
    }
    let edge = site_at((low + high) * 0.5 - 0.08);
    for (name, observer, fov) in [
        ("center", site_at(0.0), 0.1),
        ("viewport", edge, 0.1),
        ("limb", limb, 100.0),
    ] {
        let view = View::rectangular(fov, layout.aspect_ratio()).unwrap();
        let camera = Camera::new(observer).unwrap();
        let study = Study {
            camera: &camera,
            view,
            layout,
            a,
            b,
        };
        let (result, absent) = compare(&study);
        let actual = isolated(&study);
        let possible_misses = missing_pixels(&result.possible_pixels, &actual, 3);
        assert!(
            !result.pixels.is_empty(),
            "{name}: no certified visible cell"
        );
        std::println!(
            "{name}: segment={index}, observer={observer:?}, fov={fov}, certified_cells={}, hidden_leaves={}, outside_leaves={}, unknown_leaves={}, unknown_fraction={:.6}, nodes={}, possible_cells={}, unbounded_unknown={}, uncovered_possible={}, missing={absent:?}",
            result.pixels.len(),
            result.hidden,
            result.outside,
            result.unknown,
            result.unknown_fraction,
            result.nodes,
            result.possible_pixels.len(),
            result.unbounded_unknown,
            possible_misses.len()
        );
        assert!(
            absent.is_empty(),
            "bounded visible cell absent from isolated export segment"
        );
        // A possibly visible interval remains unknown even if every possible
        // raster cell is already covered by the isolated segment's ink.
        assert_eq!(result.unbounded_unknown, 0);
        assert!(
            possible_misses.is_empty(),
            "possible screen effect lacks nearby ink"
        );
        if name == "center" {
            // Negative control: the comparison must detect an erased resolved
            // run, not merely report certified pixels without inspecting ink.
            let (x, y) = *result.pixels.iter().next().unwrap();
            let mut erased = isolated(&study);
            for dy in -3..=3 {
                for dx in -3..=3 {
                    erased.0[(y + dy) as usize][(x + dx) as usize] = false;
                }
            }
            assert!(missing(&result, &erased, 3).contains(&(x, y)));
        }
    }
}

// Scan every checked-export segment at named scenes. A possibly visible
// screen region may stop subdividing ONLY if every pixel it could affect
// already has isolated segment ink nearby. This is not an all-scene theorem.
#[test]
fn named_scene_all_export_segments_preflight() {
    let layout = Layout::wide();
    for (name, observer, fov) in [
        ("sf-tight", site(37.7749, -122.4194), 0.1),
        ("sf-1", site(37.7749, -122.4194), 1.0),
        ("sf-local", site(37.7749, -122.4194), 5.0),
        ("sf-20", site(37.7749, -122.4194), 20.0),
        ("sf-wide", site(37.7749, -122.4194), 100.0),
        ("polar", site(89.9, 0.0), 20.0),
        ("dateline", site(0.0, 179.9), 5.0),
    ] {
        let camera = Camera::new(observer).unwrap();
        let view = View::rectangular(fov, layout.aspect_ratio()).unwrap();
        let (mut segments, mut reference, mut possible, mut unbounded) = (0, 0, 0, 0);
        let mut gaps = Vec::new();
        let mut capped = Vec::new();
        let mut negative_checked = false;
        for part in 0..PARTS {
            let start = u32_at(ASSET, COORD_BYTES + part * 4).unwrap();
            let end = u32_at(ASSET, COORD_BYTES + (part + 1) * 4).unwrap();
            for index in start..end - 1 {
                let study = Study {
                    camera: &camera,
                    view,
                    layout,
                    a: point(ASSET, index).unwrap(),
                    b: point(ASSET, index + 1).unwrap(),
                };
                let actual = isolated(&study);
                let mut result = Counts::default();
                visit(&study, 0.0, 1.0, 0, &mut result, Some((&actual, 3)));
                let missing3 = missing(&result, &actual, 3);
                let maybe_gap = missing_pixels(&result.possible_pixels, &actual, 3);
                if !negative_checked && !result.possible_pixels.is_empty() {
                    // Full-export negative control for the covered-unknown
                    // shortcut: with the same segment blank, it must report a
                    // missing cell or leave the case unresolved, never pass.
                    let blank = Ink([[false; 400]; 240]);
                    let mut erased = Counts::default();
                    visit(&study, 0.0, 1.0, 0, &mut erased, Some((&blank, 3)));
                    assert!(
                        !missing(&erased, &blank, 3).is_empty()
                            || !missing_pixels(&erased.possible_pixels, &blank, 3).is_empty()
                            || erased.unbounded_unknown > 0,
                        "blank isolated ink falsely passed segment={index} in {name}"
                    );
                    negative_checked = true;
                }
                if !missing3.is_empty() || !maybe_gap.is_empty() {
                    gaps.push((part, index, missing3.len(), maybe_gap.len()));
                }
                reference += result.pixels.len();
                possible += result.possible_pixels.len();
                unbounded += result.unbounded_unknown;
                if result.unbounded_unknown > 0 {
                    capped.push((part, index, result.unbounded_unknown));
                }
                segments += 1;
            }
        }
        std::println!(
            "{name}: segments={segments}, reference={reference}, possible={possible}, unbounded_leaves={unbounded}, capped_first={:?}, suspected_segment_gaps={}, first={:?}",
            capped.iter().take(8).collect::<Vec<_>>(),
            gaps.len(),
            gaps.iter().take(12).collect::<Vec<_>>()
        );
        assert_eq!(segments, POINTS - PARTS);
        assert!(
            negative_checked,
            "no segment exercised ink coverage in {name}"
        );
        assert!(gaps.is_empty(), "uncovered possible screen cells in {name}");
        assert_eq!(unbounded, 0, "non-enclosed unknown leaves in {name}");
    }
}

// Falsification control for the enclosure itself. Sampling cannot certify a
// continuum (or libm rounding), but a wrongly classified *sample* must fail.
#[test]
fn enclosure_never_misclassifies_stratified_samples() {
    let layout = Layout::wide();
    let mut definite = 0;
    for index in [0, 46, 552, 575, 621, 667] {
        let a = point(ASSET, index).unwrap();
        let b = point(ASSET, index + 1).unwrap();
        let origin = interpolate(a, b, 0.5);
        for (site_offset, fov) in [(0.0, 0.1), (0.07, 0.1), (40.0, 20.0), (85.0, 100.0)] {
            let observer = site(
                origin.1,
                (origin.0 + site_offset + 180.0).rem_euclid(360.0) - 180.0,
            );
            let camera = Camera::new(observer).unwrap();
            let view = View::rectangular(fov, layout.aspect_ratio()).unwrap();
            let study = Study {
                camera: &camera,
                view,
                layout,
                a,
                b,
            };
            for i in 0..64 {
                let (lo, hi) = (i as f64 / 64.0, (i + 1) as f64 / 64.0);
                let bound = enclosure(&study, lo, hi);
                if matches!(bound, Bound::Unknown) {
                    continue;
                }
                definite += 1;
                for j in 0..=8 {
                    let at = interpolate(a, b, lo + (hi - lo) * j as f64 / 8.0);
                    let (p, facing) = sample(&camera, at).unwrap();
                    let projected = view.project_unclipped(p).unwrap();
                    match bound {
                        Bound::Hidden => assert!(facing < 0.0, "segment={index}, t={lo}..{hi}"),
                        Bound::Outside => assert!(
                            projected.east.abs() > 1.0 || projected.north.abs() > 1.0,
                            "segment={index}, t={lo}..{hi}"
                        ),
                        Bound::Pixel(pixel) => {
                            assert!(facing > 0.0);
                            assert!(view.viewport().contains(projected));
                            assert_eq!(layout.marker(projected), pixel);
                        }
                        Bound::Possible(a, b) => {
                            if facing > 0.0 && view.viewport().contains(projected) {
                                let pixel = layout.marker(projected);
                                assert!((a.x.min(b.x)..=a.x.max(b.x)).contains(&pixel.x));
                                assert!((a.y.min(b.y)..=a.y.max(b.y)).contains(&pixel.y));
                            }
                        }
                        Bound::Unknown => unreachable!(),
                    }
                }
            }
        }
    }
    assert!(definite > 100, "enclosures did not exercise definite cases");
}

// Seek short, screen-relevant edge runs in four DIFFERENT checked export
// segments. Sites are constructed from a near-endpoint at the 0.1° horizontal
// boundary and moved 0.001–0.004° beyond it. A one-pixel allowance is used
// here: three pixels could mask the entire short run.
#[test]
fn bounded_short_export_runs_near_viewport_edge() {
    let layout = Layout::wide();
    let view = View::rectangular(0.1, layout.aspect_ratio()).unwrap();
    for (index, t, outward) in [
        (552, 0.95, 0.004),
        (575, 0.95, 0.004),
        (621, 0.05, 0.004),
        (667, 0.95, 0.001),
    ] {
        let a = point(ASSET, index).unwrap();
        let b = point(ASSET, index + 1).unwrap();
        let origin = interpolate(a, b, t);
        let at = |offset: f64| {
            site(
                origin.1,
                (origin.0 + offset + 180.0).rem_euclid(360.0) - 180.0,
            )
        };
        let east = |offset: f64| {
            let camera = Camera::new(at(offset)).unwrap();
            view.project_unclipped(sample(&camera, origin).unwrap().0)
                .unwrap()
                .east
                .abs()
        };
        let (mut lo, mut hi) = (0.0, 3.0);
        assert!(east(lo) < 1.0 && east(hi) > 1.0);
        for _ in 0..32 {
            let mid = (lo + hi) * 0.5;
            if east(mid) < 1.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let observer = at((lo + hi) * 0.5 + outward);
        let camera = Camera::new(observer).unwrap();
        let study = Study {
            camera: &camera,
            view,
            layout,
            a,
            b,
        };
        let (result, missing3) = compare(&study);
        let actual = isolated(&study);
        let missing1 = missing(&result, &actual, 1);
        let possible_misses = missing_pixels(&result.possible_pixels, &actual, 1);
        std::println!(
            "short segment={index}, observer={observer:?}, certified_cells={}, unknown_leaves={}, unknown_fraction={:.6}, nodes={}, possible_cells={}, unbounded_unknown={}, missing_1px={missing1:?}, uncovered_possible_count={}, first_uncovered={:?}",
            result.pixels.len(),
            result.unknown,
            result.unknown_fraction,
            result.nodes,
            result.possible_pixels.len(),
            result.unbounded_unknown,
            possible_misses.len(),
            possible_misses.iter().take(5).collect::<Vec<_>>()
        );
        assert!(
            (2..=8).contains(&result.pixels.len()),
            "site no longer probes a short run"
        );
        assert!(
            missing3.is_empty()
                && missing1.is_empty()
                && possible_misses.is_empty()
                && result.unbounded_unknown == 0,
            "short visible run absent or possible screen effect not covered by ink"
        );
    }
}
