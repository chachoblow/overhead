//! Throwaway board test of the radar lab's "Favourite C" (README on the
//! `radar-lab` branch). Full-width radar, left pills, 7 px octagon markers
//! with cleared halos, below-horizon group members as a 5 px hollow diamond, sphere
//! shading of the coastline beyond the 1,000 km sky ring, past-line trails,
//! pulsing ring (no crosshair) and solid orbit line for the selected satellite,
//! scale ruler, arrows with cleared halos. Not a decided design.
extern crate alloc;

use alloc::{collections::VecDeque, format, string::String, vec, vec::Vec};
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
    projection::{CAMERA_HEIGHT_KM, CameraPoint, Viewport},
    visibility::Visibility,
};
use overhead_core::{
    EcefPosition, TemePosition,
    sgp4::chrono::{NaiveDateTime, TimeDelta, Timelike},
    teme_to_ecef,
};

use crate::{CoastLayer, Layout, LineBuffer};

const W: i32 = 400;
const H: i32 = 240;
/// Spherical Earth radius used by the lab for occlusion and shading (km).
const RS: f64 = 6371.0;
/// Orbit line: points over one period (lab: 160 segments).
const ORBIT_N: usize = 160;
/// Propagations per frame while an orbit fills, then while it refreshes.
const ORBIT_FILL: usize = 12;
const ORBIT_REFRESH: usize = 2;
/// Trail history: one snapshot of every object per 5 s, 49 kept (240 s).
const HISTORY_STEP_S: i64 = 5;
const HISTORY_LEN: usize = 49;

/// Long-lived renderer state for the board test (kept in `PeriodCache`).
#[derive(Debug, Default)]
pub struct FavState {
    orbit_id: Option<u64>,
    /// TEME km per orbit point; `None` until propagated (or failed).
    orbit: Vec<Option<[f64; 3]>>,
    orbit_cursor: usize,
    orbit_filled: usize,
    history_ids: Vec<u64>,
    history: VecDeque<(NaiveDateTime, Vec<[f32; 3]>)>,
    tone_fov: Option<f64>,
    tone: Vec<f32>,
}

