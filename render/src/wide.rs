//! The full-width wide radar of decision 0089 (the radar lab's Favourite C,
//! as tweaked on the panel): left pills, 7 px octagon markers and 5 px hollow
//! below-horizon diamonds with 1 px cleared halos, the selected octagon with a
//! pulsing ring, haloed off-screen arrows and a km scale ruler. The coastline's
//! sphere shading is baked into the coast raster ([`shade_levels`]). Heap-free.
use core::fmt::Write;

use embedded_graphics::{
    mono_font::{MonoTextStyle, iso_8859_1::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::Rectangle,
    text::{Baseline, Text},
};
use overhead_app::{
    RadarApp,
    projection::{CAMERA_HEIGHT_KM, ViewPoint},
    visibility::Visibility,
};
use overhead_core::sgp4::chrono::{NaiveDateTime, Timelike};

use crate::{FrameStage, Layout, LineBuffer};

/// Pulsing ring cycle (0089).
pub(crate) const PULSE_MS: u64 = 1400;

/// Octagon offsets: |dx|, |dy| <= 3 and |dx| + |dy| <= 4 (37 pixels).
pub(crate) fn octagon() -> impl Iterator<Item = (i32, i32)> + Clone {
    (-3..=3)
        .flat_map(|y| (-3..=3).map(move |x| (x, y)))
        .filter(|(x, y): &(i32, i32)| x.abs() + y.abs() <= 4)
}

/// 5 px hollow diamond for below-horizon group members.
pub(crate) const DIAMOND5: [(i32, i32); 8] = [
    (0, -2),
    (1, -1),
    (2, 0),
    (1, 1),
    (0, 2),
    (-1, 1),
    (-2, 0),
    (-1, -1),
];

/// Half-up rounding, the same on every target (`f64::round` is std-only).
fn rnd(v: f64) -> i32 {
    libm::floor(v + 0.5) as i32
}

/// Pixel writer for one frame; the caller has clipped the target to 400×240.
struct Ink<'a, D> {
    d: &'a mut D,
}

impl<D: DrawTarget<Color = BinaryColor>> Ink<'_, D> {
    fn px(&mut self, p: Point, on: bool) -> Result<(), D::Error> {
        self.d.draw_iter([Pixel(p, BinaryColor::from(on))])
    }
    fn pixels(
        &mut self,
        at: Point,
        shape: impl Iterator<Item = (i32, i32)>,
        on: bool,
    ) -> Result<(), D::Error> {
        let color = BinaryColor::from(on);
        self.d
            .draw_iter(shape.map(|(x, y)| Pixel(at + Point::new(x, y), color)))
    }
    /// The shape after its 1 px cleared halo (a 3×3 dilation set Off).
    fn haloed(
        &mut self,
        at: Point,
        shape: impl Iterator<Item = (i32, i32)> + Clone,
    ) -> Result<(), D::Error> {
        let halo = shape
            .clone()
            .flat_map(|(x, y)| (-1..=1).flat_map(move |j| (-1..=1).map(move |i| (x + i, y + j))));
        self.pixels(at, halo, false)?;
        self.pixels(at, shape, true)
    }
    fn hline(&mut self, x0: i32, x1: i32, y: i32, on: bool) -> Result<(), D::Error> {
        self.pixels(Point::zero(), (x0..=x1).map(|x| (x, y)), on)
    }
    fn vline(&mut self, x: i32, y0: i32, y1: i32, on: bool) -> Result<(), D::Error> {
        self.pixels(Point::zero(), (y0..=y1).map(|y| (x, y)), on)
    }
    /// Ring of radius `r` sampled by angle; `dash` keeps every dash-th point.
    fn circle(&mut self, c: Point, r: f64, dash: usize) -> Result<(), D::Error> {
        let n = libm::ceil(r * 6.3).max(12.0) as usize;
        let points = (0..n).filter(|k| dash == 0 || k % dash == 0).map(|k| {
            let (s, co) = libm::sincos(k as f64 / n as f64 * core::f64::consts::TAU);
            (rnd(r * co), rnd(r * s))
        });
        self.pixels(c, points, true)
    }
    fn text(&mut self, x: i32, y: i32, s: &str, on: bool) -> Result<(), D::Error> {
        Text::with_baseline(
            s,
            Point::new(x, y),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::from(on)),
            Baseline::Top,
        )
        .draw(self.d)?;
        Ok(())
    }
    /// Fill with the pills' 2 px corner cut; `on(x)` picks the colour by column.
    fn rounded_fill(&mut self, r: Rectangle, on: impl Fn(i32) -> bool) -> Result<(), D::Error> {
        let (x, y) = (r.top_left.x, r.top_left.y);
        let (w, h) = (r.size.width as i32, r.size.height as i32);
        for j in 0..h {
            let inset = match j.min(h - 1 - j) {
                0 => 2,
                1 => 1,
                _ => 0,
            };
            let row = (x + inset..x + w - inset)
                .map(|k| Pixel(Point::new(k, y + j), BinaryColor::from(on(k))));
            self.d.draw_iter(row)?;
        }
        Ok(())
    }
}

