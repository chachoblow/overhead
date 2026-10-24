//! Bounded, allocation-free first *comparison* of the reviewed 40 km geography.
//! Approximate angular strokes, not a geographic accuracy or target-time guarantee.
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle},
};
use overhead_app::projection::{Camera, CameraPoint, View, ViewPoint};
use overhead_core::GeodeticPosition;

use crate::Layout;

pub(super) const ASSET: &[u8] = include_bytes!("../assets/coastline-110m-40km.bin");
const PARTS: usize = 134;
const POINTS: usize = 1868;
const COORD_BYTES: usize = POINTS * 8;
const STEP_DEG: f64 = 0.2;
// Checked on the decoded pinned asset by the offline generator.
const MAX_STEPS: usize = 108;
const MAX_TOTAL_STEPS: usize = 23_377;
const ROOT_STEPS: usize = 24;

type Geo = (f64, f64); // longitude, latitude in degrees

fn u32_at(asset: &[u8], offset: usize) -> Option<usize> {
    Some(u32::from_le_bytes(asset.get(offset..offset + 4)?.try_into().ok()?) as usize)
}

fn point(asset: &[u8], index: usize) -> Option<Geo> {
    let offset = index.checked_mul(8)?;
    let x = i32::from_le_bytes(asset.get(offset..offset + 4)?.try_into().ok()?);
    let y = i32::from_le_bytes(asset.get(offset + 4..offset + 8)?.try_into().ok()?);
    Some((f64::from(x) / 10_000.0, f64::from(y) / 10_000.0))
}

// Do not start writing any pixels until every part and vertex has been checked.
fn valid(asset: &[u8]) -> bool {
    if asset.len() != COORD_BYTES + (PARTS + 1) * 4
        || u32_at(asset, COORD_BYTES) != Some(0)
        || u32_at(asset, COORD_BYTES + PARTS * 4) != Some(POINTS)
    {
        return false;
    }
    for part in 0..PARTS {
        let (Some(a), Some(b)) = (
            u32_at(asset, COORD_BYTES + part * 4),
            u32_at(asset, COORD_BYTES + (part + 1) * 4),
        ) else {
            return false;
        };
        if a >= b || b > POINTS {
            return false;
        }
    }
    if !(0..POINTS).all(|i| matches!(point(asset, i), Some((lon, lat)) if (-180.0..=180.0).contains(&lon) && (-90.0..=90.0).contains(&lat))) {
        return false;
    }
    let mut total = 0;
    for part in 0..PARTS {
        let start = u32_at(asset, COORD_BYTES + part * 4).unwrap();
        let end = u32_at(asset, COORD_BYTES + (part + 1) * 4).unwrap();
        for index in start..end - 1 {
            let steps = steps(
                point(asset, index).unwrap(),
                point(asset, index + 1).unwrap(),
            );
            if steps > MAX_STEPS {
                return false;
            }
            total += steps;
            if total > MAX_TOTAL_STEPS {
                return false;
            }
        }
    }
    total == MAX_TOTAL_STEPS
}

fn steps(a: Geo, b: Geo) -> usize {
    let mut dl = b.0 - a.0;
    if dl.abs() > 180.0 {
        dl -= 360.0 * dl.signum();
    }
    (libm::ceil(libm::hypot(dl, b.1 - a.1) / STEP_DEG) as usize).max(1)
}

fn interpolate(a: Geo, b: Geo, t: f64) -> Geo {
    let mut delta = b.0 - a.0;
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta < -180.0 {
        delta += 360.0;
    }
    // Permit unwrapped longitude near the dateline; normalize for WGS-84 input.
    let longitude = a.0 + t * delta;
    (
        if longitude > 180.0 {
            longitude - 360.0
        } else if longitude < -180.0 {
            longitude + 360.0
        } else {
            longitude
        },
        a.1 + t * (b.1 - a.1),
    )
}

fn sample(camera: &Camera, at: Geo) -> Option<(CameraPoint, f64)> {
    camera.coast_point(at.0, at.1)
}