impl FavState {
    pub const fn new() -> Self {
        Self {
            orbit_id: None,
            orbit: Vec::new(),
            orbit_cursor: 0,
            orbit_filled: 0,
            history_ids: Vec::new(),
            history: VecDeque::new(),
            tone_fov: None,
            tone: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------- pixels

/// Collects pixels and writes them in one `draw_iter` per primitive.
struct Ink<'a, D> {
    d: &'a mut D,
}

impl<D: DrawTarget<Color = BinaryColor>> Ink<'_, D> {
    fn px(&mut self, x: i32, y: i32, on: bool) -> Result<(), D::Error> {
        if !(0..W).contains(&x) || !(0..H).contains(&y) {
            return Ok(());
        }
        self.d
            .draw_iter([Pixel(Point::new(x, y), BinaryColor::from(on))])
    }
    /// Bresenham as in the lab's `FB.line`; `dash` keeps every dash-th pixel.
    fn line(&mut self, a: (f64, f64), b: (f64, f64), dash: u32, on: bool) -> Result<(), D::Error> {
        let (mut x0, mut y0, x1, y1) = (rnd(a.0), rnd(a.1), rnd(b.0), rnd(b.1));
        if (x1 - x0).abs() > 4000 || (y1 - y0).abs() > 4000 {
            return Ok(());
        }
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let mut err = dx + dy;
        let mut i = 0;
        loop {
            if dash == 0 || i % dash == 0 {
                self.px(x0, y0, on)?;
            }
            i += 1;
            if x0 == x1 && y0 == y1 {
                return Ok(());
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }
    fn circle(&mut self, cx: i32, cy: i32, r: f64, dash: usize, on: bool) -> Result<(), D::Error> {
        let n = libm::ceil(r * 6.3).max(12.0) as usize;
        for k in 0..n {
            if dash != 0 && k % dash != 0 {
                continue;
            }
            let a = k as f64 / n as f64 * core::f64::consts::TAU;
            self.px(
                cx + rnd(r * libm::cos(a)),
                cy + rnd(r * libm::sin(a)),
                on,
            )?;
        }
        Ok(())
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
    /// Light/dark fill with the pills' 2 px corner cut.
    fn rounded_fill(&mut self, x: i32, y: i32, w: i32, h: i32, on: impl Fn(i32) -> bool) -> Result<(), D::Error> {
        for j in 0..h {
            let inset = if j == 0 || j == h - 1 {
                2
            } else if j == 1 || j == h - 2 {
                1
            } else {
                0
            };
            for k in x + inset..x + w - inset {
                self.px(k, y + j, on(k))?;
            }
        }
        Ok(())
    }
}

fn rnd(v: f64) -> i32 {
    // Math.round: halves toward +infinity.
    libm::floor(v + 0.5) as i32
}

/// Only lets On pixels through where the sphere-shading tone keeps them.
struct Toned<'a, D> {
    d: &'a mut D,
    tone: &'a [f32],
}

impl<D: DrawTarget<Color = BinaryColor>> Dimensions for Toned<'_, D> {
    fn bounding_box(&self) -> Rectangle {
        self.d.bounding_box()
    }
}

impl<D: DrawTarget<Color = BinaryColor>> DrawTarget for Toned<'_, D> {
    type Color = BinaryColor;
    type Error = D::Error;
    fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(&mut self, pixels: I) -> Result<(), D::Error> {
        let tone = self.tone;
        self.d.draw_iter(pixels.into_iter().filter(|Pixel(p, c)| {
            *c == BinaryColor::Off || keep(tone, p.x, p.y)
        }))
    }
}

/// 8×8 ordered-dither thresholds, built like the lab's `BAYER8`.
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

/// Tone table index: half-pixel steps of screen distance from the centre.
fn keep(tone: &[f32], x: i32, y: i32) -> bool {
    let (dx, dy) = ((x - W / 2) as f32, (y - H / 2) as f32);
    let i = (libm::sqrtf(dx * dx + dy * dy) * 2.0 + 0.5) as usize;
    let f = tone.get(i).copied().unwrap_or(0.0);
    (f32::from(BAYER8[(y & 7) as usize][(x & 7) as usize]) + 0.5) / 64.0 < f
}

/// The lab's "sphere1000deepoutlinefade": full inside the ring where
/// 1,000 km satellites are above the horizon, a smoothstep drop to 70% over
/// ~3° of ground just past it, then sphere shading down to 12.5% at the limb.
/// The camera is straight above the observer, so the tone depends only on
/// screen distance from the centre.
fn tone_table(fov_deg: f64, radius_px: f64) -> Vec<f32> {
    let d = RS + CAMERA_HEIGHT_KM;
    let l0 = libm::acos(RS / (RS + 1000.0));
    let mu0 = (d * libm::cos(l0) - RS) / libm::sqrt(d * d - 2.0 * d * RS * libm::cos(l0) + RS * RS);
    let half = libm::tan(fov_deg.to_radians() / 2.0);
    let (lo, mid) = (0.125, 0.7);
    (0..=2 * 240)
        .map(|i| {
            let r = i as f64 / 2.0;
            let alpha = libm::atan(r * half / radius_px);
            let s = d * libm::sin(alpha) / RS;
            let (lam, mu) = if s >= 1.0 {
                (core::f64::consts::FRAC_PI_2, 0.0)
            } else {
                (libm::asin(s) - alpha, libm::sqrt(1.0 - s * s))
            };
            if lam <= l0 {
                return 1.0;
            }
            let tt = (mu / mu0).clamp(0.0, 1.0);
            let w = ((lam - l0) / 3f64.to_radians()).clamp(0.0, 1.0);
            let ws = w * w * (3.0 - 2.0 * w);
            ((1.0 - ws) + ws * (lo + (mid - lo) * (1.0 - libm::pow(1.0 - tt, 2.2)))) as f32
        })
        .collect()
}

// ------------------------------------------------------------- geometry

struct Geo {
    cx: f64,
    cy: f64,
    rx: f64,
    ry: f64,
    /// Earth centre in camera coordinates.
    centre: [f64; 3],
}

impl Geo {
    fn new(app: &RadarApp, layout: Layout) -> Option<Self> {
        let c = app
            .camera()
            .project(EcefPosition::from_km([0.0; 3]))
            .ok()?;
        Some(Self {
            cx: f64::from(layout.center().x),
            cy: f64::from(layout.center().y),
            rx: f64::from(layout.horizontal_radius()),
            ry: f64::from(layout.radius()),
            centre: [c.east_km, c.north_km, c.depth_km],
        })
    }
    fn screen(&self, app: &RadarApp, p: CameraPoint) -> Option<(f64, f64)> {
        let v = app.view().project_unclipped(p)?;
        Some((self.cx + v.east * self.rx, self.cy - v.north * self.ry))
    }
    /// Segment camera→p enters the Earth sphere before reaching p.
    fn occluded(&self, p: CameraPoint) -> bool {
        let p = [p.east_km, p.north_km, p.depth_km];
        let c = self.centre;
        let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let a = dot(p, p);
        let b = -2.0 * dot(p, c);
        let cc = dot(c, c) - RS * RS;
        let disc = b * b - 4.0 * a * cc;
        if disc < 0.0 || a == 0.0 {
            return false;
        }
        let t = (-b - libm::sqrt(disc)) / (2.0 * a);
        t > 0.0 && t < 1.0
    }
}

fn camera_of(app: &RadarApp, ecef: [f64; 3]) -> Option<CameraPoint> {
    app.camera().project(EcefPosition::from_km(ecef)).ok()
}

/// 7 px octagon: |a|,|b| <= 3 and |a|+|b| <= 4.
fn octagon() -> impl Iterator<Item = (i32, i32)> {
    (-3..=3).flat_map(|b| (-3..=3).map(move |a| (a, b))).filter(|(a, b): &(i32, i32)| a.abs() + b.abs() <= 4)
}

/// 5 px hollow diamond (lab "diamondhalo").
const DIAMOND5: [(i32, i32); 8] = [(0, -2), (1, -1), (2, 0), (1, 1), (0, 2), (-1, 1), (-2, 0), (-1, -1)];

// ---------------------------------------------------------------- frame

pub(crate) fn draw<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    app: &RadarApp,
    layout: Layout,
    ingestion_warning: bool,
    coast: Option<&mut CoastLayer>,
    state: &mut FavState,
) -> Result<(), D::Error> {
    let mut display = display.clipped(&Rectangle::new(Point::zero(), crate::DISPLAY_SIZE));
    display.clear(BinaryColor::Off)?;
    let view = app.view();
    if state.tone_fov != Some(view.fov_deg()) {
        state.tone = tone_table(view.fov_deg(), f64::from(layout.radius()));
        state.tone_fov = Some(view.fov_deg());
    }
    let mut coast_error = false;
    if let Some(coast) = coast {
        let mut toned = Toned {
            d: &mut display,
            tone: &state.tone,
        };
        if !coast.draw(&mut toned, app.observer(), view, layout)? {
            display.clear(BinaryColor::Off)?;
            coast_error = true;
        }
    }
    // Outline stays solid.
    crate::draw_earth(&mut display, app.earth_limb(), layout, Viewport::Rectangle)?;

    record_history(app, state);
    let Some(geo) = Geo::new(app, layout) else {
        return crate::draw_wide_status(&mut display, app, ingestion_warning, coast_error);
    };
    let mut ink = Ink { d: &mut display };
    let selected = app.selected();
    let sel_id = selected.map(|(e, _)| e.elements().norad_id);

    // Selected orbit, solid, broken where the Earth hides it.
    update_orbit(app, state);
    if let (Some(now), Some(_)) = (app.time(), sel_id) {
        // Earth rotation at the frame time, from two unit vectors.
        let ex = teme_to_ecef(TemePosition::from_km([1.0, 0.0, 0.0]), now).ok();
        let ey = teme_to_ecef(TemePosition::from_km([0.0, 1.0, 0.0]), now).ok();
        if let (Some(ex), Some(ey)) = (ex, ey) {
            let (ex, ey) = (ex.km(), ey.km());
            let mut prev = None;
            for p in &state.orbit {
                let q = p.and_then(|[x, y, z]| {
                    let e = [ex[0] * x + ey[0] * y, ex[1] * x + ey[1] * y, z];
                    let c = camera_of(app, e)?;
                    (!geo.occluded(c)).then(|| geo.screen(app, c)).flatten()
                });
                if let (Some(a), Some(b)) = (prev, q) {
                    ink.line(a, b, 0, true)?;
                }
                prev = q;
            }
        }
    }

    // Past-line trails (not the selected satellite: its orbit shows motion).
    if let Some(now) = app.time() {
        let dt = trail_dt(view.fov_deg());
        for (i, (entry, result)) in app.objects().enumerate() {
            let Ok(sample) = result else { continue };
            if !sample.above_horizon() || Some(entry.elements().norad_id) == sel_id {
                continue;
            }
            let Some(mut prev) = geo.screen(app, sample.camera) else {
                continue;
            };
            for k in 1..=4 {
                let t = now - TimeDelta::milliseconds((k as f64 * dt * 1000.0) as i64);
                let Some(q) = past(state, i, t)
                    .and_then(|e| camera_of(app, e))
                    .and_then(|c| geo.screen(app, c))
                else {
                    break;
                };
                ink.line(prev, q, 0, true)?;
                prev = q;
            }
        }
    }

    // Below-horizon members of the group: 5 px hollow diamond, cleared halo.
    for (_, result) in app.objects() {
        let Ok(sample) = result else { continue };
        if sample.above_horizon() || geo.occluded(sample.camera) {
            continue;
        }
        let Some(v) = view.locate(sample.camera) else {
            continue;
        };
        let (x, y) = (rnd(geo.cx + v.east * geo.rx), rnd(geo.cy - v.north * geo.ry));
        for (a, b) in DIAMOND5 {
            for j in -1..=1 {
                for i in -1..=1 {
                    ink.px(x + a + i, y + b + j, false)?;
                }
            }
        }
        for (a, b) in DIAMOND5 {
            ink.px(x + a, y + b, true)?;
        }
    }

    // Above-horizon markers: octagon after its own cleared halo.
    let mut sel_px = None;
    for (entry, _, v) in app.markers() {
        let (x, y) = (rnd(geo.cx + v.east * geo.rx), rnd(geo.cy - v.north * geo.ry));
        if Some(entry.elements().norad_id) == sel_id {
            sel_px = Some((x, y));
            continue;
        }
        for (a, b) in octagon() {
            for j in -1..=1 {
                for i in -1..=1 {
                    ink.px(x + a + i, y + b + j, false)?;
                }
            }
        }
        for (a, b) in octagon() {
            ink.px(x + a, y + b, true)?;
        }
    }

    // Selected: its own octagon with the usual cleared halo, drawn last, and
    // the pulsing ring (no crosshair).
    if let Some((x, y)) = sel_px {
        let g = 7;
        for (a, b) in octagon() {
            for j in -1..=1 {
                for i in -1..=1 {
                    ink.px(x + a + i, y + b + j, false)?;
                }
            }
        }
        for (a, b) in octagon() {
            ink.px(x + a, y + b, true)?;
        }
        let ph = pulse_phase(app.time());
        ink.circle(x, y, f64::from(g - 1) + ph * 7.0, if ph > 0.6 { 2 } else { 0 }, true)?;
    }
    drop(ink);

    // Off-screen arrows with a cleared halo.
    for (_, indicator) in app.indicators() {
        for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
            let mut shifted = Shifted {
                d: &mut display,
                by: Point::new(dx, dy),
            };
            crate::draw_indicator(&mut shifted, indicator, layout)?;
        }
        crate::draw_indicator(&mut display, indicator, layout)?;
    }

    let mut ink = Ink { d: &mut display };
    scale_ruler(&mut ink, view.fov_deg(), f64::from(layout.radius()))?;
    pills(&mut ink, app)?;
    drop(ink);
    crate::draw_wide_status(&mut display, app, ingestion_warning, coast_error)
}

