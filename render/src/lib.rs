//! Platform-independent 400×240, 1-bit radar renderer. No framebuffer or clock
//! ownership; all readouts come from the app's true, timestamped frame.
#![no_std]

use core::fmt::{self, Write};
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, Triangle},
    text::{Baseline, Text},
};
use overhead_app::{
    RadarApp,
    projection::{EarthLimb, EdgeIndicator, ViewPoint, Viewport},
};
use overhead_core::sgp4::chrono::Timelike;

mod coast;
pub mod debug;
#[cfg(feature = "measurement")]
pub mod measurement;
pub mod setup;
mod wide;

pub use coast::CoastLayer;

pub const DISPLAY_SIZE: Size = Size::new(400, 240);
/// Wide radar size (0089: the full display), shared with the coast raster.
pub(crate) const WIDE_WIDTH: u32 = 400;
pub(crate) const WIDE_HEIGHT: u32 = 240;

#[derive(Debug, Clone, Copy)]
pub struct Layout {
    radar_width: u32,
    radar_height: u32,
}

impl Layout {
    /// Provisional visual-comparison knob. Circle stays vertically centered;
    /// smaller values trade radar area for a wider information area.
    pub fn new(radar_size: u32) -> Result<Self, &'static str> {
        if !(180..=240).contains(&radar_size) {
            return Err("radar size must be in [180, 240] pixels");
        }
        Ok(Self {
            radar_width: radar_size,
            radar_height: radar_size,
        })
    }
    /// 0089's full-width 400x240 radar with left pills and explicit notices.
    pub fn wide() -> Self {
        Self {
            radar_width: WIDE_WIDTH,
            radar_height: WIDE_HEIGHT,
        }
    }
    pub fn aspect_ratio(self) -> f64 {
        f64::from(self.horizontal_radius()) / f64::from(self.radius())
    }
    fn center(self) -> Point {
        Point::new(self.radar_width as i32 / 2, 120)
    }
    fn radius(self) -> i32 {
        self.radar_height as i32 / 2 - 3
    }
    fn horizontal_radius(self) -> i32 {
        self.radar_width as i32 / 2 - 3
    }
    fn contains(self, pixel: Point, viewport: Viewport) -> bool {
        let delta = pixel - self.center();
        let radius = self.radius();
        match viewport {
            Viewport::Circle | Viewport::BorderlessCircle => {
                delta.x * delta.x + delta.y * delta.y <= radius * radius
            }
            Viewport::Square | Viewport::Rectangle => {
                delta.x.abs() <= self.horizontal_radius() && delta.y.abs() <= radius
            }
        }
    }
    fn marker(self, p: ViewPoint) -> Point {
        let round = |v: f64| (v + if v < 0.0 { -0.5 } else { 0.5 }) as i32;
        self.center()
            + Point::new(
                round(p.east * f64::from(self.horizontal_radius())),
                -round(p.north * f64::from(self.radius())),
            )
    }
}

/// Draw an entire frame, clearing previous markers. `ingestion_warning` means
/// accepted catalogue data was accompanied by warnings printed by the host.
/// The wide view draws `coast`, prepared once for the app's fixed observer and
/// re-rastered only when the lens changes; a failed or mismatched layer shows
/// `COAST ERROR` instead of geography.
pub fn draw<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    app: &RadarApp,
    layout: Layout,
    ingestion_warning: bool,
    coast: &mut CoastLayer,
) -> Result<(), D::Error> {
    draw_frame(display, app, layout, ingestion_warning, Some(coast))
}

/// Provisional full-screen loading frame shown while the observer's coastline
/// is prepared (0058). Wording/layout are not a final contract; it never shows
/// geography, markers or notices from a previous preparation.
pub fn draw_preparing<D: DrawTarget<Color = BinaryColor>>(display: &mut D) -> Result<(), D::Error> {
    let mut display = display.clipped(&Rectangle::new(Point::zero(), DISPLAY_SIZE));
    display.clear(BinaryColor::Off)?;
    Text::with_baseline(
        PREPARING,
        Point::new(200 - PREPARING.len() as i32 * 3, 115),
        MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        Baseline::Top,
    )
    .draw(&mut display)?;
    Ok(())
}

const PREPARING: &str = "PREPARING MAP";

/// Full-screen install frame (0085): `UPDATING n%` while an update streams
/// into the other slot. Display and input pause during flash writes.
pub fn draw_updating<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    percent: u8,
) -> Result<(), D::Error> {
    let mut display = display.clipped(&Rectangle::new(Point::zero(), DISPLAY_SIZE));
    display.clear(BinaryColor::Off)?;
    let mut text = heapless_text::<16>();
    let _ = core::fmt::Write::write_fmt(&mut text, format_args!("UPDATING {}%", percent.min(100)));
    let s = text.as_str();
    Text::with_baseline(
        s,
        Point::new(200 - s.len() as i32 * 3, 115),
        MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        Baseline::Top,
    )
    .draw(&mut display)?;
    Ok(())
}

/// Fixed-capacity text for one line; longer writes are cut.
struct LineText<const N: usize> {
    buf: [u8; N],
    len: usize,
}

const fn heapless_text<const N: usize>() -> LineText<N> {
    LineText {
        buf: [0; N],
        len: 0,
    }
}

impl<const N: usize> LineText<N> {
    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

impl<const N: usize> core::fmt::Write for LineText<N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let take = s.len().min(N - self.len);
        self.buf[self.len..self.len + take].copy_from_slice(&s.as_bytes()[..take]);
        self.len += take;
        Ok(())
    }
}

/// `coast: None` omits geography without a notice: test/measurement controls only.
fn draw_frame<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    app: &RadarApp,
    layout: Layout,
    ingestion_warning: bool,
    coast: Option<&mut CoastLayer>,
) -> Result<(), D::Error> {
    draw_frame_with_coast(
        display,
        app,
        layout,
        ingestion_warning,
        coast.is_some(),
        |display, observer, view, layout| match coast {
            Some(coast) => coast.draw(display, observer, view, layout),
            None => Ok(true),
        },
    )
}

/// Measurement boundaries, in compositor order. The production wrapper passes
/// a no-op callback, monomorphized away. Timing the callback is diagnostic only.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum FrameStage {
    Clear,
    Coast,
    Earth,
    /// Square/circle diagnostics only: centre cross and "N".
    Radar,
    Markers,
    /// Wide only from here: haloed off-screen arrows.
    Arrows,
    /// Scale ruler and pills.
    Overlays,
    Status,
}

fn draw_frame_with_coast<'a, D, F>(
    display: &'a mut D,
    app: &RadarApp,
    layout: Layout,
    ingestion_warning: bool,
    coast_enabled: bool,
    coast_draw: F,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
    F: FnOnce(
        &mut embedded_graphics::draw_target::Clipped<'a, D>,
        overhead_core::GeodeticPosition,
        overhead_app::projection::View,
        Layout,
    ) -> Result<bool, D::Error>,
{
    draw_frame_traced(
        display,
        app,
        layout,
        ingestion_warning,
        coast_enabled,
        coast_draw,
        |_| {},
    )
}

