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
                if !interval(display, &camera, view, layout, first, last)? {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

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