/// Writes every pixel shifted and Off: the 1 px cleared halo of a glyph.
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
    fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(&mut self, pixels: I) -> Result<(), D::Error> {
        let by = self.by;
        self.d.draw_iter(
            pixels
                .into_iter()
                .filter(|Pixel(_, c)| *c == BinaryColor::On)
                .map(|Pixel(p, _)| Pixel(p + by, BinaryColor::Off)),
        )
    }
}

fn pulse_phase(time: Option<NaiveDateTime>) -> f64 {
    let Some(t) = time else { return 0.0 };
    let ms = (t.num_seconds_from_midnight() as u64) * 1000 + u64::from(t.nanosecond() / 1_000_000);
    (ms % 1400) as f64 / 1400.0
}

/// Lab `trailDt`: seconds per trail step, scaled with the lens.
fn trail_dt(fov_deg: f64) -> f64 {
    (60.0 * libm::tan(fov_deg.to_radians() / 2.0) / libm::tan(10f64.to_radians())).clamp(1.0, 900.0)
}

// -------------------------------------------------------------- history

fn record_history(app: &RadarApp, state: &mut FavState) {
    let Some(now) = app.time() else {
        state.history.clear();
        return;
    };
    let ids_match = state.history_ids.len() == app.objects().count()
        && app
            .objects()
            .zip(&state.history_ids)
            .all(|((e, _), id)| e.elements().norad_id == *id);
    if !ids_match {
        state.history_ids = app.objects().map(|(e, _)| e.elements().norad_id).collect();
        state.history.clear();
    }
    if let Some((last, _)) = state.history.back() {
        if now < *last {
            state.history.clear();
        } else if now - *last < TimeDelta::seconds(HISTORY_STEP_S) {
            return;
        }
    }
    let mut snap = if state.history.len() >= HISTORY_LEN {
        state.history.pop_front().map(|(_, v)| v).unwrap_or_default()
    } else {
        Vec::new()
    };
    snap.clear();
    snap.extend(app.objects().map(|(_, r)| match r {
        Ok(s) => s.physical.ecef.km().map(|v| v as f32),
        Err(_) => [f32::NAN; 3],
    }));
    state.history.push_back((now, snap));
}