/// Draws every On pixel shifted and Off: one side of a glyph's cleared halo.
struct Shifted<'a, D> {
    d: &'a mut D,
    by: Point,
}

impl<D: DrawTarget<Color = BinaryColor>> Dimensions for Shifted<'_, D> {
    fn bounding_box(&self) -> Rectangle {
        self.d.bounding_box()
    }
}

impl<D: DrawTarget<Color = BinaryColor>> DrawTarget for Shifted<'_, D> {
    type Color = BinaryColor;
    type Error = D::Error;
    fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(
        &mut self,
        pixels: I,
    ) -> Result<(), D::Error> {
        let by = self.by;
        self.d.draw_iter(
            pixels
                .into_iter()
                .filter(|Pixel(_, c)| *c == BinaryColor::On)
                .map(|Pixel(p, _)| Pixel(p + by, BinaryColor::Off)),
        )
    }
}

/// Everything above the coastline and Earth outline, in 0089's order.
pub(crate) fn draw_foreground<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    app: &RadarApp,
    layout: Layout,
    trace: &mut impl FnMut(FrameStage),
) -> Result<(), D::Error> {
    let mut ink = Ink { d: display };
    let point = |v: ViewPoint| layout.marker(v);
    for (_, _, v) in app.below_horizon_markers() {
        ink.haloed(point(v), DIAMOND5.into_iter())?;
    }
    let selected = app.selected_id();
    let mut selected_at = None;
    for (entry, _, v) in app.markers() {
        if Some(entry.elements().norad_id) == selected {
            selected_at = Some(point(v));
        } else {
            ink.haloed(point(v), octagon())?;
        }
    }
    // The selected octagon goes on top of every other marker.
    if let Some(at) = selected_at {
        ink.haloed(at, octagon())?;
        let phase = pulse_phase(app.time());
        ink.circle(at, 6.0 + phase * 7.0, if phase > 0.6 { 2 } else { 0 })?;
    }
    trace(FrameStage::Markers);
    for (_, indicator) in app.indicators() {
        for (dx, dy) in [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ] {
            let mut halo = Shifted {
                d: ink.d,
                by: Point::new(dx, dy),
            };
            crate::draw_indicator(&mut halo, indicator, layout)?;
        }
        crate::draw_indicator(ink.d, indicator, layout)?;
    }
    trace(FrameStage::Arrows);
    if app.time().is_some() {
        scale_ruler(&mut ink, app.view().fov_deg(), f64::from(layout.radius()))?;
    }
    pills(&mut ink, app)?;
    trace(FrameStage::Overlays);
    Ok(())
}

/// Phase in [0, 1) of the 1.4 s pulse, from the frame's own UTC.
pub(crate) fn pulse_phase(time: Option<NaiveDateTime>) -> f64 {
    let Some(t) = time else { return 0.0 };
    let ms =
        u64::from(t.num_seconds_from_midnight()) * 1000 + u64::from(t.nanosecond() / 1_000_000);
    (ms % PULSE_MS) as f64 / PULSE_MS as f64
}

