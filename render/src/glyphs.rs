//! Static selected elevation, altitude, speed and period (decisions 0041–0044).
//! No presentation history or physical-state mutation belongs here.

use core::f64::consts::{FRAC_PI_2, TAU};
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
use overhead_app::RadarApp;

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

/// Bounded presentation parameters, not true physical readouts. Each field's
/// endpoint uses the same mapping as the original static renderer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Params(pub [Option<f64>; 4]);

impl Params {
    pub(crate) fn selected(app: &RadarApp) -> Self {
        let selected = app.selected().map(|(_, s)| s.physical);
        Self::from_readouts(
            selected.map(|s| s.look.elevation_rad),
            selected.map(|s| s.geodetic.altitude_km),
            selected.map(|s| s.teme_speed_km_s),
            app.selected_period_minutes(),
        )
    }

    fn from_readouts(
        elevation: Option<f64>,
        altitude_km: Option<f64>,
        speed_km_s: Option<f64>,
        period_minutes: Option<f64>,
    ) -> Self {
        Self([
            elevation
                .filter(|v| v.is_finite())
                .map(|v| v.clamp(0.0, FRAC_PI_2)),
            altitude_km.and_then(altitude_continuous_radius),
            speed_km_s
                .filter(|v| v.is_finite())
                .map(|v| v.clamp(0.0, 12.0)),
            period_minutes.and_then(period_turns),
        ])
    }
}

pub(super) fn draw_strip<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    elevation: Option<f64>,
    altitude_km: Option<f64>,
    speed_km_s: Option<f64>,
    period_minutes: Option<f64>,
) -> Result<(), D::Error> {
    draw_strip_params(
        display,
        Params::from_readouts(elevation, altitude_km, speed_km_s, period_minutes),
    )
}

pub(crate) fn draw_strip_params<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    Params([elevation, altitude_radius, speed, turns]): Params,
) -> Result<(), D::Error> {
    display.draw_iter((12..=227).map(|y| Pixel(Point::new(352, y), BinaryColor::On)))?;
    if let Some(elevation) = elevation {
        Text::new(
            "Elev",
            Point::new(364, 64),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        )
        .draw(display)?;
        draw_elevation(display, elevation)?;
    }
    if let Some(radius) = altitude_radius {
        Text::new(
            "Alt",
            Point::new(367, 111),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        )
        .draw(display)?;
        let mut rows = [0_u32; 23];
        circle(&mut rows, 11);
        circle(&mut rows, i32::from(round_radius(radius)));
        draw_rows(display, rows, Point::new(365, 116))?;
    }
    if let Some(speed) = speed {
        Text::new(
            "Spd",
            Point::new(367, 158),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        )
        .draw(display)?;
        let mut rows = [0_u32; 23];
        for index in 0..speed_pixel_count(speed) {
            mark(&mut rows, speed_pixel(index));
        }
        draw_rows(display, rows, Point::new(365, 163))?;
    }
    if let Some(turns) = turns {
        Text::new(
            "Per",
            Point::new(367, 205),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        )
        .draw(display)?;
        draw_rows(display, period_rows(turns), Point::new(365, 209))?;
    }
    Ok(())
}

fn period_turns(minutes: f64) -> Option<f64> {
    if !minutes.is_finite() || minutes <= 0.0 {
        return None;
    }
    Some(2.5 - 1.5 * libm::log(minutes.clamp(90.0, 1440.0) / 90.0) / libm::log(16.0))
}

// The reviewed spiral is a union of 4097 rounded parametric samples, not a
// stroked primitive. Adjacent samples are much less than one pixel apart, so
// their rounded centers are 8-connected without line interpolation. Fix the
// sample count for reproducible ink; changing it can remove boundary pixels.
// Bounded, allocation-free work, but not a measured device-frame budget.
fn period_rows(turns: f64) -> [u32; 23] {
    let mut rows = [0; 23];
    for step in 0..=4096 {
        let u = f64::from(step) / 4096.0;
        let radius = 1.5 + 7.5 * u;
        let angle = -FRAC_PI_2 + TAU * turns * (u - 1.0);
        // Round absolute display coordinates, half ties toward +infinity,
        // exactly as in the approved preview; mark() uses local right/up.
        let x = libm::floor(376.0 + radius * libm::cos(angle) + 0.5) as i32 - 376;
        let y = 220 - libm::floor(220.0 + radius * libm::sin(angle) + 0.5) as i32;
        mark(&mut rows, Point::new(x, y));
    }
    rows
}