/// ECEF of object `i` at `t`, interpolated between snapshots.
fn past(state: &FavState, i: usize, t: NaiveDateTime) -> Option<[f64; 3]> {
    let h = &state.history;
    let k = h.iter().position(|(ts, _)| *ts >= t)?;
    let (t1, v1) = &h[k];
    let p1 = v1.get(i)?.map(f64::from);
    if *t1 == t || k == 0 {
        return (*t1 == t && p1.iter().all(|v| v.is_finite())).then_some(p1);
    }
    let (t0, v0) = &h[k - 1];
    let p0 = v0.get(i)?.map(f64::from);
    let span = (*t1 - *t0).num_milliseconds() as f64;
    let f = (t - *t0).num_milliseconds() as f64 / span;
    let p = core::array::from_fn(|j| p0[j] + (p1[j] - p0[j]) * f);
    p.iter().all(|v: &f64| v.is_finite()).then_some(p)
}

// ---------------------------------------------------------------- orbit

fn update_orbit(app: &RadarApp, state: &mut FavState) {
    let (Some(now), Some((entry, _)), Some(period)) =
        (app.time(), app.selected(), app.selected_period_minutes())
    else {
        state.orbit_id = None;
        state.orbit.clear();
        return;
    };
    let id = entry.elements().norad_id;
    if state.orbit_id != Some(id) {
        state.orbit_id = Some(id);
        state.orbit = vec![None; ORBIT_N + 1];
        state.orbit_cursor = 0;
        state.orbit_filled = 0;
    }
    let budget = if state.orbit_filled <= ORBIT_N { ORBIT_FILL } else { ORBIT_REFRESH };
    for _ in 0..budget {
        let k = state.orbit_cursor;
        let offset_s = (k as f64 / ORBIT_N as f64 - 0.5) * period * 60.0;
        let t = now + TimeDelta::milliseconds((offset_s * 1000.0) as i64);
        state.orbit[k] = entry
            .satellite()
            .state_at(t)
            .ok()
            .map(|s| s.position().km());
        state.orbit_cursor = (k + 1) % (ORBIT_N + 1);
        state.orbit_filled += 1;
    }
}