// ---------------------------------------------------------------- ruler

/// Ruler step: the largest 1/2/2.5/5 × 10^n km that spans at most 32 px.
pub(crate) fn ruler_step_km(km_per_px: f64) -> f64 {
    let mut step = 0.01;
    for e in -2..=6 {
        for m in [1.0, 2.0, 2.5, 5.0] {
            let v = m * libm::pow(10.0, f64::from(e));
            if v / km_per_px <= 32.0 {
                step = v;
            }
        }
    }
    step
}

/// Ground scale at the aim point: km per pixel at the observer's distance.
pub(crate) fn km_per_px(fov_deg: f64, radius_px: f64) -> f64 {
    CAMERA_HEIGHT_KM * libm::tan(fov_deg.to_radians() / 2.0) / radius_px
}

/// Label: up to two decimals without trailing zeros; thousands as "k".
pub(crate) fn ruler_label(km: f64) -> LineBuffer {
    let mut out = LineBuffer::new(12);
    let v = libm::round(km * 100.0) / 100.0;
    let (v, suffix) = if v >= 1000.0 {
        (libm::round(v / 100.0) / 10.0, "k")
    } else {
        (v, "")
    };
    let mut num = LineBuffer::new(12);
    let _ = write!(num, "{v:.2}");
    let s = num.as_str().trim_end_matches('0').trim_end_matches('.');
    let _ = write!(out, "{s}{suffix}");
    out
}

/// One line with end caps and four ticked divisions, labels above, "km" at
/// the end, on a rounded light backing at the bottom right.
fn scale_ruler<D: DrawTarget<Color = BinaryColor>>(
    ink: &mut Ink<'_, D>,
    fov_deg: f64,
    radius_px: f64,
) -> Result<(), D::Error> {
    let kpp = km_per_px(fov_deg, radius_px);
    let step = ruler_step_km(kpp);
    let span = step / kpp;
    let labels: [LineBuffer; 5] = core::array::from_fn(|k| ruler_label(step * k as f64));
    let widest = labels.iter().map(|l| l.as_str().len()).max().unwrap_or(1) as f64 * 6.0;
    let every = if span >= widest + 3.0 { 1 } else { 2 };
    let bar = rnd(span * 4.0);
    let last_half = (labels[4].as_str().len() * 3) as i32;
    let x0 = 400 - 4 - (3 + bar + last_half + 2 + 12) - 3;
    let base = 240 - 11;
    ink.rounded_fill(
        Rectangle::new(
            Point::new(x0 - 8, base - 16),
            Size::new((bar + last_half + 2 + 12 + 13) as u32, 23),
        ),
        |_| false,
    )?;
    ink.hline(x0, x0 + bar, base + 3, true)?;
    for (k, label) in labels.iter().enumerate() {
        let x = x0 + rnd(span * k as f64);
        let top = if k == 0 || k == 4 { base - 1 } else { base + 1 };
        ink.vline(x, top, base + 3, true)?;
        if k % every == 0 || k == 4 {
            let s = label.as_str();
            ink.text(x - s.len() as i32 * 3, base - 13, s, true)?;
        }
    }
    ink.text(x0 + bar + last_half + 2, base - 13, "km", true)
}

// ---------------------------------------------------------------- pills

/// One pill: an optional dark label side and a value.
pub(crate) struct PillText {
    pub(crate) label: Option<&'static str>,
    pub(crate) value: LineBuffer,
}