fn crossing(camera: &Camera, a: Geo, b: Geo, front_at_a: bool) -> Option<Geo> {
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..ROOT_STEPS {
        let mid = (lo + hi) * 0.5;
        if (sample(camera, interpolate(a, b, mid))?.1 > 0.0) == front_at_a {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(interpolate(a, b, (lo + hi) * 0.5))
}

// Liang–Barsky clip in normalized lens coordinates, before rounding to pixels.
fn clip(a: ViewPoint, b: ViewPoint) -> Option<(ViewPoint, ViewPoint)> {
    let dx = b.east - a.east;
    let dy = b.north - a.north;
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for (p, q) in [
        (-dx, a.east + 1.0),
        (dx, 1.0 - a.east),
        (-dy, a.north + 1.0),
        (dy, 1.0 - a.north),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else if p < 0.0 {
            lo = lo.max(q / p);
        } else {
            hi = hi.min(q / p);
        }
        if lo > hi {
            return None;
        }
    }
    Some((
        ViewPoint {
            east: a.east + lo * dx,
            north: a.north + lo * dy,
        },
        ViewPoint {
            east: a.east + hi * dx,
            north: a.north + hi * dy,
        },
    ))
}

fn stroke<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    camera: &Camera,
    view: View,
    layout: Layout,
    a: Geo,
    b: Geo,
) -> Result<bool, D::Error> {
    let Some((pa, pb)) = sample(camera, a).zip(sample(camera, b)) else {
        return Ok(false);
    };
    let Some((pa, pb)) = view
        .project_unclipped(pa.0)
        .zip(view.project_unclipped(pb.0))
    else {
        return Ok(false);
    };
    if let Some((start, end)) = clip(pa, pb) {
        if ![start.east, start.north, end.east, end.north]
            .iter()
            .all(|v| v.is_finite())
        {
            return Ok(false);
        }
        // Clipping arithmetic can land just beyond ±1 by a rounding ulp.
        // Clamp only after a finite clip, not as a substitute for visibility.
        let inside = |p: ViewPoint| ViewPoint {
            east: p.east.clamp(-1.0, 1.0),
            north: p.north.clamp(-1.0, 1.0),
        };
        let a = layout.marker(inside(start));
        let b = layout.marker(inside(end));
        Line::new(a, b)
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(display)?;
    }
    Ok(true)
}

fn interval<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    camera: &Camera,
    view: View,
    layout: Layout,
    a: Geo,
    b: Geo,
) -> Result<bool, D::Error> {
    let midpoint = interpolate(a, b, 0.5);
    let Some((fa, fm, fb)) = sample(camera, a)
        .zip(sample(camera, midpoint))
        .zip(sample(camera, b))
        .map(|((a, m), b)| (a.1 > 0.0, m.1 > 0.0, b.1 > 0.0))
    else {
        return Ok(false);
    };
    // Inspect the middle as well as the ends: both hidden endpoints can enclose
    // a visible run. No joining across hidden, part or clipped-out intervals.
    if fa && fb {
        if fm {
            return stroke(display, camera, view, layout, a, b);
        }
        let Some((left, right)) =
            crossing(camera, a, midpoint, true).zip(crossing(camera, midpoint, b, false))
        else {
            return Ok(false);
        };
        Ok(stroke(display, camera, view, layout, a, left)?
            && stroke(display, camera, view, layout, right, b)?)
    } else if !fa && !fb {
        if !fm {
            return Ok(true);
        }
        let Some((left, right)) =
            crossing(camera, a, midpoint, false).zip(crossing(camera, midpoint, b, true))
        else {
            return Ok(false);
        };
        stroke(display, camera, view, layout, left, right)
    } else {
        let Some(root) = crossing(camera, a, b, fa) else {
            return Ok(false);
        };
        if fa {
            stroke(display, camera, view, layout, a, root)
        } else {
            stroke(display, camera, view, layout, root, b)
        }
    }
}

/// Returns false only for an invalid asset or a failed geometric calculation.
/// Caller must clear/repaint without coast before showing COAST ERROR: an error
/// discovered after a stroke must not leave partial geography in a good frame.
pub(super) fn draw<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    observer: GeodeticPosition,
    view: View,
    layout: Layout,
    asset: &[u8],
) -> Result<bool, D::Error> {
    draw_with_hook(display, observer, view, layout, asset, || false)
}