// First quadrant, ordered clockwise from the top, omitting the right endpoint
// (it starts the next quadrant). Together the four quadrants have 64 pixels.
fn speed_quadrant(index: usize) -> Point {
    if index <= 8 {
        Point::new(OCTANT[index].y, OCTANT[index].x)
    } else {
        OCTANT[16 - index]
    }
}

fn speed_pixel(index: usize) -> Point {
    let p = speed_quadrant(index % 16);
    match index / 16 {
        0 => p,
        1 => Point::new(p.y, -p.x),
        2 => Point::new(-p.x, -p.y),
        _ => Point::new(-p.y, p.x),
    }
}

fn speed_angle(index: usize) -> f64 {
    let p = speed_quadrant(index % 16);
    (index / 16) as f64 * FRAC_PI_2 + libm::atan2(f64::from(p.x), f64::from(p.y))
}

fn speed_boundary(next: usize) -> f64 {
    // Express angular midpoint thresholds in km/s; exact floating-point
    // threshold ties belong to the shorter arc. No wrapped distance at 2π.
    12.0 * ((speed_angle(next - 1) + speed_angle(next)) / 2.0) / TAU
}

// Caller has checked finite input. Positive values always include the top
// pixel; the last quantization bucket is already the full circle below 12.
fn speed_pixel_count(speed: f64) -> usize {
    if speed <= 0.0 {
        return 0;
    }
    if speed >= 12.0 {
        return 64;
    }
    // Lower-bound search over 63 increasing thresholds: at most six
    // comparisons, without a runtime allocation or a rounded lookup table.
    let mut low = 1;
    let mut high = 64;
    while low < high {
        let next = (low + high) / 2;
        if speed <= speed_boundary(next) {
            high = next;
        } else {
            low = next + 1;
        }
    }
    low
}

fn altitude_continuous_radius(altitude_km: f64) -> Option<f64> {
    if !altitude_km.is_finite() {
        return None;
    }
    let h = altitude_km.clamp(0.0, 40_000.0);
    let t = libm::log1p(h / 400.0) / libm::log(101.0);
    Some(9.0 - 7.0 * t)
}

#[cfg(test)]
fn altitude_radius(altitude_km: f64) -> Option<u8> {
    altitude_continuous_radius(altitude_km).map(round_radius)
}

// Input is bounded to [2,9]; truncation after adding 0.5 rounds half ties up.
fn round_radius(radius: f64) -> u8 {
    (radius + 0.5) as u8
}

fn circle(rows: &mut [u32; 23], radius: i32) {
    let mut major = radius;
    for minor in 0..=radius {
        let squared = radius * radius - minor * minor;
        // Nearest sqrt using exact integer half-step boundaries, not a
        // library stroke convention. Integer radicands cannot land on a tie.
        while major > 0 && 4 * squared < (2 * major - 1) * (2 * major - 1) {
            major -= 1;
        }
        if minor > major {
            break;
        }
        for (x, y) in [(major, minor), (minor, major)] {
            for x in [x, -x] {
                for y in [y, -y] {
                    mark(rows, Point::new(x, y));
                }
            }
        }
    }
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
    draw_rows(display, rows, Point::new(365, 69))
}