/// Pill rows and the indices after which a group gap follows (0089).
pub(crate) fn pill_rows(app: &RadarApp) -> ([Option<PillText>; 9], &'static [usize]) {
    let mut rows: [Option<PillText>; 9] = Default::default();
    let mut n = 0;
    let mut push = |label, value: LineBuffer| {
        rows[n] = Some(PillText { label, value });
        n += 1;
    };
    let value = |args: core::fmt::Arguments<'_>| {
        let mut b = LineBuffer::new(12);
        let _ = b.write_fmt(args);
        b
    };
    let degrees = |v: f64| {
        let mut b = value(format_args!("{:>3}", libm::round(v) as i64));
        b.push_degree();
        b
    };
    let gaps: &'static [usize] = if let Some((entry, sample)) = app.selected() {
        let truth = sample.physical;
        push(None, value(format_args!("{}", entry.elements().norad_id)));
        let mut name = LineBuffer::new(9);
        let shown = entry
            .elements()
            .object_name
            .as_deref()
            .unwrap_or("")
            .chars()
            .map(|c| c.to_ascii_uppercase())
            .filter(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == ' ' || *c == '-');
        for c in shown.take(9) {
            let _ = name.write_char(c);
        }
        let trimmed = name.as_str().trim();
        push(
            None,
            value(format_args!(
                "{}",
                if trimmed.is_empty() {
                    "UNNAMED"
                } else {
                    trimmed
                }
            )),
        );
        push(Some("EL"), degrees(truth.look.elevation_rad.to_degrees()));
        push(
            Some("AZ"),
            match truth.look.azimuth_rad {
                Some(az) => degrees(libm::fmod(az.to_degrees() + 360.0, 360.0)),
                None => value(format_args!("--")),
            },
        );
        push(
            Some("ALT"),
            value(format_args!(
                "{}km",
                libm::round(truth.geodetic.altitude_km) as i64
            )),
        );
        push(
            Some("RNG"),
            value(format_args!(
                "{}km",
                libm::round(truth.look.range_km) as i64
            )),
        );
        push(
            Some("SPD"),
            if truth.teme_speed_km_s.is_finite() {
                value(format_args!("{:.1}km/s", truth.teme_speed_km_s))
            } else {
                value(format_args!("--"))
            },
        );
        if let Some(per) = app.selected_period_minutes() {
            push(
                Some("PER"),
                if per < 1000.0 {
                    value(format_args!("{}m", libm::round(per) as i64))
                } else {
                    value(format_args!("{:.0}h", per / 60.0))
                },
            );
        }
        if let Some(vis) = app.selected_visibility() {
            push(
                None,
                value(format_args!(
                    "{}",
                    match vis {
                        Visibility::Visible => "VISIBLE",
                        Visibility::Eclipsed => "SHADOW",
                        Visibility::Daylight => "DAYLIGHT",
                    }
                )),
            );
        }
        &[1, 3, 5, 7]
    } else if app.time().is_some() && app.counts().catalogue > 0 {
        let counts = app.counts();
        push(None, value(format_args!("NO SEL")));
        push(Some("SKY"), value(format_args!("{}", counts.above_horizon)));
        push(Some("VIEW"), value(format_args!("{}", counts.visible)));
        &[]
    } else {
        &[]
    };
    (rows, gaps)
}

pub(crate) const PILL_HEIGHT: i32 = 13;

/// Outer box of a pill at `top`; its width follows the content.
pub(crate) fn pill_box(pill: &PillText, top: i32) -> (Rectangle, i32) {
    let label_w = pill.label.map_or(2, |l| l.len() as i32 * 6 + 7);
    let w = label_w + pill.value.columns() as i32 * 6 + 8;
    let split = if pill.label.is_some() { 4 + label_w } else { 4 };
    (
        Rectangle::new(Point::new(4, top), Size::new(w as u32, PILL_HEIGHT as u32)),
        split,
    )
}