// Production supplies a constant-false hook; no runtime fault injector or
// changed failure policy. Tests may stop a successful interval after it has
// written ink, without making a draw-target error (which fails the frame).
pub(super) fn draw_with_hook<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    observer: GeodeticPosition,
    view: View,
    layout: Layout,
    asset: &[u8],
    mut fail_after_interval: impl FnMut() -> bool,
) -> Result<bool, D::Error> {
    if !valid(asset) {
        return Ok(false);
    }
    let Ok(camera) = Camera::new(observer) else {
        return Ok(false);
    };
    for part in 0..PARTS {
        let start = u32_at(asset, COORD_BYTES + part * 4).unwrap();
        let end = u32_at(asset, COORD_BYTES + (part + 1) * 4).unwrap();
        for i in start..end - 1 {
            let a = point(asset, i).unwrap();
            let b = point(asset, i + 1).unwrap();
            let steps = steps(a, b);
            for step in 0..steps {
                let first = interpolate(a, b, step as f64 / steps as f64);
                let last = interpolate(a, b, (step + 1) as f64 / steps as f64);
                if !interval(display, &camera, view, layout, first, last)? || fail_after_interval()
                {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
mod candidate_spike;
#[cfg(test)]
mod cost_preflight;
#[cfg(any(test, feature = "representation-measurement"))]
pub(crate) mod representation_spike;
#[cfg(test)]
mod visibility_bound_study;
#[cfg(test)]
mod work_review;

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use overhead_app::projection::Viewport;

    struct Screen([[bool; 400]; 240]);
    impl OriginDimensions for Screen {
        fn size(&self) -> Size {
            crate::DISPLAY_SIZE
        }
    }
    impl DrawTarget for Screen {
        type Color = BinaryColor;
        type Error = core::convert::Infallible;
        fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(
            &mut self,
            pixels: I,
        ) -> Result<(), Self::Error> {
            for Pixel(point, color) in pixels {
                assert!((0..400).contains(&point.x) && (0..240).contains(&point.y));
                self.0[point.y as usize][point.x as usize] = color == BinaryColor::On;
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

    // Independent *dense point* control on the same checked export. This is not
    // independent reference geography or a proof between sample points. Compare
    // only confidently front-facing, in-viewport samples, allowing raster and
    // straight-chord approximation to differ by a few pixels. Report the part
    // and segment rather than accepting an empty geography layer as a pass.
    fn check_dense_visible_samples(site: GeodeticPosition, fov: f64) {
        let layout = Layout::wide();
        let view = View::rectangular(fov, layout.aspect_ratio()).unwrap();
        let camera = Camera::new(site).unwrap();
        let mut screen = Screen([[false; 400]; 240]);
        assert!(draw(&mut screen, site, view, layout, ASSET).unwrap());
        let mut checked = 0;
        for part in 0..PARTS {
            let start = u32_at(ASSET, COORD_BYTES + part * 4).unwrap();
            let end = u32_at(ASSET, COORD_BYTES + (part + 1) * 4).unwrap();
            for index in start..end - 1 {
                let a = point(ASSET, index).unwrap();
                let b = point(ASSET, index + 1).unwrap();
                // Eight times finer than the runtime's maximum angular step;
                // use interior samples so the same endpoints cannot certify it.
                let count = steps(a, b) * 8;
                for k in 1..count {
                    let at = interpolate(a, b, k as f64 / count as f64);
                    let (camera_point, facing) = sample(&camera, at).unwrap();
                    if facing <= 1.0 {
                        continue;
                    }
                    let Some(projected) = view.project_unclipped(camera_point) else {
                        continue;
                    };
                    let pixel = layout.marker(projected);
                    if pixel.x < 4 || pixel.x >= 348 || pixel.y < 4 || pixel.y >= 236 {
                        continue;
                    }
                    checked += 1;
                    assert!(
                        (-3..=3).any(|dy| (-3..=3).any(|dx| {
                            screen.0[(pixel.y + dy) as usize][(pixel.x + dx) as usize]
                        })),
                        "missing dense coast sample: FOV={fov} part={part} segment={index} t={k}/{count} pixel={pixel:?}"
                    );
                }
            }
        }
        assert!(checked > 0, "no independent visible samples at FOV={fov}");
    }

    #[test]
    fn dense_coast_controls_cover_tight_overview_polar_and_dateline_views() {
        for (site, fov) in [
            (observer(37.7749, -122.4194), 0.1),
            (observer(37.7749, -122.4194), 1.0),
            (observer(37.7749, -122.4194), 5.0),
            (observer(37.7749, -122.4194), 100.0),
            (observer(89.9, 0.0), 20.0),
            (observer(0.0, 179.9), 5.0),
        ] {
            check_dense_visible_samples(site, fov);
        }
    }

    // An unrelated coast stroke cannot conceal an absent run. A finite point
    // probe with three-pixel allowance is not a mathematical visibility bound.
    fn check_isolated_segment(
        screen: &mut Screen,
        camera: &Camera,
        view: View,
        site: GeodeticPosition,
        fov: f64,
        part: usize,
        index: usize,
    ) -> bool {
        let layout = Layout::wide();
        screen.0.fill([false; 400]);
        let a = point(ASSET, index).unwrap();
        let b = point(ASSET, index + 1).unwrap();
        let count = steps(a, b);
        for step in 0..count {
            let first = interpolate(a, b, step as f64 / count as f64);
            let last = interpolate(a, b, (step + 1) as f64 / count as f64);
            assert!(interval(screen, camera, view, layout, first, last).unwrap());
        }
        let mut last_visible = None;
        let mut distinct = 0;
        for k in 1..count * 8 {
            let at = interpolate(a, b, k as f64 / (count * 8) as f64);
            let (camera_point, facing) = sample(camera, at).unwrap();
            if facing <= 1.0 {
                continue;
            }
            let Some(p) = view.project_unclipped(camera_point) else {
                continue;
            };
            let pixel = layout.marker(p);
            if pixel.x < 4 || pixel.x >= 348 || pixel.y < 4 || pixel.y >= 236 {
                continue;
            }
            if last_visible != Some(pixel) {
                distinct += 1;
                last_visible = Some(pixel);
            }
            if distinct >= 3 {
                assert!(
                    (-3..=3).any(|dy| (-3..=3)
                        .any(|dx| { screen.0[(pixel.y + dy) as usize][(pixel.x + dx) as usize] })),
                    "isolated coast gap: FOV={fov} observer={site:?} part={part} segment={index} t={k}/{} pixel={pixel:?}",
                    count * 8
                );
            }
        }
        distinct >= 3
    }

    // Unlike the full-layer control above, draw one asset segment at a time.
    #[test]
    fn isolated_export_segments_retain_visible_runs() {
        let layout = Layout::wide();
        for (site, fov) in [
            (observer(37.7749, -122.4194), 0.1),
            (observer(37.7749, -122.4194), 1.0),
            (observer(37.7749, -122.4194), 5.0),
            (observer(37.7749, -122.4194), 100.0),
            (observer(89.9, 0.0), 20.0),
            (observer(0.0, 179.9), 5.0),
        ] {
            let view = View::rectangular(fov, layout.aspect_ratio()).unwrap();
            let camera = Camera::new(site).unwrap();
            let mut screen = Screen([[false; 400]; 240]);
            let mut visible_segments = 0;
            for part in 0..PARTS {
                let start = u32_at(ASSET, COORD_BYTES + part * 4).unwrap();
                let end = u32_at(ASSET, COORD_BYTES + (part + 1) * 4).unwrap();
                for index in start..end - 1 {
                    visible_segments += usize::from(check_isolated_segment(
                        &mut screen,
                        &camera,
                        view,
                        site,
                        fov,
                        part,
                        index,
                    ));
                }
            }
            assert!(
                visible_segments > 0,
                "no qualifying export segments at FOV={fov}"
            );
        }
    }

    #[test]
    fn isolated_segments_near_computed_limb_do_not_drop_visible_runs() {
        let layout = Layout::wide();
        let mut screen = Screen([[false; 400]; 240]);
        let mut probed = 0;
        let mut visible = 0;
        for part in 0..PARTS {
            let start = u32_at(ASSET, COORD_BYTES + part * 4).unwrap();
            let end = u32_at(ASSET, COORD_BYTES + (part + 1) * 4).unwrap();
            for index in start..end - 1 {
                let a = point(ASSET, index).unwrap();
                let b = point(ASSET, index + 1).unwrap();
                if index % 23 != 0 || steps(a, b) < 5 {
                    continue;
                }
                let middle = interpolate(a, b, 0.5);
                // Find an observer longitude for which this asset segment's
                // midpoint is precisely on the camera-facing limb. The
                // perturbations place its endpoints on both sides in turn.
                let site_at = |offset: f64| {
                    let longitude = (middle.0 + offset + 180.0).rem_euclid(360.0) - 180.0;
                    observer(middle.1, longitude)
                };
                let facing = |offset: f64| {
                    sample(&Camera::new(site_at(offset)).unwrap(), middle)
                        .unwrap()
                        .1
                };
                if facing(180.0) >= 0.0 {
                    // A high-latitude parallel need not cross the limb under
                    // longitude change alone; this site construction cannot
                    // probe that segment.
                    continue;
                }
                assert!(facing(0.0) > 0.0);
                let (mut lo, mut hi) = (0.0, 180.0);
                for _ in 0..32 {
                    let mid = (lo + hi) / 2.0;
                    if facing(mid) > 0.0 {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                let limb = (lo + hi) / 2.0;
                for offset in [limb - 0.2, limb, limb + 0.2] {
                    let site = site_at(offset);
                    let camera = Camera::new(site).unwrap();
                    for fov in [20.0, 100.0] {
                        let view = View::rectangular(fov, layout.aspect_ratio()).unwrap();
                        visible += usize::from(check_isolated_segment(
                            &mut screen,
                            &camera,
                            view,
                            site,
                            fov,
                            part,
                            index,
                        ));
                        probed += 1;
                    }
                }
            }
        }
        assert!(
            probed >= 12 && visible >= 3,
            "limb suite failed to exercise enough screen-relevant export segments: {probed} probes, {visible} visible"
        );
    }

    #[test]
    fn isolated_segments_near_viewport_edge_do_not_drop_visible_runs() {
        let layout = Layout::wide();
        let fov = 0.1;
        let view = View::rectangular(fov, layout.aspect_ratio()).unwrap();
        let mut screen = Screen([[false; 400]; 240]);
        let mut probed = 0;
        let mut visible = 0;
        for part in 0..PARTS {
            let start = u32_at(ASSET, COORD_BYTES + part * 4).unwrap();
            let end = u32_at(ASSET, COORD_BYTES + (part + 1) * 4).unwrap();
            for index in start..end - 1 {
                let a = point(ASSET, index).unwrap();
                let b = point(ASSET, index + 1).unwrap();
                if index % 23 != 0 || steps(a, b) < 5 || a.1.abs() > 60.0 {
                    continue;
                }
                let middle = interpolate(a, b, 0.5);
                let site_at = |offset: f64| {
                    observer(
                        middle.1,
                        (middle.0 + offset + 180.0).rem_euclid(360.0) - 180.0,
                    )
                };
                let east = |offset: f64| {
                    let camera = Camera::new(site_at(offset)).unwrap();
                    let position = sample(&camera, middle).unwrap().0;
                    view.project_unclipped(position).unwrap().east.abs()
                };
                let (mut lo, mut hi) = (0.0, 3.0);
                assert!(east(lo) < 1.0 && east(hi) > 1.0);
                for _ in 0..32 {
                    let mid = (lo + hi) / 2.0;
                    if east(mid) < 1.0 {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                let edge = (lo + hi) / 2.0;
                for offset in [edge - 0.08, edge, edge + 0.08] {
                    let site = site_at(offset);
                    let camera = Camera::new(site).unwrap();
                    visible += usize::from(check_isolated_segment(
                        &mut screen,
                        &camera,
                        view,
                        site,
                        fov,
                        part,
                        index,
                    ));
                    probed += 1;
                }
            }
        }
        assert!(
            probed >= 6 && visible >= 3,
            "viewport suite failed to exercise enough screen-relevant export segments: {probed} probes, {visible} visible"
        );
    }

    #[test]
    fn limb_crossing_and_hidden_endpoints_retain_visible_runs() {
        let layout = Layout::wide();
        let site = observer(0.0, 0.0);
        let view = View::rectangular(100.0, layout.aspect_ratio()).unwrap();
        let camera = Camera::new(site).unwrap();
        let (front, hidden) = ((80.0, 0.0), (90.0, 0.0));
        assert!(sample(&camera, front).unwrap().1 > 0.0);
        assert!(sample(&camera, hidden).unwrap().1 < 0.0);
        let root = crossing(&camera, front, hidden, true).unwrap();
        let projected = view
            .project_unclipped(sample(&camera, root).unwrap().0)
            .unwrap();
        let edge = layout.marker(projected);
        let mut screen = Screen([[false; 400]; 240]);
        assert!(interval(&mut screen, &camera, view, layout, front, hidden).unwrap());
        assert!(
            (-2..=2).any(|dy| (-2..=2)
                .any(|dx| { screen.0[(edge.y + dy) as usize][(edge.x + dx) as usize] })),
            "a front-to-back tangent crossing lost its visible end"
        );

        // Both ends hidden, but the middle of the geographic interval is
        // visible. This exercises the other branch of the tangent split.
        let mut middle = Screen([[false; 400]; 240]);
        assert!(
            interval(
                &mut middle,
                &camera,
                view,
                layout,
                (-90.0, 0.0),
                (90.0, 0.0)
            )
            .unwrap()
        );
        let center = layout.marker(
            view.project_unclipped(sample(&camera, (0.0, 0.0)).unwrap().0)
                .unwrap(),
        );
        assert!(middle.0[center.y as usize][center.x as usize]);
    }

    #[test]
    fn viewport_corner_and_separate_runs_do_not_connect() {
        let layout = Layout::wide();
        let view = View::rectangular(5.0, layout.aspect_ratio()).unwrap();
        let camera = Camera::new(observer(37.7749, -122.4194)).unwrap();
        let mut screen = Screen([[false; 400]; 240]);
        // Synthetic separately drawn front-facing runs. This checks that the
        // interval routine doesn't join a gap; the real asset's part breaks
        // remain covered only by its checked offsets and the full draw loop.
        let a = (-123.0, 37.7);
        let b = (-122.9, 37.7);
        let c = (-122.0, 37.7);
        let d = (-121.9, 37.7);
        assert!(interval(&mut screen, &camera, view, layout, a, b).unwrap());
        assert!(interval(&mut screen, &camera, view, layout, c, d).unwrap());
        let mut with_bridge = Screen(screen.0);
        assert!(interval(&mut with_bridge, &camera, view, layout, b, c).unwrap());
        assert!(
            screen
                .0
                .iter()
                .flatten()
                .zip(with_bridge.0.iter().flatten())
                .any(|(&separate, &joined)| !separate && joined),
            "separate runs need visible separation"
        );
        // A diagonal missing the viewport corner must not produce edge pixels.
        assert!(
            clip(
                ViewPoint {
                    east: -2.0,
                    north: 1.1
                },
                ViewPoint {
                    east: 1.1,
                    north: 2.0
                }
            )
            .is_none()
        );
        assert!(screen.0.iter().enumerate().all(|(y, row)| {
            row.iter().enumerate().all(|(x, &ink)| {
                !ink || layout.contains(Point::new(x as i32, y as i32), Viewport::Rectangle)
            })
        }));
    }

    #[test]
    fn pinned_export_has_checked_parts_and_coordinates() {
        assert!(valid(ASSET));
        assert!(!valid(&ASSET[..ASSET.len() - 1]));
        let mut corrupt = ASSET.to_vec();
        corrupt[COORD_BYTES + 4..COORD_BYTES + 8].copy_from_slice(&0_u32.to_le_bytes());
        assert!(!valid(&corrupt));
    }

    #[test]
    fn visible_run_with_both_endpoints_outside_is_clipped() {
        let (a, b) = clip(
            ViewPoint {
                east: -2.0,
                north: 0.0,
            },
            ViewPoint {
                east: 2.0,
                north: 0.0,
            },
        )
        .unwrap();
        assert_eq!((a.east, b.east), (-1.0, 1.0));
        assert!(
            clip(
                ViewPoint {
                    east: -2.0,
                    north: 2.0
                },
                ViewPoint {
                    east: 2.0,
                    north: 2.0
                }
            )
            .is_none()
        );
    }
}
