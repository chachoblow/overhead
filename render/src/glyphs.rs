//! The bounded, static circular-elevation strip (decision 0041).
//! No presentation history or physical-state mutation belongs here.

use core::f64::consts::FRAC_PI_2;
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};

// Fixed radius-11 circle, independent of primitive stroke/raster conventions.
const OCTANT: [Point; 9] = [
    Point::new(11, 0),
    Point::new(11, 1),
    Point::new(11, 2),
    Point::new(11, 3),
    Point::new(10, 4),
    Point::new(10, 5),
    Point::new(9, 6),
    Point::new(8, 7),
    Point::new(8, 8),
];

pub(super) fn draw_strip<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    elevation: Option<f64>,
) -> Result<(), D::Error> {
    display.draw_iter((12..=227).map(|y| Pixel(Point::new(352, y), BinaryColor::On)))?;
    if let Some(elevation) = elevation.filter(|value| value.is_finite()) {
        Text::new(
            "Elev",
            Point::new(364, 64),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        )
        .draw(display)?;
        draw_elevation(display, elevation)?;
    }
    Ok(())
}

fn draw_elevation<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    elevation: f64,
) -> Result<(), D::Error> {
    if !elevation.is_finite() {
        return Ok(());
    }
    let endpoint = snapped_endpoint(elevation.clamp(0.0, FRAC_PI_2));
    // This local row bitset occupies 92 bytes, not a display framebuffer.
    // It forms the pixel union: joins/shared radial pixels are emitted once.
    let mut rows = [0_u32; 23];
    for p in OCTANT {
        for p in [p, Point::new(p.y, p.x)] {
            for x in [p.x, -p.x] {
                for y in [p.y, -p.y] {
                    // Remove only the open angular interval (0, endpoint).
                    // Integer cross products preserve both arc endpoints.
                    if !(x > 0 && y > 0 && y * endpoint.x < x * endpoint.y) {
                        mark(&mut rows, Point::new(x, y));
                    }
                }
            }
        }
    }
    if endpoint.y != 0 {
        radial(&mut rows, Point::new(11, 0));
        radial(&mut rows, endpoint);
    }
    display.draw_iter(rows.into_iter().enumerate().flat_map(|(row, bits)| {
        (0..23).filter_map(move |column| {
            (bits & (1 << column) != 0).then_some(Pixel(
                Point::new(365 + column, 69 + row as i32),
                BinaryColor::On,
            ))
        })
    }))
}

// The first-quadrant circle is traversed in increasing angle. Midpoint
// boundaries select the nearer endpoint; <= gives exact ties to the lower one.
fn snapped_endpoint(elevation: f64) -> Point {
    let mut previous = OCTANT[0];
    let mut previous_angle = 0.0;
    for next in OCTANT
        .iter()
        .copied()
        .skip(1)
        .chain(OCTANT[..8].iter().rev().map(|p| Point::new(p.y, p.x)))
    {
        let angle = libm::atan2(f64::from(next.y), f64::from(next.x));
        if elevation <= (previous_angle + angle) / 2.0 {
            return previous;
        }
        previous = next;
        previous_angle = angle;
    }
    previous
}

fn mark(rows: &mut [u32; 23], p: Point) {
    // Glyph-local coordinates are right/up around center (376,80).
    rows[(11 - p.y) as usize] |= 1 << (11 + p.x);
}

fn radial(rows: &mut [u32; 23], endpoint: Point) {
    let mut p = Point::zero();
    let dx = endpoint.x;
    let dy = -endpoint.y;
    let mut error = dx + dy;
    loop {
        mark(rows, p);
        if p == endpoint {
            return;
        }
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            p.x += 1;
        }
        if twice <= dx {
            error += dx;
            p.y += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapping_includes_endpoints_and_assigns_midpoint_ties_downward() {
        // Explicit quadrant, not generated through the production traversal.
        let quadrant = [
            (11, 0),
            (11, 1),
            (11, 2),
            (11, 3),
            (10, 4),
            (10, 5),
            (9, 6),
            (8, 7),
            (8, 8),
            (7, 8),
            (6, 9),
            (5, 10),
            (4, 10),
            (3, 11),
            (2, 11),
            (1, 11),
            (0, 11),
        ];
        for &(x, y) in &quadrant {
            assert_eq!(
                snapped_endpoint(libm::atan2(y as f64, x as f64)),
                Point::new(x, y)
            );
        }
        for pair in quadrant.windows(2) {
            let [(x0, y0), (x1, y1)] = [pair[0], pair[1]];
            let boundary =
                (libm::atan2(y0 as f64, x0 as f64) + libm::atan2(y1 as f64, x1 as f64)) / 2.0;
            assert_eq!(snapped_endpoint(boundary), Point::new(x0, y0));
            assert_eq!(
                snapped_endpoint(f64::from_bits(boundary.to_bits() - 1)),
                Point::new(x0, y0)
            );
            assert_eq!(
                snapped_endpoint(f64::from_bits(boundary.to_bits() + 1)),
                Point::new(x1, y1)
            );
        }
    }

    #[test]
    fn radials_are_inclusive_single_pixels_per_major_axis_step() {
        for x in 0..=11_i32 {
            for y in 0..=11_i32 {
                let mut rows = [0; 23];
                radial(&mut rows, Point::new(x, y));
                assert_eq!(
                    rows.iter().map(|r| r.count_ones()).sum::<u32>(),
                    x.max(y) as u32 + 1
                );
                assert_ne!(rows[11] & (1 << 11), 0);
                assert_ne!(rows[(11 - y) as usize] & (1 << (11 + x)), 0);
                // Independent major-axis nearest-integer reference, including
                // positive half ties; no Bresenham error accumulator here.
                for step in 0..=x.max(y) {
                    let (u, v) = if x == 0 && y == 0 {
                        (0, 0)
                    } else if x >= y {
                        (step, (2 * step * y + x) / (2 * x))
                    } else {
                        ((2 * step * x + y) / (2 * y), step)
                    };
                    assert_ne!(
                        rows[(11 - v) as usize] & (1 << (11 + u)),
                        0,
                        "{x},{y}: {u},{v}"
                    );
                }
            }
        }
    }
}