fn pills<D: DrawTarget<Color = BinaryColor>>(
    ink: &mut Ink<'_, D>,
    app: &RadarApp,
) -> Result<(), D::Error> {
    let (rows, gaps) = pill_rows(app);
    let mut y = 4;
    for (i, pill) in rows.iter().enumerate() {
        let Some(pill) = pill else { break };
        let (r, split) = pill_box(pill, y);
        let (x, w, h) = (r.top_left.x, r.size.width as i32, PILL_HEIGHT);
        ink.rounded_fill(r, |k| k < split)?;
        // Outline with the 2 px corner cut.
        ink.hline(x + 2, x + w - 3, y, true)?;
        ink.hline(x + 2, x + w - 3, y + h - 1, true)?;
        ink.vline(x, y + 2, y + h - 3, true)?;
        ink.vline(x + w - 1, y + 2, y + h - 3, true)?;
        for p in [
            (x + 1, y + 1),
            (x + w - 2, y + 1),
            (x + 1, y + h - 2),
            (x + w - 2, y + h - 2),
        ] {
            ink.px(Point::new(p.0, p.1), true)?;
        }
        let value = pill.value.as_str();
        if let Some(label) = pill.label {
            ink.vline(split, y, y + h - 1, true)?;
            ink.text(x + 4, y + 2, label, false)?;
            ink.text(
                x + w - 4 - pill.value.columns() as i32 * 6,
                y + 2,
                value,
                true,
            )?;
        } else {
            ink.text(x + 5, y + 2, value, true)?;
        }
        y += h + if gaps.contains(&i) { 6 } else { 3 };
    }
    Ok(())
}

// ------------------------------------------------------------ shading

/// Tone entries: half-pixel steps of screen distance from the aim point.
pub(crate) const SHADE_STEPS: usize = 2 * 240 + 1;

/// 0089's coastline tone, 0..=64 Bayer levels by screen distance: full inside
/// the ring where a 1,000 km satellite is above the horizon, a smoothstep drop
/// to 70% over ~3° of ground just past it, then sphere shading down to 12.5%
/// at the limb. The camera is straight above the observer, so the tone is a
/// function of screen distance alone (spherical Earth, as in the lab).
pub(crate) fn shade_levels(fov_deg: f64, radius_px: f64) -> [u8; SHADE_STEPS] {
    const RS: f64 = 6371.0;
    let d = RS + CAMERA_HEIGHT_KM;
    let ring = libm::acos(RS / (RS + 1000.0));
    let ring_cos =
        (d * libm::cos(ring) - RS) / libm::sqrt(d * d - 2.0 * d * RS * libm::cos(ring) + RS * RS);
    let half = libm::tan(fov_deg.to_radians() / 2.0);
    let (floor, mid) = (0.125, 0.7);
    core::array::from_fn(|i| {
        let r = i as f64 / 2.0;
        let alpha = libm::atan(r * half / radius_px);
        let s = d * libm::sin(alpha) / RS;
        // Ground angle from the observer and the surface's cosine toward the camera.
        let (ground, facing) = if s >= 1.0 {
            (core::f64::consts::FRAC_PI_2, 0.0)
        } else {
            (libm::asin(s) - alpha, libm::sqrt(1.0 - s * s))
        };
        let tone = if ground <= ring {
            1.0
        } else {
            let t = (facing / ring_cos).clamp(0.0, 1.0);
            let w = ((ground - ring) / 3f64.to_radians()).clamp(0.0, 1.0);
            let w = w * w * (3.0 - 2.0 * w);
            (1.0 - w) + w * (floor + (mid - floor) * (1.0 - libm::pow(1.0 - t, 2.2)))
        };
        libm::round(tone * 64.0) as u8
    })
}

/// 8×8 ordered-dither thresholds 0..64.
const BAYER8: [[u8; 8]; 8] = {
    let mut m = [[0u8; 8]; 8];
    let mut y = 0;
    while y < 8 {
        let mut x = 0;
        while x < 8 {
            let mut v = 0u8;
            let mut b = 0;
            while b < 3 {
                let xb = ((x >> b) & 1) as u8;
                let yb = ((y >> b) & 1) as u8;
                v |= ((xb ^ yb) << (2 * (2 - b) + 1)) | (yb << (2 * (2 - b)));
                b += 1;
            }
            m[y][x] = v;
            x += 1;
        }
        y += 1;
    }
    m
};