// ------------------------------------------------------------- overlays

fn scale_ruler<D: DrawTarget<Color = BinaryColor>>(ink: &mut Ink<'_, D>, fov_deg: f64, radius_px: f64) -> Result<(), D::Error> {
    let foc = radius_px / libm::tan(fov_deg.to_radians() / 2.0);
    let kpp = CAMERA_HEIGHT_KM / foc;
    let mut seg = 0.01;
    for e in -2..=6 {
        for m in [1.0, 2.0, 2.5, 5.0] {
            let v = m * libm::pow(10.0, f64::from(e));
            if v / kpp <= 32.0 {
                seg = v;
            }
        }
    }
    let sp = seg / kpp;
    let labels: [LineBuffer; 5] = core::array::from_fn(|k| fmt_km(seg * k as f64));
    let max_l = labels.iter().map(|l| l.as_str().len()).max().unwrap_or(1) as f64 * 6.0;
    let every = if sp >= max_l + 3.0 { 1 } else { 2 };
    let bar_w = rnd(sp * 4.0);
    let last_half = (labels[4].as_str().len() * 3) as i32;
    let total = 3 + bar_w + last_half + 2 + 12;
    let x0 = W - 4 - total - 3;
    let yb = H - 11;
    ink.rounded_fill(x0 - 8, yb - 16, bar_w + last_half + 2 + 12 + 13, 23, |_| false)?;
    ink.line((f64::from(x0), f64::from(yb + 3)), (f64::from(x0 + bar_w), f64::from(yb + 3)), 0, true)?;
    for (k, label) in labels.iter().enumerate() {
        let tx = x0 + rnd(sp * k as f64);
        let top = if k == 0 || k == 4 { yb - 1 } else { yb + 1 };
        ink.line((f64::from(tx), f64::from(top)), (f64::from(tx), f64::from(yb + 3)), 0, true)?;
        if k % every == 0 || k == 4 {
            let s = label.as_str();
            ink.text(tx - s.len() as i32 * 3, yb - 13, s, true)?;
        }
    }
    ink.text(x0 + bar_w + last_half + 2, yb - 13, "km", true)
}