fn draw_frame_traced<'a, D, F>(
    display: &'a mut D,
    app: &RadarApp,
    layout: Layout,
    ingestion_warning: bool,
    coast_enabled: bool,
    coast_draw: F,
    mut trace: impl FnMut(FrameStage),
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
    F: FnOnce(
        &mut embedded_graphics::draw_target::Clipped<'a, D>,
        overhead_core::GeodeticPosition,
        overhead_app::projection::View,
        Layout,
    ) -> Result<bool, D::Error>,
{
    let mut display = display.clipped(&Rectangle::new(Point::zero(), DISPLAY_SIZE));
    display.clear(BinaryColor::Off)?;
    trace(FrameStage::Clear);
    let mut coast_error = false;
    if app.view().viewport() == Viewport::Rectangle
        && coast_enabled
        && !coast_draw(&mut display, app.observer(), app.view(), layout)?
    {
        // A coast-only numeric error can occur after ink was emitted.
        // Repaint the frame from scratch instead of keeping partial ink.
        display.clear(BinaryColor::Off)?;
        coast_error = true;
    }
    trace(FrameStage::Coast);
    let stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 1);
    let center = layout.center();
    let radius = layout.radius();
    let viewport = app.view().viewport();
    if viewport == Viewport::Circle {
        Circle::with_center(center, (radius * 2 + 1) as u32)
            .into_styled(stroke)
            .draw(&mut display)?;
    }
    draw_earth(&mut display, app.earth_limb(), layout, viewport)?;
    trace(FrameStage::Earth);
    if viewport == Viewport::Rectangle {
        wide::draw_foreground(&mut display, app, layout, &mut trace)?;
        draw_wide_status(&mut display, app, ingestion_warning, coast_error)?;
        trace(FrameStage::Status);
        return Ok(());
    }
    Line::new(center - Point::new(3, 0), center + Point::new(3, 0))
        .into_styled(stroke)
        .draw(&mut display)?;
    Line::new(center - Point::new(0, 3), center + Point::new(0, 3))
        .into_styled(stroke)
        .draw(&mut display)?;
    Text::new(
        "N",
        Point::new(center.x - 2, center.y - radius + 12),
        MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
    )
    .draw(&mut display)?;
    trace(FrameStage::Radar);
    for (entry, _, point) in app.markers() {
        let selected = app.selected_id() == Some(entry.elements().norad_id);
        draw_marker(
            &mut display,
            layout.marker(point),
            selected,
            layout,
            viewport,
        )?;
    }
    trace(FrameStage::Markers);

    let x = layout.radar_width as i32 + 10;
    let columns = (400 - x) as usize / 6;
    let mut panel = Panel {
        display: &mut display,
        x,
        columns,
        y: 16,
    };
    panel.line(format_args!("OVERHEAD"))?;
    panel.line(format_args!("ORBIT VIEW / REPLAY"))?;
    if let Some(time) = app.time() {
        panel.line(format_args!("{} UTC", time.date()))?;
        panel.line(format_args!(
            "{:02}:{:02}:{:02}",
            time.hour(),
            time.minute(),
            time.second()
        ))?;
    } else {
        panel.line(format_args!("NO VALID FRAME"))?;
        panel.line(format_args!("--"))?;
    }
    panel.line(format_args!("FOV {:.2} DEG", app.view().fov_deg()))?;
    let counts = app.counts();
    panel.line(format_args!(
        "VIEW {} / SKY {}",
        counts.visible, counts.above_horizon
    ))?;
    panel.line(format_args!(
        "CAT {} / ERR {}",
        counts.catalogue, counts.failed
    ))?;
    panel.y += 8;
    if let Some((entry, sample)) = app.selected() {
        let truth = sample.physical;
        panel.line(format_args!(
            "{}",
            entry.elements().object_name.as_deref().unwrap_or("UNNAMED")
        ))?;
        panel.line(format_args!("NORAD {}", entry.elements().norad_id))?;
        panel.line(format_args!("ALT {:.0} KM", truth.geodetic.altitude_km))?;
        panel.line(format_args!("RNG {:.0} KM", truth.look.range_km))?;
        panel.line(format_args!(
            "EL {:.1} DEG",
            truth.look.elevation_rad.to_degrees()
        ))?;
        // Signed epoch age is not a freshness assessment (future epochs valid).
        if let Some(time) = app.time() {
            let age_hours = (time - entry.satellite().epoch()).num_seconds() as f64 / 3600.0;
            panel.line(format_args!("EPOCH {age_hours:+.1} H"))?;
        }
    } else {
        panel.line(format_args!(
            "{}",
            if app.time().is_none() {
                "AWAITING DATA"
            } else if counts.catalogue == 0 {
                "NO DATA"
            } else if counts.failed > 0 {
                "NO MARKERS / ERRORS"
            } else if counts.above_horizon > 0 {
                "NONE IN VIEW"
            } else {
                "NONE ABOVE HORIZON"
            }
        ))?;
    }
    panel.y = 216;
    panel.line(format_args!(
        "{}",
        if ingestion_warning {
            "INGEST WARN: SEE LOG"
        } else if app.data_stale() {
            "OLD DATA"
        } else if app.fetched_at().is_some() {
            "FETCHED DATA"
        } else {
            "LOCAL DATA / NO FETCH"
        }
    ))?;
    panel.line(format_args!("UP/DN ZOOM  TAB PICK"))?;
    Ok(())
}

fn draw_earth<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    limb: EarthLimb,
    layout: Layout,
    viewport: Viewport,
) -> Result<(), D::Error> {
    let radius = layout.radius();
    let horizontal_radius = layout.horizontal_radius();
    // Sample both rows and columns for a continuous one-bit ellipse. Work is
    // bounded by viewport diameter, not the zoomed ellipse's enormous extent.
    // All clipping happens before float-to-pixel conversion, then again after
    // rounding so neither the outline nor a glyph can leak out of the radar.
    let rows = (-radius..=radius).flat_map(|offset| {
        let coordinate = f64::from(offset) / f64::from(radius);
        limb.east_at(coordinate)
            .into_iter()
            .flatten()
            .map(move |east| ViewPoint {
                east,
                north: coordinate,
            })
    });
    let columns = (-horizontal_radius..=horizontal_radius).flat_map(|offset| {
        let coordinate = f64::from(offset) / f64::from(horizontal_radius);
        limb.north_at(coordinate)
            .into_iter()
            .flatten()
            .map(move |north| ViewPoint {
                east: coordinate,
                north,
            })
    });
    let pixels = rows.chain(columns).filter_map(|point| {
        if !viewport.contains(point) {
            return None;
        }
        let pixel = layout.marker(point);
        layout
            .contains(pixel, viewport)
            .then_some(Pixel(pixel, BinaryColor::On))
    });
    display.draw_iter(pixels)
}

fn draw_marker<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    marker: Point,
    selected: bool,
    layout: Layout,
    viewport: Viewport,
) -> Result<(), D::Error> {
    // Square/circle diagnostics only (the wide view draws 0089's octagons).
    // Clip every glyph pixel to the viewport, not just the center. A highlighted
    // edge marker must not spill into the information area.
    let pixels = (-3_i32..=3)
        .flat_map(|y| (-3_i32..=3).map(move |x| (x, y)))
        .filter_map(|(x, y)| {
            let distance = x.abs() + y.abs();
            let ink = if selected {
                distance == 3 || distance == 0
            } else {
                distance <= 1
            };
            let p = marker + Point::new(x, y);
            (ink && layout.contains(p, viewport)).then_some(Pixel(p, BinaryColor::On))
        });
    display.draw_iter(pixels)
}

/// Filled triangle (0061) pointing along the true image-plane direction. Intersect
/// the same ray with a slightly inset rectangle so the complete glyph fits.
/// No sorting, suppression, grouping or displacement to avoid other arrows.
fn draw_indicator<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    indicator: EdgeIndicator,
    layout: Layout,
) -> Result<(), D::Error> {
    let rx = f64::from(layout.horizontal_radius());
    let ry = f64::from(layout.radius());
    let x = indicator.edge.east * rx;
    let y = -indicator.edge.north * ry;
    let scale = ((rx - 3.0) / x.abs()).min((ry - 3.0) / y.abs());
    let x = x * scale;
    let y = y * scale;
    let dx = indicator.direction.east;
    let dy = -indicator.direction.north;
    let pixel = |x: f64, y: f64| {
        let round = |v: f64| (v + if v < 0.0 { -0.5 } else { 0.5 }) as i32;
        layout.center() + Point::new(round(x), round(y))
    };
    let tip = pixel(x, y);
    let a = pixel(x - 5.0 * dx - 2.0 * dy, y - 5.0 * dy + 2.0 * dx);
    let b = pixel(x - 5.0 * dx + 2.0 * dy, y - 5.0 * dy - 2.0 * dx);
    let bounds = Rectangle::new(
        layout.center() - Point::new(layout.horizontal_radius(), layout.radius()),
        Size::new(
            (2 * layout.horizontal_radius() + 1) as u32,
            (2 * layout.radius() + 1) as u32,
        ),
    );
    let mut clipped = display.clipped(&bounds);
    // Filled (0061): same outline pixels plus the interior, for contrast
    // against coastline and edges. Overlaps still merge, never suppressed.
    Triangle::new(tip, a, b)
        .into_styled(
            PrimitiveStyleBuilder::new()
                .fill_color(BinaryColor::On)
                .stroke_color(BinaryColor::On)
                .stroke_width(1)
                .build(),
        )
        .draw(&mut clipped)?;
    Ok(())
}

/// Condition-driven notices drawn last, above scene graphics (decision 0040).
/// Their backing may cover pixels, never change eligibility, counts or selection.
fn draw_wide_status<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    app: &RadarApp,
    ingestion_warning: bool,
    coast_error: bool,
) -> Result<(), D::Error> {
    let counts = app.counts();
    let status = wide_empty_status(
        app.time().is_some(),
        counts,
        app.indicators().next().is_some(),
        ingestion_warning,
    );
    draw_wide_notices(
        display,
        &[
            status,
            // 0075: fetched elements past the stale threshold are still
            // tracked; the notice clears only when a new fetch loads.
            app.data_stale().then_some("OLD DATA"),
            (counts.failed > 0).then_some("TRACK ERROR"),
            ingestion_warning.then_some("INGEST WARN"),
            coast_error.then_some("COAST ERROR"),
        ],
    )
}