/// Whether the shaded coastline keeps an On pixel at `p` (`center` = aim point).
pub(crate) fn shade_keeps(levels: &[u8; SHADE_STEPS], center: Point, p: Point) -> bool {
    let (dx, dy) = ((p.x - center.x) as f32, (p.y - center.y) as f32);
    let i = (libm::sqrtf(dx * dx + dy * dy) * 2.0 + 0.5) as usize;
    let level = levels.get(i).copied().unwrap_or(0);
    BAYER8[(p.y & 7) as usize][(p.x & 7) as usize] < level
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use overhead_core::sgp4::chrono::NaiveDate;

    #[test]
    fn marker_shapes() {
        let oct: [(i32, i32); 37] = core::array::from_fn(|i| octagon().nth(i).unwrap());
        assert_eq!(octagon().count(), 37);
        for (x, y) in oct {
            assert!(oct.contains(&(-x, y)) && oct.contains(&(y, x)));
        }
        assert!(!oct.contains(&(3, 3)) && oct.contains(&(3, 1)) && oct.contains(&(0, 0)));
        // Hollow, 5 px across.
        assert!(!DIAMOND5.contains(&(0, 0)));
        assert_eq!(DIAMOND5.iter().map(|p| p.0).max(), Some(2));
    }

    #[test]
    fn pulse_follows_frame_time() {
        let t = |ms: u32| {
            NaiveDate::from_ymd_opt(2026, 11, 22)
                .unwrap()
                .and_hms_milli_opt(0, 0, ms / 1000, ms % 1000)
                .unwrap()
        };
        assert_eq!(pulse_phase(None), 0.0);
        assert_eq!(pulse_phase(Some(t(0))), 0.0);
        assert_eq!(pulse_phase(Some(t(700))), 0.5);
        assert_eq!(pulse_phase(Some(t(1400))), 0.0);
    }

    #[test]
    fn ruler_steps_and_labels() {
        for kpp in [0.001, 0.05, 1.0, 7.3, 80.0, 400.0] {
            let step = ruler_step_km(kpp);
            assert!(
                step / kpp <= 32.0 && step / kpp > 12.0,
                "kpp={kpp} step={step}"
            );
        }
        let label = |km: f64| std::string::String::from(ruler_label(km).as_str());
        assert_eq!(label(0.0), "0");
        assert_eq!(label(0.25), "0.25");
        assert_eq!(label(2.5), "2.5");
        assert_eq!(label(500.0), "500");
        assert_eq!(label(2500.0), "2.5k");
        assert_eq!(label(40_000.0), "40k");
        // 20° vertical lens on the 117 px radius: ~113 km/px.
        assert!((km_per_px(20.0, 117.0) - 113.03).abs() < 0.01);
    }

    #[test]
    fn shading_is_full_inside_the_ring_then_falls_to_the_limb_floor() {
        let radius = 117.0;
        // A tight lens is wholly inside the 1,000 km ring out to the corners.
        let corner = 2 * 234;
        assert!(shade_levels(1.0, radius)[..corner].iter().all(|&l| l == 64));
        let levels = shade_levels(20.0, radius);
        let first = levels.iter().position(|&l| l < 64).unwrap();
        assert!(levels[..first].iter().all(|&l| l == 64));
        assert!(levels[first..].windows(2).all(|w| w[1] <= w[0]));
        // Earth's limb at 20°: ~55 px. 12.5% floor (8/64) at and beyond it,
        // 70% (~45/64) just past the drop.
        assert!(levels[2 * 60..].iter().all(|&l| l == 8));
        assert!(levels[first..].iter().any(|&l| (40..=46).contains(&l)));
        // The ordered dither keeps about that share of pixels.
        let center = Point::new(200, 120);
        let kept = |level: u8| {
            let levels = [level; SHADE_STEPS];
            (0..64)
                .filter(|i| shade_keeps(&levels, center, Point::new(i % 8, i / 8)))
                .count()
        };
        assert_eq!((kept(0), kept(8), kept(32), kept(64)), (0, 8, 32, 64));
    }
}