fn draw_rows<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    rows: [u32; 23],
    top_left: Point,
) -> Result<(), D::Error> {
    display.draw_iter(rows.into_iter().enumerate().flat_map(|(row, bits)| {
        (0..23).filter_map(move |column| {
            (bits & (1 << column) != 0).then_some(Pixel(
                top_left + Point::new(column, row as i32),
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
    // Glyph-local coordinates are right/up around the integer center.
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
    extern crate std;
    use super::*;

    #[test]
    fn period_scale_is_fixed_saturating_and_monotonic() {
        for (period, turns) in [
            (f64::from_bits(1), 2.5),
            (90.0, 2.5),
            (180.0, 2.125),
            (360.0, 1.75),
            (720.0, 1.375),
            (1440.0, 1.0),
            (f64::MAX, 1.0),
        ] {
            assert!((period_turns(period).unwrap() - turns).abs() < 1e-14);
        }
        for invalid in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -f64::MAX,
            -0.0,
            0.0,
        ] {
            assert_eq!(period_turns(invalid), None);
        }
        let mut previous = 2.5;
        for minute in 1..=2880 {
            let turns = period_turns(f64::from(minute)).unwrap();
            assert!((1.0..=2.5).contains(&turns) && turns <= previous);
            previous = turns;
        }
        assert_eq!(
            period_turns(f64::from_bits(90_f64.to_bits() - 1)),
            Some(2.5)
        );
        assert_eq!(
            period_turns(f64::from_bits(1440_f64.to_bits() + 1)),
            Some(1.0)
        );
    }

    #[test]
    fn period_rasters_are_bounded_connected_and_have_no_center_fill() {
        for step in 0..=256 {
            let turns = 1.0 + 1.5 * f64::from(step) / 256.0;
            let rows = period_rows(turns);
            let mut seen = [0_u32; 23];
            // Every spiral ends at absolute (376,211), local top radius 9.
            assert_ne!(rows[2] & (1 << 11), 0);
            assert_eq!(rows[11] & (1 << 11), 0);
            seen[2] = 1 << 11;
            loop {
                let old = seen;
                for y in 1..22 {
                    let neighbors = old[y - 1] | old[y] | old[y + 1];
                    seen[y] |= rows[y] & (neighbors | (neighbors << 1) | (neighbors >> 1));
                }
                if seen == old {
                    break;
                }
            }
            assert_eq!(seen, rows, "turns={turns}");
            for (y, row) in rows.iter().enumerate() {
                if !(2..=20).contains(&y) {
                    assert_eq!(*row, 0);
                }
                assert_eq!(row & !0x1ffffc, 0); // x local -9..9 only.
            }
        }
    }

    #[test]
    fn speed_endpoints_saturation_and_all_midpoint_ties() {
        for speed in [-f64::MAX, -0.0, 0.0] {
            assert_eq!(speed_pixel_count(speed), 0);
        }
        assert_eq!(speed_pixel_count(f64::from_bits(1)), 1);
        for (speed, count) in [
            (3.0, 17),
            (6.0, 33),
            (9.0, 49),
            (11.99, 64),
            (12.0, 64),
            (f64::MAX, 64),
        ] {
            assert_eq!(speed_pixel_count(speed), count);
        }
        let mut previous = 0.0;
        for next in 1..64 {
            let boundary = speed_boundary(next);
            assert!(boundary > previous && boundary < 12.0);
            assert_eq!(
                speed_pixel_count(f64::from_bits(boundary.to_bits() - 1)),
                next
            );
            assert_eq!(speed_pixel_count(boundary), next);
            assert_eq!(
                speed_pixel_count(f64::from_bits(boundary.to_bits() + 1)),
                next + 1
            );
            previous = boundary;
        }
    }

    #[test]
    fn speed_traversal_is_the_same_circle_with_connected_unique_prefixes() {
        let mut rows = [0; 23];
        let mut previous = speed_pixel(63);
        assert_eq!(speed_pixel(0), Point::new(0, 11));
        assert_eq!(speed_pixel(16), Point::new(11, 0));
        assert_eq!(speed_pixel(32), Point::new(0, -11));
        assert_eq!(speed_pixel(48), Point::new(-11, 0));
        for index in 0..64 {
            let p = speed_pixel(index);
            assert!((p.x - previous.x).abs() <= 1 && (p.y - previous.y).abs() <= 1);
            mark(&mut rows, p);
            assert_eq!(
                rows.iter().map(|row| row.count_ones()).sum::<u32>(),
                index as u32 + 1
            );
            previous = p;
        }
        let mut expected = [0; 23];
        circle(&mut expected, 11);
        assert_eq!(rows, expected);
    }

    #[test]
    fn altitude_mapping_saturates_and_is_monotonic() {
        for (altitude, radius) in [
            (-f64::MAX, 9),
            (-0.0, 9),
            (0.0, 9),
            (400.0, 8),
            (800.0, 7),
            (2_000.0, 6),
            (5_000.0, 5),
            (10_000.0, 4),
            (20_200.0, 3),
            (35_786.0, 2),
            (40_000.0, 2),
            (f64::MAX, 2),
        ] {
            assert_eq!(altitude_radius(altitude), Some(radius));
        }
        for altitude in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(altitude_radius(altitude), None);
        }
        let mut previous = 9;
        for altitude in 0..=40_000 {
            let radius = altitude_radius(f64::from(altitude)).unwrap();
            assert!((2..=9).contains(&radius) && radius <= previous);
            previous = radius;
        }
    }

    #[test]
    fn altitude_boundaries_and_radius_half_ties() {
        for lower in 2..9 {
            let halfway = f64::from(lower) + 0.5;
            assert_eq!(round_radius(halfway), lower + 1);
            assert_eq!(round_radius(f64::from_bits(halfway.to_bits() - 1)), lower);
            assert_eq!(
                round_radius(f64::from_bits(halfway.to_bits() + 1)),
                lower + 1
            );
            // Independent inverse of the log mapping. The inverse itself is
            // rounded, so bracket the physical boundary rather than claiming
            // its binary float is an exact transcendental tie.
            let altitude = 400.0 * (101_f64.powf((9.0 - halfway) / 7.0) - 1.0);
            assert_eq!(altitude_radius(altitude - 1e-7), Some(lower + 1));
            assert_eq!(altitude_radius(altitude + 1e-7), Some(lower));
        }
    }

    #[test]
    fn altitude_circles_match_independent_raster_and_are_connected() {
        for radius in (2..=9).chain(core::iter::once(11)) {
            let mut rows = [0; 23];
            circle(&mut rows, radius);
            for y in -11_i32..=11 {
                for x in -11_i32..=11 {
                    let minor = x.abs().min(y.abs());
                    let major = x.abs().max(y.abs());
                    let expected = minor <= radius
                        && major
                            == f64::from(radius * radius - minor * minor).sqrt().round() as i32;
                    assert_eq!(
                        rows[(11 - y) as usize] & (1 << (11 + x)) != 0,
                        expected,
                        "r={radius} at {x},{y}"
                    );
                }
            }
            assert_eq!(rows[11] & (1 << 11), 0); // Hollow center.
            let mut visited = [0_u32; 23];
            let mut pending = std::vec![Point::new(radius, 0)];
            while let Some(p) = pending.pop() {
                if p.x.abs() > 11 || p.y.abs() > 11 {
                    continue;
                }
                let row = (11 - p.y) as usize;
                let bit = 1 << (11 + p.x);
                if rows[row] & bit == 0 || visited[row] & bit != 0 {
                    continue;
                }
                mark(&mut visited, p);
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        pending.push(p + Point::new(dx, dy));
                    }
                }
            }
            assert_eq!(visited, rows);
            let mut outer = [0; 23];
            for p in OCTANT {
                for p in [p, Point::new(p.y, p.x)] {
                    for x in [p.x, -p.x] {
                        for y in [p.y, -p.y] {
                            mark(&mut outer, Point::new(x, y));
                        }
                    }
                }
            }
            if radius == 11 {
                assert_eq!(rows, outer); // Preserve elevation's circle exactly.
            } else {
                assert!(rows.iter().zip(outer).all(|(a, b)| a & b == 0));
            }
        }
    }

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