// A horizon claim requires a complete, successful sample of a nonempty load.
// An arrow is evidence of tracked sky outside the viewport, not zoom advice
// for other objects (which may have invalid camera depth).
fn wide_empty_status(
    has_frame: bool,
    counts: overhead_app::Counts,
    has_indicator: bool,
    ingestion_warning: bool,
) -> Option<&'static str> {
    if !has_frame {
        Some("NO FRAME")
    } else if counts.catalogue == 0 {
        // No elements loaded (e.g. a device fetch pending or failed): an empty
        // sky claim would be unfounded, and no objects means no markers.
        Some("NO DATA")
    } else if counts.visible > 0 {
        None
    } else if has_indicator {
        Some("NONE IN VIEW")
    } else if counts.catalogue > 0
        && counts.above_horizon == 0
        && counts.failed == 0
        && !ingestion_warning
    {
        Some("NONE ABOVE HORIZON")
    } else {
        Some("NO MARKERS")
    }
}

fn draw_wide_notices<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    notices: &[Option<&str>],
) -> Result<(), D::Error> {
    let count = notices.iter().flatten().count() as i32;
    let Some(columns) = notices.iter().flatten().map(|line| line.len()).max() else {
        return Ok(()); // No ink and no reserved/cleared corner in healthy views.
    };
    let first_baseline = 222 - (count - 1) * 13;
    // 6x10 cells start 7px above the baseline; pad the complete block by 2px.
    Rectangle::new(
        Point::new(10, first_baseline - 9),
        Size::new(columns as u32 * 6 + 4, (count as u32 - 1) * 13 + 14),
    )
    .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
    .draw(display)?;
    for (row, line) in notices.iter().flatten().enumerate() {
        Text::new(
            line,
            Point::new(12, first_baseline + row as i32 * 13),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        )
        .draw(display)?;
    }
    Ok(())
}

struct Panel<'a, D> {
    display: &'a mut D,
    x: i32,
    y: i32,
    columns: usize,
}
impl<D: DrawTarget<Color = BinaryColor>> Panel<'_, D> {
    fn line(&mut self, args: fmt::Arguments<'_>) -> Result<(), D::Error> {
        let mut text = LineBuffer::new(self.columns);
        // LineBuffer is deliberately truncating and never returns fmt::Error.
        let _ = text.write_fmt(args);
        Text::new(
            text.as_str(),
            Point::new(self.x, self.y),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        )
        .draw(self.display)?;
        self.y += 13;
        Ok(())
    }
}

/// Truncating ASCII line of at most `limit` columns (`+` marks a cut), plus
/// the degree sign, which takes one column and two bytes.
struct LineBuffer {
    bytes: [u8; 40],
    len: usize,
    columns: usize,
    limit: usize,
}
impl LineBuffer {
    fn new(limit: usize) -> Self {
        Self {
            bytes: [0; 40],
            len: 0,
            columns: 0,
            limit: limit.min(20),
        }
    }
    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
    fn columns(&self) -> usize {
        self.columns
    }
    /// Append "°" (ISO 8859-1 fonts draw it) if a column is left.
    fn push_degree(&mut self) {
        if self.columns < self.limit {
            self.bytes[self.len..self.len + 2].copy_from_slice("°".as_bytes());
            self.len += 2;
            self.columns += 1;
        }
    }
}
impl Write for LineBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for ch in s.chars() {
            if self.columns == self.limit {
                // Only ASCII precedes a cut in practice; never split a degree sign.
                if self.limit > 0 && self.bytes[self.len - 1].is_ascii() {
                    self.bytes[self.len - 1] = b'+';
                }
                break;
            }
            // Names cannot inject newlines or draw outside the panel.
            self.bytes[self.len] = if ch.is_ascii() && !ch.is_ascii_control() {
                ch as u8
            } else {
                b'?'
            };
            self.len += 1;
            self.columns += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use overhead_app::projection::View;
    use overhead_core::{
        GeodeticPosition, OmmElements,
        catalogue::{CatalogueBuilder, RecordOrigin},
        ecef_to_geodetic,
    };
    use std::vec::Vec;

    // Public entry points with a layer freshly prepared for the app's observer.
    // Shadows the glob import so scene tests exercise the real wide coast path.
    fn draw<D: DrawTarget<Color = BinaryColor>>(
        display: &mut D,
        app: &RadarApp,
        layout: Layout,
        warning: bool,
    ) -> Result<(), D::Error> {
        super::draw(
            display,
            app,
            layout,
            warning,
            &mut CoastLayer::prepare(app.observer()),
        )
    }
    struct Screen {
        pixels: [[bool; 400]; 240],
    }
    impl OriginDimensions for Screen {
        fn size(&self) -> Size {
            DISPLAY_SIZE
        }
    }
    impl DrawTarget for Screen {
        type Color = BinaryColor;
        type Error = core::convert::Infallible;
        fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(
            &mut self,
            pixels: I,
        ) -> Result<(), Self::Error> {
            for Pixel(p, c) in pixels {
                assert!((0..400).contains(&p.x) && (0..240).contains(&p.y));
                self.pixels[p.y as usize][p.x as usize] = c == BinaryColor::On;
            }
            Ok(())
        }
    }
    fn app() -> RadarApp {
        app_with_viewport(Viewport::Circle)
    }
    fn app_with_viewport(viewport: Viewport) -> RadarApp {
        app_with_failure(viewport, false)
    }
    fn app_with_failure(viewport: Viewport, failure: bool) -> RadarApp {
        app_with_ids(viewport, failure, &[25544])
    }
    fn app_with_ids(viewport: Viewport, failure: bool, ids: &[u64]) -> RadarApp {
        app_with_orbits(viewport, failure, ids, 0.0)
    }
    fn app_with_orbits(
        viewport: Viewport,
        failure: bool,
        ids: &[u64],
        anomaly_step_deg: f64,
    ) -> RadarApp {
        let mut builder = CatalogueBuilder::new();
        let mut data: serde_json::Value =
            serde_json::from_str(include_str!("../../core/tests/fixtures/iss-25544.json")).unwrap();
        // Coincident by default for identity tests. The glyph lifecycle test
        // offsets mean anomaly to exercise selection of distinct physical values.
        // These synthetic orbits are test controls, not new physical references.
        let anomaly = data[0]["MEAN_ANOMALY"].as_f64().unwrap();
        for (record, &id) in ids.iter().enumerate() {
            data[0]["NORAD_CAT_ID"] = serde_json::json!(id);
            data[0]["MEAN_ANOMALY"] = serde_json::json!(anomaly + record as f64 * anomaly_step_deg);
            let elements: Vec<OmmElements> = serde_json::from_value(data.clone()).unwrap();
            builder.push(
                RecordOrigin { group: 0, record },
                elements.into_iter().next().unwrap(),
            );
        }
        if failure {
            let mut data: serde_json::Value =
                serde_json::from_str(include_str!("../../core/tests/fixtures/iss-25544.json"))
                    .unwrap();
            data[0]["NORAD_CAT_ID"] = serde_json::json!(99999);
            data[0]["MEAN_MOTION"] = serde_json::json!(1e300);
            let elements: Vec<OmmElements> = serde_json::from_value(data).unwrap();
            builder.push(
                RecordOrigin {
                    group: 0,
                    record: 1,
                },
                elements.into_iter().next().unwrap(),
            );
        }
        let catalogue = builder.finish().unwrap();
        let entry = &catalogue.entries()[0];
        let time = entry.satellite().epoch();
        let geo =
            ecef_to_geodetic(entry.satellite().state_at(time).unwrap().to_ecef().unwrap()).unwrap();
        let mut app = RadarApp::new(
            catalogue,
            GeodeticPosition {
                altitude_km: 0.0,
                ..geo
            },
            if viewport == Viewport::Rectangle {
                View::rectangular(20.0, Layout::wide().aspect_ratio()).unwrap()
            } else {
                View::new(20.0).unwrap().with_viewport(viewport)
            },
        )
        .unwrap();
        app.update(time).unwrap();
        app
    }
    #[test]
    fn real_frame_fits_both_layouts_and_is_reproducible() {
        let app = app();
        let truth = app.selected().unwrap().1.physical;
        for size in [180, 200, 240] {
            let mut screen = Screen {
                pixels: [[false; 400]; 240],
            };
            draw(&mut screen, &app, Layout::new(size).unwrap(), false).unwrap();
            assert!(screen.pixels.iter().flatten().filter(|&&v| v).count() > 500);
            let expected = screen.pixels;
            // Clear also removes stale pixels from a previous frame.
            screen.pixels = [[true; 400]; 240];
            draw(&mut screen, &app, Layout::new(size).unwrap(), false).unwrap();
            assert_eq!(screen.pixels, expected);
            assert_eq!(app.selected().unwrap().1.physical, truth);
        }
        assert!(Layout::new(179).is_err());
        assert!(Layout::new(241).is_err());
    }
    #[test]
    fn earth_outline_is_bounded_and_clipped_at_every_zoom() {
        use overhead_app::projection::Camera;
        let camera = Camera::new(GeodeticPosition {
            latitude_rad: 39.007_f64.to_radians(),
            longitude_rad: (-104.883_f64).to_radians(),
            altitude_km: 2.187,
        })
        .unwrap();
        for size in [180, 200, 240] {
            let layout = Layout::new(size).unwrap();
            for fov in [0.1, 5.0, 8.9, 9.0, 9.1, 20.0, 100.0] {
                let mut screen = Screen {
                    pixels: [[false; 400]; 240],
                };
                draw_earth(
                    &mut screen,
                    camera.earth_limb(View::new(fov).unwrap()),
                    layout,
                    Viewport::Circle,
                )
                .unwrap();
                let mut ink = 0;
                for (y, row) in screen.pixels.iter().enumerate() {
                    for (x, &on) in row.iter().enumerate() {
                        if on {
                            ink += 1;
                            let delta = Point::new(x as i32, y as i32) - layout.center();
                            assert!(
                                delta.x * delta.x + delta.y * delta.y
                                    <= layout.radius() * layout.radius()
                            );
                            if fov == 100.0 {
                                assert!(delta.x.abs() <= 8 && delta.y.abs() <= 8);
                            }
                        }
                    }
                }
                if fov <= 5.0 {
                    assert_eq!(ink, 0);
                }
                if fov >= 9.1 {
                    assert!(ink > 24);
                }
                // Four candidate boundary pixels per row/column index at most.
                assert!(ink <= 4 * (2 * layout.radius() + 1));
            }
        }
    }