/// Lab label format: up to two decimals, thousands as "k".
fn fmt_km(v: f64) -> LineBuffer {
    let mut out = LineBuffer::new(12);
    let v = libm::round(v * 100.0) / 100.0;
    let (v, suffix) = if v >= 1000.0 {
        (libm::round(v / 100.0) / 10.0, "k")
    } else {
        (v, "")
    };
    let mut num = LineBuffer::new(12);
    let _ = write!(num, "{v:.2}");
    let s = num.as_str();
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.')
    } else {
        s
    };
    let _ = write!(out, "{s}{suffix}");
    out
}

fn pills<D: DrawTarget<Color = BinaryColor>>(ink: &mut Ink<'_, D>, app: &RadarApp) -> Result<(), D::Error> {
    let mut lines: Vec<(Option<&'static str>, String)> = Vec::new();
    let value = |args: core::fmt::Arguments<'_>| alloc::fmt::format(args);
    let ends: &[usize] = if let Some((entry, sample)) = app.selected() {
        let truth = sample.physical;
        let name: String = entry
            .elements()
            .object_name
            .as_deref()
            .unwrap_or("")
            .chars()
            .filter(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == ' ' || *c == '-')
            .take(9)
            .collect();
        let deg = |v: f64| format!("{:>3}°", libm::round(v) as i64);
        let el = truth.look.elevation_rad.to_degrees();
        lines.push((None, value(format_args!("{}", entry.elements().norad_id))));
        lines.push((None, value(format_args!("{}", name.trim()))));
        lines.push((Some("EL"), deg(el)));
        lines.push((
            Some("AZ"),
            match truth.look.azimuth_rad {
                Some(az) => deg(libm::fmod(az.to_degrees() + 360.0, 360.0)),
                None => value(format_args!("--")),
            },
        ));
        lines.push((Some("ALT"), value(format_args!("{}km", libm::round(truth.geodetic.altitude_km) as i64))));
        lines.push((Some("RNG"), value(format_args!("{}km", libm::round(truth.look.range_km) as i64))));
        lines.push((Some("SPD"), value(format_args!("{:.1}km/s", truth.teme_speed_km_s))));
        if let Some(per) = app.selected_period_minutes() {
            lines.push((
                Some("PER"),
                if per < 1000.0 {
                    value(format_args!("{}m", libm::round(per) as i64))
                } else {
                    value(format_args!("{:.0}h", per / 60.0))
                },
            ));
        }
        if let Some(vis) = app.selected_visibility() {
            lines.push((
                None,
                value(format_args!(
                    "{}",
                    match vis {
                        Visibility::Visible => "VISIBLE",
                        Visibility::Eclipsed => "SHADOW",
                        Visibility::Daylight => "DAYLIGHT",
                    }
                )),
            ));
        }
        &[1, 3, 5, 7]
    } else if app.time().is_some() {
        let counts = app.counts();
        lines.push((None, value(format_args!("NO SEL"))));
        lines.push((Some("SKY"), value(format_args!("{}", counts.above_horizon))));
        lines.push((Some("VIEW"), value(format_args!("{}", counts.visible))));
        &[]
    } else {
        &[]
    };
    let h = 13;
    let mut y = 4;
    for (i, (lab, val)) in lines.iter().enumerate() {
        let val = val.as_str();
        let x = 4;
        let lab_w = lab.map_or(2, |l| l.len() as i32 * 6 + 7);
        let w = lab_w + val.chars().count() as i32 * 6 + 8;
        let split = if lab.is_some() { x + lab_w } else { x };
        ink.rounded_fill(x, y, w, h, |k| k < split)?;
        ink.line((f64::from(x + 2), f64::from(y)), (f64::from(x + w - 3), f64::from(y)), 0, true)?;
        ink.line((f64::from(x + 2), f64::from(y + h - 1)), (f64::from(x + w - 3), f64::from(y + h - 1)), 0, true)?;
        ink.line((f64::from(x), f64::from(y + 2)), (f64::from(x), f64::from(y + h - 3)), 0, true)?;
        ink.line((f64::from(x + w - 1), f64::from(y + 2)), (f64::from(x + w - 1), f64::from(y + h - 3)), 0, true)?;
        for (cx, cy) in [(x + 1, y + 1), (x + w - 2, y + 1), (x + 1, y + h - 2), (x + w - 2, y + h - 2)] {
            ink.px(cx, cy, true)?;
        }
        if let Some(lab) = lab {
            ink.line((f64::from(split), f64::from(y)), (f64::from(split), f64::from(y + h - 1)), 0, true)?;
            ink.text(x + 4, y + 2, lab, false)?;
            ink.text(x + w - 4 - val.chars().count() as i32 * 6, y + 2, val, true)?;
        } else {
            ink.text(x + 5, y + 2, val, true)?;
        }
        y += h + if ends.contains(&i) { 6 } else { 3 };
    }
    Ok(())
}