    #[test]
    fn full_marker_is_clipped_at_circular_edge() {
        let mut screen = Screen {
            pixels: [[false; 400]; 240],
        };
        let center = Point::new(120, 120);
        let radius = 117;
        for offset in [
            Point::new(radius, 0),
            Point::new(-radius, 0),
            Point::new(0, radius),
            Point::new(0, -radius),
        ] {
            draw_marker(
                &mut screen,
                center + offset,
                true,
                Layout::new(240).unwrap(),
                Viewport::Circle,
            )
            .unwrap();
        }
        for (y, row) in screen.pixels.iter().enumerate() {
            for (x, &on) in row.iter().enumerate() {
                if on {
                    let dx = x as i32 - center.x;
                    let dy = y as i32 - center.y;
                    assert!(dx * dx + dy * dy <= radius * radius);
                }
            }
        }
    }
    #[test]
    fn borderless_changes_only_the_outer_ring() {
        let layout = Layout::new(240).unwrap();
        let mut circle = Screen {
            pixels: [[false; 400]; 240],
        };
        let mut borderless = Screen {
            pixels: [[false; 400]; 240],
        };
        draw(&mut circle, &app(), layout, false).unwrap();
        draw(
            &mut borderless,
            &app_with_viewport(Viewport::BorderlessCircle),
            layout,
            false,
        )
        .unwrap();
        assert_ne!(circle.pixels, borderless.pixels);
        Circle::with_center(layout.center(), (layout.radius() * 2 + 1) as u32)
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(&mut borderless)
            .unwrap();
        assert_eq!(circle.pixels, borderless.pixels);
    }

    #[test]
    fn square_clips_glyphs_and_earth_at_its_edges_not_the_circle() {
        let camera = overhead_app::projection::Camera::new(GeodeticPosition {
            latitude_rad: 39.007_f64.to_radians(),
            longitude_rad: (-104.883_f64).to_radians(),
            altitude_km: 2.187,
        })
        .unwrap();
        for size in [180, 200, 240] {
            let layout = Layout::new(size).unwrap();
            let mut screen = Screen {
                pixels: [[false; 400]; 240],
            };
            for east in [-1.0, 1.0] {
                for north in [-1.0, 1.0] {
                    let corner = layout.marker(ViewPoint { east, north });
                    draw_marker(&mut screen, corner, true, layout, Viewport::Square).unwrap();
                    assert!(screen.pixels[corner.y as usize][corner.x as usize]);
                }
            }
            for (y, row) in screen.pixels.iter().enumerate() {
                for (x, &on) in row.iter().enumerate() {
                    if on {
                        assert!(layout.contains(Point::new(x as i32, y as i32), Viewport::Square));
                    }
                }
            }
            let mut corner_limb = false;
            for fov in [0.1, 5.0, 8.0, 9.0, 20.0, 100.0] {
                screen.pixels = [[false; 400]; 240];
                draw_earth(
                    &mut screen,
                    camera.earth_limb(View::new(fov).unwrap()),
                    layout,
                    Viewport::Square,
                )
                .unwrap();
                for (y, row) in screen.pixels.iter().enumerate() {
                    for (x, &on) in row.iter().enumerate() {
                        if on {
                            let p = Point::new(x as i32, y as i32);
                            assert!(layout.contains(p, Viewport::Square));
                            corner_limb |= !layout.contains(p, Viewport::Circle);
                        }
                    }
                }
            }
            assert!(corner_limb);
        }
    }

    #[test]
    fn wide_earth_preserves_pixel_scale_and_every_glyph_stays_in_the_rectangle() {
        let camera = overhead_app::projection::Camera::new(GeodeticPosition {
            latitude_rad: 39.007_f64.to_radians(),
            longitude_rad: (-104.883_f64).to_radians(),
            altitude_km: 2.187,
        })
        .unwrap();
        let wide = Layout::wide();
        let square = Layout::new(240).unwrap();
        for fov in [0.1, 5.0, 8.0, 9.0, 20.0, 100.0] {
            let mut a = Screen {
                pixels: [[false; 400]; 240],
            };
            let mut b = Screen {
                pixels: [[false; 400]; 240],
            };
            draw_earth(
                &mut a,
                camera.earth_limb(View::new(fov).unwrap()),
                square,
                Viewport::Square,
            )
            .unwrap();
            draw_earth(
                &mut b,
                camera.earth_limb(View::rectangular(fov, wide.aspect_ratio()).unwrap()),
                wide,
                Viewport::Rectangle,
            )
            .unwrap();
            // Compare the shared interior. The old vertical clipping edges can
            // gain a rounded limb pixel when formerly outside geometry is admitted.
            for y in 3..=237 {
                for x in 4..237 {
                    assert_eq!(a.pixels[y][x], b.pixels[y][x + 80], "fov={fov} at {x},{y}");
                }
            }
            let ink = b.pixels.iter().flatten().filter(|&&on| on).count();
            assert!(ink <= 2 * (2 * wide.horizontal_radius() + 1 + 2 * wide.radius() + 1) as usize);
            for east in [-1.0, 1.0] {
                for north in [-1.0, 1.0] {
                    let pixel = wide.marker(ViewPoint { east, north });
                    draw_marker(&mut b, pixel, true, wide, Viewport::Rectangle).unwrap();
                    assert!(b.pixels[pixel.y as usize][pixel.x as usize]);
                }
            }
            for (y, row) in b.pixels.iter().enumerate() {
                for (x, &on) in row.iter().enumerate() {
                    if on {
                        assert!(wide.contains(Point::new(x as i32, y as i32), Viewport::Rectangle));
                    }
                }
            }
        }
    }

    #[test]
    fn arrows_are_filled_outward_and_clipped_including_corner_directions() {
        use overhead_app::projection::CameraPoint;
        let layout = Layout::wide();
        let view = View::rectangular(20.0, layout.aspect_ratio()).unwrap();
        for angle in (0..360).step_by(5) {
            let (north, east) = (angle as f64).to_radians().sin_cos();
            let arrow = view
                .edge_indicator(CameraPoint {
                    east_km: east,
                    north_km: north,
                    depth_km: 1.0,
                })
                .unwrap();
            let mut screen = Screen {
                pixels: [[false; 400]; 240],
            };
            draw_indicator(&mut screen, arrow, layout).unwrap();
            assert!(screen.pixels.iter().flatten().filter(|&&on| on).count() >= 10);
            for (y, row) in screen.pixels.iter().enumerate() {
                for (x, &on) in row.iter().enumerate() {
                    if on {
                        assert!(
                            layout.contains(Point::new(x as i32, y as i32), Viewport::Rectangle)
                        );
                        let delta = Point::new(x as i32, y as i32) - layout.center();
                        assert!(f64::from(delta.x) * east - f64::from(delta.y) * north > 0.0);
                    }
                }
            }
            if angle == 0 {
                assert!(screen.pixels[120][394]); // Outward tip.
                assert!(screen.pixels[118][389]); // Base corners.
                assert!(screen.pixels[122][389]);
                assert!(screen.pixels[120][391]); // Filled interior (0061).
            }
            let expected = screen.pixels;
            draw_indicator(&mut screen, arrow, layout).unwrap();
            assert_eq!(screen.pixels, expected); // Exact overlaps are permitted.
        }
    }

    fn assert_wide_notice(app: &RadarApp, warning: bool, lines: &[&str]) {
        let mut screen = Screen {
            pixels: [[true; 400]; 240],
        };
        draw(&mut screen, app, Layout::wide(), warning).unwrap();
        let expected = screen.pixels;
        draw(&mut screen, app, Layout::wide(), warning).unwrap();
        assert_eq!(screen.pixels, expected);
        let mut reference = Screen {
            pixels: [[false; 400]; 240],
        };
        if !lines.is_empty() {
            let top = 213 - (lines.len() - 1) * 13;
            let right = 10 + lines.iter().map(|line| line.len()).max().unwrap() * 6 + 4;
            for (row, line) in lines.iter().enumerate() {
                Text::new(
                    line,
                    Point::new(12, 222 - (lines.len() - 1 - row) as i32 * 13),
                    MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
                )
                .draw(&mut reference)
                .unwrap();
            }
            for y in top..227 {
                assert_eq!(screen.pixels[y][10..right], reference.pixels[y][10..right]);
            }
        }
    }

    #[test]
    fn empty_status_requires_positive_evidence_and_clean_horizon_samples() {
        use overhead_app::Counts;
        let clean = Counts {
            catalogue: 1,
            ..Counts::default()
        };
        assert_eq!(
            wide_empty_status(false, clean, true, false),
            Some("NO FRAME")
        );
        assert_eq!(
            wide_empty_status(
                true,
                Counts {
                    visible: 1,
                    ..clean
                },
                true,
                true
            ),
            None
        );
        assert_eq!(
            wide_empty_status(true, clean, true, false),
            Some("NONE IN VIEW")
        );
        assert_eq!(
            wide_empty_status(true, clean, true, true),
            Some("NONE IN VIEW")
        );
        assert_eq!(
            wide_empty_status(true, Counts { failed: 1, ..clean }, true, false),
            Some("NONE IN VIEW")
        );
        assert_eq!(
            wide_empty_status(true, clean, false, false),
            Some("NONE ABOVE HORIZON")
        );
        for warning in [false, true] {
            assert_eq!(
                wide_empty_status(true, Counts::default(), false, warning),
                Some("NO DATA")
            );
        }
        for (counts, warning) in [
            (
                Counts {
                    above_horizon: 1,
                    ..clean
                },
                false,
            ), // e.g. invalid camera depth
            (Counts { failed: 1, ..clean }, false),
            (clean, true),
        ] {
            assert_eq!(
                wide_empty_status(true, counts, false, warning),
                Some("NO MARKERS")
            );
        }
    }

    #[test]
    fn notice_backing_is_opaque_bounded_and_absent_without_notices() {
        for notices in [
            [None, None, None, None],
            [Some("NO FRAME"), None, None, None],
            [Some("NO MARKERS"), None, None, None],
            [Some("NONE IN VIEW"), None, None, None],
            [
                Some("NONE ABOVE HORIZON"),
                Some("TRACK ERROR"),
                Some("INGEST WARN"),
                None,
            ],
            [None, Some("TRACK ERROR"), Some("INGEST WARN"), None],
            [
                Some("NO MARKERS"),
                Some("TRACK ERROR"),
                Some("INGEST WARN"),
                None,
            ],
            [
                Some("NO MARKERS"),
                Some("TRACK ERROR"),
                Some("INGEST WARN"),
                Some("COAST ERROR"),
            ],
        ] {
            // Fully inked background is a worst-case collision: every cell,
            // inter-line gap and padding pixel must be cleared inside the mask.
            let mut screen = Screen {
                pixels: [[true; 400]; 240],
            };
            let mut expected = Screen {
                pixels: [[true; 400]; 240],
            };
            let lines: Vec<_> = notices.iter().flatten().copied().collect();
            if !lines.is_empty() {
                let top = match lines.len() {
                    1 => 213,
                    2 => 200,
                    3 => 187,
                    4 => 174,
                    _ => unreachable!(),
                };
                let right = 14 + lines.iter().map(|line| line.len()).max().unwrap() * 6;
                for row in &mut expected.pixels[top..227] {
                    row[10..right].fill(false);
                }
                for (row, line) in lines.iter().enumerate() {
                    Text::new(
                        line,
                        Point::new(12, top as i32 + 9 + row as i32 * 13),
                        MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
                    )
                    .draw(&mut expected)
                    .unwrap();
                }
            }
            draw_wide_notices(&mut screen, &notices).unwrap();
            assert_eq!(screen.pixels, expected.pixels);
        }
    }

    #[test]
    fn notices_only_change_their_backing_not_the_scene_or_app_state() {
        let app = app_with_viewport(Viewport::Rectangle);
        let counts = app.counts();
        let selected = app.selected_id();
        let samples: Vec<_> = app.objects().map(|(_, sample)| sample.clone()).collect();
        let mut clean = Screen {
            pixels: [[false; 400]; 240],
        };
        draw(&mut clean, &app, Layout::wide(), false).unwrap();
        let mut warned = Screen {
            pixels: [[false; 400]; 240],
        };
        draw(&mut warned, &app, Layout::wide(), true).unwrap();
        for y in 0..240 {
            for x in 0..400 {
                if !(10..80).contains(&x) || !(213..227).contains(&y) {
                    assert_eq!(clean.pixels[y][x], warned.pixels[y][x]);
                }
            }
        }
        assert_ne!(clean.pixels, warned.pixels);
        assert_eq!(app.counts(), counts);
        assert_eq!(app.selected_id(), selected);
        assert_eq!(
            app.objects().map(|(_, s)| s.clone()).collect::<Vec<_>>(),
            samples
        );
        draw(&mut warned, &app, Layout::wide(), false).unwrap();
        assert_eq!(warned.pixels, clean.pixels);
    }

    #[test]
    fn notice_overlay_is_drawn_after_a_crossing_selected_marker_and_id() {
        let elements: Vec<OmmElements> =
            serde_json::from_str(include_str!("../../core/tests/fixtures/iss-25544.json")).unwrap();
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
        let time = satellite.epoch();
        let geo = ecef_to_geodetic(satellite.state_at(time).unwrap().to_ecef().unwrap()).unwrap();
        // Deliberately choose a site/FOV putting the real ISS marker under the
        // notice. This is a collision control, not a production observer choice.
        let layout = Layout::wide();
        let mut app = RadarApp::new(
            catalogue,
            GeodeticPosition {
                latitude_rad: geo.latitude_rad + 0.48_f64.to_radians(),
                longitude_rad: geo.longitude_rad + 0.70_f64.to_radians(),
                altitude_km: 0.0,
            },
            View::rectangular(0.1, layout.aspect_ratio()).unwrap(),
        )
        .unwrap();
        app.update(time).unwrap();
        let marker = layout.marker(app.markers().next().unwrap().2);
        assert!(
            (13..77).contains(&marker.x) && (213..224).contains(&marker.y),
            "{marker:?}"
        );
        let counts = app.counts();
        let physical = app.selected().unwrap().1.physical;
        let mut clean = Screen {
            pixels: [[false; 400]; 240],
        };
        draw(&mut clean, &app, layout, false).unwrap();
        let mut expected = Screen {
            pixels: clean.pixels,
        };
        for row in &mut expected.pixels[213..227] {
            row[10..80].fill(false);
        }
        Text::new(
            "INGEST WARN",
            Point::new(12, 222),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        )
        .draw(&mut expected)
        .unwrap();
        let mut warned = Screen {
            pixels: clean.pixels,
        };
        draw(&mut warned, &app, layout, true).unwrap();
        assert_eq!(warned.pixels, expected.pixels);
        assert!((-3..=3).any(|dy| (-3..=3).any(|dx| {
            let x = (marker.x + dx) as usize;
            let y = (marker.y + dy) as usize;
            clean.pixels[y][x] && !warned.pixels[y][x]
        })));
        assert_eq!(app.selected_id(), Some(25544));
        assert_eq!(app.counts(), counts);
        assert_eq!(app.selected().unwrap().1.physical, physical);
        draw(&mut warned, &app, layout, false).unwrap();
        assert_eq!(warned.pixels, clean.pixels);
    }

    #[test]
    fn awaiting_catalogue_shows_no_data_then_clears_when_elements_load() {
        let loaded = app_with_viewport(Viewport::Rectangle);
        let time = loaded.time().unwrap();
        let mut app = RadarApp::awaiting_catalogue(loaded.observer(), loaded.view()).unwrap();
        assert_wide_notice(&app, false, &["NO FRAME"]);
        app.update(time).unwrap();
        assert_wide_notice(&app, false, &["NO DATA"]);
        assert_wide_notice(&app, true, &["NO DATA", "INGEST WARN"]);
        let mut builder = CatalogueBuilder::new();
        let elements: Vec<OmmElements> =
            serde_json::from_str(include_str!("../../core/tests/fixtures/iss-25544.json")).unwrap();
        builder.push(
            RecordOrigin {
                group: 0,
                record: 0,
            },
            elements.into_iter().next().unwrap(),
        );
        app.load_catalogue(builder.finish().unwrap());
        assert_eq!(app.counts(), loaded.counts());
        assert_wide_notice(&app, false, &[]);
    }

    #[test]
    fn old_data_notice_follows_fetch_age_and_clears_on_a_new_fetch() {
        let loaded = app_with_viewport(Viewport::Rectangle);
        let time = loaded.time().unwrap();
        let catalogue = || {
            let mut builder = CatalogueBuilder::new();
            let elements: Vec<OmmElements> =
                serde_json::from_str(include_str!("../../core/tests/fixtures/iss-25544.json"))
                    .unwrap();
            builder.push(
                RecordOrigin {
                    group: 0,
                    record: 0,
                },
                elements.into_iter().next().unwrap(),
            );
            builder.finish().unwrap()
        };
        let mut app = RadarApp::awaiting_catalogue(loaded.observer(), loaded.view())
            .unwrap()
            .with_freshness(overhead_app::Freshness::DEVICE);
        app.update(time).unwrap();
        let fetched = time - overhead_core::sgp4::chrono::TimeDelta::hours(25);
        app.load_fetched_catalogue(catalogue(), fetched);
        assert_eq!(app.counts(), loaded.counts()); // still tracked
        assert_wide_notice(&app, false, &["OLD DATA"]);
        assert_wide_notice(&app, true, &["OLD DATA", "INGEST WARN"]);
        // Same scene apart from the notice corner.
        let mut stale = Screen {
            pixels: [[false; 400]; 240],
        };
        let mut fresh = Screen {
            pixels: [[false; 400]; 240],
        };
        draw(&mut stale, &app, Layout::wide(), false).unwrap();
        app.load_fetched_catalogue(catalogue(), time);
        assert_wide_notice(&app, false, &[]);
        draw(&mut fresh, &app, Layout::wide(), false).unwrap();
        let mut reference = Screen {
            pixels: [[false; 400]; 240],
        };
        draw(&mut reference, &loaded, Layout::wide(), false).unwrap();
        assert_eq!(fresh.pixels, reference.pixels);
        assert_ne!(stale.pixels, fresh.pixels);
        for (y, (a, b)) in stale.pixels.iter().zip(&fresh.pixels).enumerate() {
            for (x, (p, q)) in a.iter().zip(b).enumerate() {
                if p != q {
                    assert!(x < 70 && y > 200, "difference outside notice at {x},{y}");
                }
            }
        }
    }

    #[test]
    fn wide_notices_remain_visible_with_valid_markers_and_clear_on_recovery() {
        let mut healthy = app_with_viewport(Viewport::Rectangle);
        assert_eq!(healthy.counts().visible, 1);
        assert_wide_notice(&healthy, false, &[]);
        assert_wide_notice(&healthy, true, &["INGEST WARN"]);
        let mut failed = app_with_failure(Viewport::Rectangle, true);
        assert_eq!(failed.counts().visible, 1);
        assert_eq!(failed.counts().failed, 1);
        assert_wide_notice(&failed, false, &["TRACK ERROR"]);
        assert_wide_notice(&failed, true, &["TRACK ERROR", "INGEST WARN"]);
        let mut screen = Screen {
            pixels: [[false; 400]; 240],
        };
        let mut clean = Screen {
            pixels: [[false; 400]; 240],
        };
        // Shrinking and disappearing overlays must restore the complete scene,
        // not just remove old text or leave a larger white patch behind.
        for (app, warning) in [
            (&failed, true),
            (&failed, false),
            (&healthy, true),
            (&healthy, false),
        ] {
            draw(&mut screen, app, Layout::wide(), warning).unwrap();
            clean.pixels = [[false; 400]; 240];
            draw(&mut clean, app, Layout::wide(), warning).unwrap();
            assert_eq!(screen.pixels, clean.pixels);
        }
        // About half an ISS orbit later, the formerly overhead object is below
        // this fixed site's horizon. Empty-marker text is not an empty-sky claim.
        let later = healthy.time().unwrap() + overhead_core::sgp4::chrono::Duration::minutes(45);
        healthy.update(later).unwrap();
        failed.update(later).unwrap();
        assert_eq!(healthy.counts().visible, 0);
        assert_eq!(failed.counts().visible, 0);
        assert_eq!(failed.counts().failed, 1);
        assert_eq!(healthy.counts().above_horizon, 0);
        assert_wide_notice(&healthy, false, &["NONE ABOVE HORIZON"]);
        assert_wide_notice(&healthy, true, &["NO MARKERS", "INGEST WARN"]);
        assert_wide_notice(&failed, true, &["NO MARKERS", "TRACK ERROR", "INGEST WARN"]);
        let mut horizon = Screen {
            pixels: [[false; 400]; 240],
        };
        let mut incomplete = Screen {
            pixels: [[false; 400]; 240],
        };
        draw(&mut horizon, &healthy, Layout::wide(), false).unwrap();
        draw(&mut incomplete, &healthy, Layout::wide(), true).unwrap();
        for y in 0..240 {
            for x in 0..400 {
                if !(10..122).contains(&x) || !(200..227).contains(&y) {
                    assert_eq!(horizon.pixels[y][x], incomplete.pixels[y][x]);
                }
            }
        }
        draw(&mut screen, &failed, Layout::wide(), true).unwrap();
        draw(&mut screen, &healthy, Layout::wide(), false).unwrap();
        draw(&mut clean, &healthy, Layout::wide(), false).unwrap();
        assert_eq!(screen.pixels, clean.pixels);
        let invalid = overhead_core::sgp4::chrono::NaiveDate::from_ymd_opt(2101, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert!(healthy.update(invalid).is_err());
        assert_wide_notice(&healthy, false, &["NO FRAME"]);
        assert_wide_notice(&healthy, true, &["NO FRAME", "INGEST WARN"]);
        draw(&mut screen, &healthy, Layout::wide(), true).unwrap();
        healthy.update(later).unwrap();
        draw(&mut screen, &healthy, Layout::wide(), false).unwrap();
        draw(&mut clean, &healthy, Layout::wide(), false).unwrap();
        assert_eq!(screen.pixels, clean.pixels);
    }

    #[test]
    fn baseline_radar_has_no_marker_adjacent_identity() {
        for viewport in [
            Viewport::Circle,
            Viewport::BorderlessCircle,
            Viewport::Square,
        ] {
            let mut a = Screen {
                pixels: [[false; 400]; 240],
            };
            let mut b = Screen {
                pixels: [[false; 400]; 240],
            };
            let layout = Layout::new(240).unwrap();
            draw(
                &mut a,
                &app_with_ids(viewport, false, &[25544]),
                layout,
                false,
            )
            .unwrap();
            draw(
                &mut b,
                &app_with_ids(viewport, false, &[100530]),
                layout,
                false,
            )
            .unwrap();
            assert_ne!(a.pixels, b.pixels); // Diagnostic NORAD field differs.
            for y in 0..240 {
                assert_eq!(a.pixels[y][..240], b.pixels[y][..240]);
            }
        }
    }

    #[test]
    fn coastline_is_a_background_in_selected_and_empty_notice_scenes() {
        let layout = Layout::wide();
        let mut app = app_with_viewport(Viewport::Rectangle);
        let mut coast_only_pixels = 0;
        for empty in [false, true] {
            if empty {
                let later =
                    app.time().unwrap() + overhead_core::sgp4::chrono::Duration::minutes(45);
                app.update(later).unwrap();
                assert_eq!(app.counts().visible, 0);
            } else {
                assert!(app.selected_id().is_some());
            }
            for warning in [false, true] {
                let mut coast = Screen {
                    pixels: [[false; 400]; 240],
                };
                let mut control = Screen {
                    pixels: [[false; 400]; 240],
                };
                let mut layer = CoastLayer::prepare(app.observer());
                draw_frame(&mut coast, &app, layout, warning, Some(&mut layer)).unwrap();
                draw_frame(&mut control, &app, layout, warning, None).unwrap();
                // Geographic ink may add to the background, but may not erase
                // existing marker, limb, strip or notice ink. Compare complete
                // frames; the earlier control isolates coast without CLI flags.
                for y in 0..240 {
                    for x in 0..400 {
                        assert!(
                            !control.pixels[y][x] || coast.pixels[y][x],
                            "coast erased foreground at ({x}, {y}), empty={empty} warning={warning}"
                        );
                        coast_only_pixels +=
                            usize::from(coast.pixels[y][x] && !control.pixels[y][x]);
                    }
                }
                assert_eq!(
                    coast
                        .pixels
                        .iter()
                        .map(|row| &row[352..])
                        .collect::<Vec<_>>(),
                    control
                        .pixels
                        .iter()
                        .map(|row| &row[352..])
                        .collect::<Vec<_>>()
                );
            }
        }
        assert!(
            coast_only_pixels > 0,
            "control never isolated visible coast ink"
        );
    }

    #[test]
    fn coastline_draws_only_in_wide_view_and_invalid_asset_has_explicit_notice() {
        let layout = Layout::wide();
        let app = app_with_viewport(Viewport::Rectangle);
        let mut with_coast = Screen {
            pixels: [[false; 400]; 240],
        };
        let mut without = Screen {
            pixels: [[false; 400]; 240],
        };
        draw(&mut with_coast, &app, layout, false).unwrap();
        draw_frame(&mut without, &app, layout, false, None).unwrap();
        // The chosen test observer is not guaranteed coastal: prove geographic
        // ink independently at the actual SF shore, not via a satellite fixture.
        let observer = GeodeticPosition {
            latitude_rad: 37.7749_f64.to_radians(),
            longitude_rad: (-122.4194_f64).to_radians(),
            altitude_km: 0.0,
        };
        let mut sf = CoastLayer::prepare(observer);
        assert_ne!(
            with_coast.pixels, without.pixels,
            "test observer shows no coast"
        );
        for fov in [0.1, 1.0, 5.0, 20.0, 100.0] {
            let mut ink = Screen {
                pixels: [[false; 400]; 240],
            };
            let view = View::rectangular(fov, layout.aspect_ratio()).unwrap();
            assert!(sf.draw(&mut ink, observer, view, layout).unwrap());
            assert!(
                ink.pixels.iter().flatten().any(|p| *p),
                "no coast at {fov} degrees"
            );
            for (y, row) in ink.pixels.iter().enumerate() {
                for (x, &pixel) in row.iter().enumerate() {
                    if pixel {
                        assert!(
                            layout.contains(Point::new(x as i32, y as i32), Viewport::Rectangle)
                        );
                    }
                }
            }
        }
        // An invalid preparation, or a layer prepared for another observer,
        // shows the exceptional notice rather than empty or stale geography.
        let mut expected = Screen {
            pixels: without.pixels,
        };
        draw_wide_notices(&mut expected, &[Some("COAST ERROR")]).unwrap();
        let mut invalid = CoastLayer::prepare(GeodeticPosition {
            latitude_rad: f64::NAN,
            ..app.observer()
        });
        assert!(invalid.error().is_some());
        for layer in [&mut invalid, &mut sf] {
            let mut broken = Screen {
                pixels: [[true; 400]; 240],
            };
            super::draw(&mut broken, &app, layout, false, layer).unwrap();
            assert_eq!(broken.pixels, expected.pixels);
        }
        // Failure does not invent a tracking error or change app state.
        assert_eq!(
            app.counts(),
            app_with_viewport(Viewport::Rectangle).counts()
        );
        for viewport in [
            Viewport::Square,
            Viewport::Circle,
            Viewport::BorderlessCircle,
        ] {
            let app = app_with_viewport(viewport);
            let square = Layout::new(240).unwrap();
            draw_frame(&mut with_coast, &app, square, false, Some(&mut invalid)).unwrap();
            draw_frame(&mut without, &app, square, false, None).unwrap();
            assert_eq!(with_coast.pixels, without.pixels);
        }
    }

    fn wide_screen(app: &RadarApp) -> Screen {
        let mut screen = Screen {
            pixels: [[true; 400]; 240],
        };
        draw(&mut screen, app, Layout::wide(), false).unwrap();
        screen
    }

    fn halo_of(shape: &[(i32, i32)]) -> Vec<(i32, i32)> {
        let mut halo = Vec::new();
        for (x, y) in shape {
            for j in -1..=1 {
                for i in -1..=1 {
                    let p = (x + i, y + j);
                    if !shape.contains(&p) && !halo.contains(&p) {
                        halo.push(p);
                    }
                }
            }
        }
        halo
    }

    #[test]
    fn selected_octagon_has_cleared_halo_and_pulsing_ring() {
        let app = app_with_viewport(Viewport::Rectangle);
        let (_, _, point) = app.markers().next().unwrap();
        assert_eq!(app.selected_id(), Some(25544));
        let at = Layout::wide().marker(point);
        let screen = wide_screen(&app);
        let ink = |(x, y): (i32, i32)| screen.pixels[(at.y + y) as usize][(at.x + x) as usize];
        let octagon: Vec<_> = wide::octagon().collect();
        assert!(octagon.iter().all(|&p| ink(p)));
        assert!(halo_of(&octagon).iter().all(|&p| !ink(p)));
        // The ring for this frame's phase, the only ink 5-14 px out besides
        // coastline and limb, which it is drawn over.
        let phase = wide::pulse_phase(app.time());
        let r = 6.0 + phase * 7.0;
        let n = libm::ceil(r * 6.3).max(12.0) as usize;
        let dash = if phase > 0.6 { 2 } else { 1 };
        for k in (0..n).step_by(dash) {
            let (s, c) = libm::sincos(k as f64 / n as f64 * core::f64::consts::TAU);
            let p = (
                libm::floor(r * c + 0.5) as i32,
                libm::floor(r * s + 0.5) as i32,
            );
            assert!(ink(p), "ring point {p:?} r={r}");
        }
    }

    #[test]
    fn below_horizon_group_members_are_hollow_diamonds_and_never_selected() {
        // ISS copies around the orbit: some above, some below the horizon.
        let mut app = app_with_orbits(
            Viewport::Rectangle,
            false,
            &[25544, 1, 2, 3, 4, 5, 6, 7],
            45.0,
        );
        app.zoom(-64);
        let below: Vec<_> = app
            .below_horizon_markers()
            .map(|(e, _, v)| (e.elements().norad_id, v))
            .collect();
        assert!(!below.is_empty() && app.markers().count() > 0);
        let screen = wide_screen(&app);
        let mut intact = 0;
        for (id, v) in &below {
            assert_ne!(app.selected_id(), Some(*id));
            let at = Layout::wide().marker(*v);
            let ink = |(x, y): (i32, i32)| screen.pixels[(at.y + y) as usize][(at.x + x) as usize];
            let hollow = wide::DIAMOND5.iter().all(|&p| ink(p)) && !ink((0, 0));
            intact += usize::from(hollow && halo_of(&wide::DIAMOND5).iter().all(|&p| !ink(p)));
        }
        assert!(intact > 0);
        // Not counted, no arrows.
        let counts = app.counts();
        assert_eq!(counts.above_horizon, app.markers().count());
        assert!(
            app.indicators()
                .all(|(e, _)| below.iter().all(|(id, _)| *id != e.elements().norad_id))
        );
    }

    #[test]
    fn pills_show_selected_truth_or_counts_and_nothing_without_a_frame() {
        let app = app_with_viewport(Viewport::Rectangle);
        let (rows, gaps) = wide::pill_rows(&app);
        let labels: Vec<_> = rows
            .iter()
            .flatten()
            .map(|p| p.label.unwrap_or(""))
            .collect();
        assert_eq!(
            &labels[..8],
            ["", "", "EL", "AZ", "ALT", "RNG", "SPD", "PER"]
        );
        assert_eq!(gaps, [1, 3, 5, 7]);
        let value = |i: usize| rows[i].as_ref().unwrap().value.as_str();
        let (_, sample) = app.selected().unwrap();
        assert_eq!(value(0), "25544");
        assert_eq!(value(1), "ISS ZARYA");
        assert!(value(2).ends_with('°'));
        assert_eq!(
            value(5),
            std::format!("{}km", libm::round(sample.physical.look.range_km) as i64)
        );
        // Drawn: the first pill's outline and its value text, label sides inverted.
        let screen = wide_screen(&app);
        assert!(screen.pixels[4][6] && !screen.pixels[4][4] && screen.pixels[6][4]);
        let mut text = Screen {
            pixels: [[false; 400]; 240],
        };
        Text::with_baseline(
            "25544",
            Point::new(9, 6),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
            Baseline::Top,
        )
        .draw(&mut text)
        .unwrap();
        for y in 6..16 {
            for x in 9..39 {
                assert_eq!(screen.pixels[y][x], text.pixels[y][x], "{x},{y}");
            }
        }
        let (el, split) = wide::pill_box(rows[2].as_ref().unwrap(), 0);
        assert!(el.size.width > 30 && split == 4 + 2 * 6 + 7);

        // Nothing selectable in view: counts instead.
        let mut empty = app_with_viewport(Viewport::Rectangle);
        empty.zoom(64);
        if empty.selected_id().is_none() {
            let (rows, _) = wide::pill_rows(&empty);
            assert_eq!(rows[0].as_ref().unwrap().value.as_str(), "NO SEL");
        }
        // No frame: no pills and no ruler, only the notice.
        empty.invalidate_frame();
        assert!(wide::pill_rows(&empty).0.iter().all(Option::is_none));
        let screen = wide_screen(&empty);
        for y in 0..40 {
            assert!(screen.pixels[y][..100].iter().all(|&p| !p));
        }
        for y in 200..240 {
            assert!(screen.pixels[y][300..].iter().all(|&p| !p));
        }
    }

    #[test]
    fn scale_ruler_sits_bottom_right_on_a_cleared_backing() {
        let app = app_with_viewport(Viewport::Rectangle);
        let screen = wide_screen(&app);
        // "km" ends 4 px from the right edge; the bar is the base row + 3.
        let row = &screen.pixels[240 - 11 + 3];
        let first = row.iter().position(|&p| p).unwrap();
        let last = row.iter().rposition(|&p| p).unwrap();
        assert!(first > 250 && last < 390, "{first}..{last}");
        assert!(row[first..=last].iter().all(|&p| p));
        // Cleared backing above the bar between labels.
        assert!(!screen.pixels[240 - 11 - 15][first + 1]);
    }

    #[test]
    fn one_prepared_layer_serves_satellite_motion_and_zoom() {
        let layout = Layout::wide();
        let mut app = app_with_viewport(Viewport::Rectangle);
        let mut layer = CoastLayer::prepare(app.observer());
        for step in 0..4 {
            let later =
                app.time().unwrap() + overhead_core::sgp4::chrono::Duration::seconds(20 * step);
            app.update(later).unwrap();
            app.zoom(if step % 2 == 0 { 20 } else { -35 });
            let mut reused = Screen {
                pixels: [[true; 400]; 240],
            };
            super::draw(&mut reused, &app, layout, false, &mut layer).unwrap();
            let mut fresh = Screen {
                pixels: [[false; 400]; 240],
            };
            draw(&mut fresh, &app, layout, false).unwrap();
            assert_eq!(reused.pixels, fresh.pixels, "step={step}");
        }
    }

    #[test]
    fn preparing_screen_is_full_screen_text_only() {
        let mut screen = Screen {
            pixels: [[true; 400]; 240],
        };
        draw_preparing(&mut screen).unwrap();
        let mut expected = Screen {
            pixels: [[false; 400]; 240],
        };
        Text::with_baseline(
            "PREPARING MAP",
            Point::new(161, 115),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
            Baseline::Top,
        )
        .draw(&mut expected)
        .unwrap();
        // No radar, limb, coast, strip, separator or notices from any scene.
        assert_eq!(screen.pixels, expected.pixels);
    }

    #[test]
    fn updating_screen_is_centred_text_only() {
        for (percent, text, x) in [
            (0u8, "UPDATING 0%", 167),
            (42, "UPDATING 42%", 164),
            (250, "UPDATING 100%", 161),
        ] {
            let mut screen = Screen {
                pixels: [[true; 400]; 240],
            };
            draw_updating(&mut screen, percent).unwrap();
            let mut expected = Screen {
                pixels: [[false; 400]; 240],
            };
            Text::with_baseline(
                text,
                Point::new(x, 115),
                MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
                Baseline::Top,
            )
            .draw(&mut expected)
            .unwrap();
            assert_eq!(screen.pixels, expected.pixels, "{text}");
        }
    }

    #[test]
    fn late_coast_failure_rolls_back_ink_without_losing_foreground() {
        use core::cell::Cell;

        // Count actual On writes to the draw target. The failure is injected
        // only after a coast stroke has been emitted, not merely projected.
        struct TrackingScreen<'a> {
            pixels: [[bool; 400]; 240],
            on_writes: &'a Cell<usize>,
        }
        impl OriginDimensions for TrackingScreen<'_> {
            fn size(&self) -> Size {
                DISPLAY_SIZE
            }
        }
        impl DrawTarget for TrackingScreen<'_> {
            type Color = BinaryColor;
            type Error = core::convert::Infallible;
            fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(
                &mut self,
                pixels: I,
            ) -> Result<(), Self::Error> {
                for Pixel(p, color) in pixels {
                    assert!((0..400).contains(&p.x) && (0..240).contains(&p.y));
                    if color == BinaryColor::On {
                        self.on_writes.set(self.on_writes.get() + 1);
                    }
                    self.pixels[p.y as usize][p.x as usize] = color == BinaryColor::On;
                }
                Ok(())
            }
        }

        let layout = Layout::wide();
        let mut app = app_with_viewport(Viewport::Rectangle);
        for empty in [false, true] {
            if empty {
                let later =
                    app.time().unwrap() + overhead_core::sgp4::chrono::Duration::minutes(45);
                app.update(later).unwrap();
                assert_eq!(app.counts().visible, 0);
            } else {
                assert!(app.selected_id().is_some());
            }
            for warning in [false, true] {
                let on_writes = Cell::new(0);
                let stopped = Cell::new(false);
                let mut actual = TrackingScreen {
                    pixels: [[true; 400]; 240],
                    on_writes: &on_writes,
                };
                let mut layer = CoastLayer::prepare(app.observer());
                // Inject after the whole prepared layer succeeded with real ink.
                draw_frame_with_coast(
                    &mut actual,
                    &app,
                    layout,
                    warning,
                    true,
                    |display, observer, view, layout| {
                        let drawn = layer.draw(display, observer, view, layout)?;
                        stopped.set(drawn && on_writes.get() > 0);
                        Ok(false)
                    },
                )
                .unwrap();
                assert!(stopped.get(), "did not inject after coast ink");

                let mut expected = Screen {
                    pixels: [[false; 400]; 240],
                };
                draw_frame(&mut expected, &app, layout, warning, None).unwrap();
                let counts = app.counts();
                draw_wide_notices(
                    &mut expected,
                    &[
                        wide_empty_status(
                            app.time().is_some(),
                            counts,
                            app.indicators().next().is_some(),
                            warning,
                        ),
                        (counts.failed > 0).then_some("TRACK ERROR"),
                        warning.then_some("INGEST WARN"),
                        Some("COAST ERROR"),
                    ],
                )
                .unwrap();
                assert_eq!(
                    actual.pixels, expected.pixels,
                    "late failure changed foreground: empty={empty} warning={warning}"
                );
            }
        }
    }

    #[test]
    fn labels_are_sanitized_and_truncation_is_visible() {
        let mut line = LineBuffer::new(5);
        write!(line, "AB\n🛰CDEF").unwrap();
        assert_eq!(line.as_str(), "AB??+");
        let layout = Layout::new(240).unwrap();
        assert!(
            layout
                .marker(ViewPoint {
                    east: 0.0,
                    north: 1.0
                })
                .y
                < layout.center().y
        );
        assert!(
            layout
                .marker(ViewPoint {
                    east: 1.0,
                    north: 0.0
                })
                .x
                > layout.center().x
        );
    }
}
