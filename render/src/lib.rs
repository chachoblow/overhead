//! Platform-independent 400×240, 1-bit radar renderer. No framebuffer or clock
//! ownership; all readouts come from the app's true, timestamped frame.
#![no_std]

use core::fmt::{self, Write};
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};
use overhead_app::{
    RadarApp,
    projection::{EarthLimb, EdgeIndicator, ViewPoint, Viewport},
};
use overhead_core::sgp4::chrono::Timelike;

mod glyphs;

pub const DISPLAY_SIZE: Size = Size::new(400, 240);

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
    /// Opt-in 352x240 radar plus the elevation capsule and explicit notices.
    pub fn wide() -> Self {
        Self {
            radar_width: 352,
            radar_height: 240,
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
pub fn draw<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    app: &RadarApp,
    layout: Layout,
    ingestion_warning: bool,
) -> Result<(), D::Error> {
    let mut display = display.clipped(&Rectangle::new(Point::zero(), DISPLAY_SIZE));
    display.clear(BinaryColor::Off)?;
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

    if viewport == Viewport::Rectangle {
        for (_, indicator) in app.indicators() {
            draw_indicator(&mut display, indicator, layout)?;
        }
    }

    let mut selected_marker = None;
    for (entry, _, point) in app.markers() {
        let marker = layout.marker(point);
        let id = entry.elements().norad_id;
        let selected = app.selected_id() == Some(id);
        draw_marker(&mut display, marker, selected, layout, viewport)?;
        if selected {
            selected_marker = Some((id, marker));
        }
    }

    if viewport == Viewport::Rectangle {
        if let Some((id, marker)) = selected_marker.filter(|_| app.selected_label_visible()) {
            draw_selected_label(&mut display, id, marker, layout)?;
        }
        if let Some((_, sample)) = app.selected() {
            glyphs::draw_capsule(&mut display)?;
            glyphs::draw_elevation(&mut display, sample.physical.look.elevation_rad)?;
        }
        return draw_wide_status(&mut display, app, ingestion_warning);
    }

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
            } else if counts.failed > 0 {
                "NO MARKERS / ERRORS"
            } else if counts.above_horizon > 0 {
                "SKY OUTSIDE VIEW"
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

/// Hollow triangle pointing along the true image-plane direction. Intersect
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
    let stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 1);
    for (start, end) in [(tip, a), (a, b), (b, tip)] {
        Line::new(start, end)
            .into_styled(stroke)
            .draw(&mut clipped)?;
    }
    Ok(())
}

/// Familiar names are keyed by identity, never inferred from catalogue names.
fn selected_label(id: u64) -> LineBuffer {
    // A u64 needs at most 20 decimal digits; no five-/six-digit truncation.
    let mut label = LineBuffer::new(20);
    if id == 25544 {
        let _ = label.write_str("ISS");
    } else {
        let _ = write!(label, "{id}");
    }
    label
}

fn selected_label_origin(marker: Point, size: Size, layout: Layout) -> Point {
    let left = layout.center().x - layout.horizontal_radius();
    let right = layout.center().x + layout.horizontal_radius();
    let top = layout.center().y - layout.radius();
    let bottom = layout.center().y + layout.radius();
    let width = size.width as i32;
    let height = size.height as i32;
    // Two blank columns between the diamond's three-pixel extent and text.
    // Even a full u64 label fits on one side in the wide radar.
    let x = if marker.x + 6 + width - 1 <= right {
        marker.x + 6
    } else {
        marker.x - 6 - width + 1
    };
    Point::new(
        x.max(left),
        (marker.y - height / 2).clamp(top, bottom - height + 1),
    )
}

fn draw_selected_label<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    id: u64,
    marker: Point,
    layout: Layout,
) -> Result<(), D::Error> {
    let label = selected_label(id);
    let text = Text::with_baseline(
        label.as_str(),
        Point::zero(),
        MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        Baseline::Top,
    );
    let origin = selected_label_origin(marker, text.bounding_box().size, layout);
    // Transparent ink only: scene overlaps are allowed, with no clearing mask,
    // collision search, leader line or displacement of markers/arrows.
    text.translate(origin).draw(display)?;
    Ok(())
}

/// Condition-driven notices drawn last, above scene graphics (decision 0040).
/// Their backing may cover pixels, never change eligibility, counts or selection.
fn draw_wide_status<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    app: &RadarApp,
    ingestion_warning: bool,
) -> Result<(), D::Error> {
    let counts = app.counts();
    let status = if app.time().is_none() {
        Some("NO FRAME")
    } else if counts.visible == 0 {
        Some("NO MARKERS")
    } else {
        None
    };
    draw_wide_notices(
        display,
        &[
            status,
            (counts.failed > 0).then_some("TRACK ERROR"),
            ingestion_warning.then_some("INGEST WARN"),
        ],
    )
}

fn draw_wide_notices<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    notices: &[Option<&str>; 3],
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

struct LineBuffer {
    bytes: [u8; 40],
    len: usize,
    limit: usize,
}
impl LineBuffer {
    fn new(limit: usize) -> Self {
        Self {
            bytes: [0; 40],
            len: 0,
            limit: limit.min(40),
        }
    }
    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}
impl Write for LineBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for ch in s.chars() {
            if self.len == self.limit {
                if self.limit > 0 {
                    self.bytes[self.limit - 1] = b'+';
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
                    assert_eq!(a.pixels[y][x], b.pixels[y][x + 56], "fov={fov} at {x},{y}");
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
    fn arrows_are_hollow_outward_and_clipped_including_corner_directions() {
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
                assert!(screen.pixels[120][346]); // Outward tip.
                assert!(screen.pixels[118][341]); // Base corners.
                assert!(screen.pixels[122][341]);
                assert!(!screen.pixels[120][343]); // Hollow interior.
            }
            let expected = screen.pixels;
            draw_indicator(&mut screen, arrow, layout).unwrap();
            assert_eq!(screen.pixels, expected); // Exact overlaps are permitted.
        }
    }

    #[test]
    fn capsule_has_bounded_one_pixel_outline_and_only_two_centered_openings() {
        let mut screen = Screen {
            pixels: [[false; 400]; 240],
        };
        glyphs::draw_capsule(&mut screen).unwrap();
        for (y, row) in screen.pixels.iter().enumerate() {
            for (x, &on) in row.iter().enumerate() {
                if on {
                    assert!((355..=396).contains(&x) && (7..=152).contains(&y));
                    assert_eq!(on, screen.pixels[y][751 - x]);
                    assert_eq!(on, screen.pixels[159 - y][x]);
                }
            }
        }
        for y in 17..143 {
            for x in 355..=396 {
                assert_eq!(
                    screen.pixels[y][x],
                    (x == 355 || x == 396) && !(77..=82).contains(&y),
                    "side/interior at {x},{y}"
                );
            }
        }
        for x in 365..387 {
            for y in 7..=152 {
                assert_eq!(screen.pixels[y][x], y == 7 || y == 152);
            }
        }
        // Corners turn inward within the ten-pixel radius, with no square tips.
        assert!(!screen.pixels[7][355]);
        assert!(screen.pixels[7][365]);
        assert!(screen.pixels[17][355]);
        for y in 7..17 {
            assert!(screen.pixels[y][355..365].iter().any(|&on| on));
        }
    }

    fn assert_elevation_pixels(screen: &Screen, elevation: f64) {
        // Independent angle reference, not the renderer's sin/cos inequality.
        for row in 0..24 {
            for col in 0..24 {
                let angle = (23.5 - row as f64).atan2(col as f64 + 0.5);
                assert_eq!(
                    screen.pixels[60 + row][364 + col],
                    elevation.is_finite() && angle <= elevation,
                    "elevation={elevation}, column={col}, row={row}"
                );
            }
        }
    }

    #[test]
    fn elevation_fill_matches_angles_endpoints_and_monotonic_area_without_a_box() {
        let mut previous = [[false; 400]; 240];
        for degrees in 0..=90 {
            let mut screen = Screen {
                pixels: [[false; 400]; 240],
            };
            let elevation = (degrees as f64).to_radians();
            glyphs::draw_elevation(&mut screen, elevation).unwrap();
            assert_elevation_pixels(&screen, elevation);
            let mut count = 0;
            for (y, row) in screen.pixels.iter().enumerate() {
                for (x, &on) in row.iter().enumerate() {
                    assert!(!previous[y][x] || on, "fill decreased at {degrees} degrees");
                    if on {
                        count += 1;
                        assert!((364..388).contains(&x) && (60..84).contains(&y));
                    }
                }
            }
            match degrees {
                0 => assert_eq!(count, 0),
                45 => assert_eq!(count, 300),
                90 => assert_eq!(count, 576),
                _ => {}
            }
            previous = screen.pixels;
        }
        for elevation in [
            -1.0,
            -0.0,
            f64::MAX,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ] {
            let mut screen = Screen {
                pixels: [[false; 400]; 240],
            };
            glyphs::draw_elevation(&mut screen, elevation).unwrap();
            assert_elevation_pixels(&screen, elevation);
            assert_eq!(
                screen.pixels.iter().flatten().filter(|&&on| on).count(),
                if elevation == f64::MAX { 576 } else { 0 }
            );
        }
    }

    #[test]
    fn elevation_follows_true_selection_frames_and_clears_without_selection() {
        let mut app = app_with_orbits(Viewport::Rectangle, false, &[25544, 100530], 2.0);
        assert_eq!(app.counts().visible, 2);
        let mut screen = Screen {
            pixels: [[true; 400]; 240],
        };
        let time = app.time().unwrap();
        let samples: Vec<_> = app.objects().map(|(_, s)| s.clone()).collect();
        let mut previous = None;
        for id in [25544, 100530, 25544] {
            assert_eq!(app.selected_id(), Some(id));
            let elevation = app.selected().unwrap().1.physical.look.elevation_rad;
            draw(&mut screen, &app, Layout::wide(), false).unwrap();
            assert_elevation_pixels(&screen, elevation);
            if let Some(previous) = previous {
                assert_ne!(elevation, previous);
            }
            previous = Some(elevation);
            app.select_next();
        }
        assert_eq!(
            app.objects().map(|(_, s)| s.clone()).collect::<Vec<_>>(),
            samples
        );
        app.update(time + overhead_core::sgp4::chrono::Duration::seconds(30))
            .unwrap();
        draw(&mut screen, &app, Layout::wide(), false).unwrap();
        let elevation = app.selected().unwrap().1.physical.look.elevation_rad;
        assert_ne!(Some(elevation), previous);
        assert_elevation_pixels(&screen, elevation);

        // Tight zoom removes both objects; arrows cannot retain a selected glyph.
        app.zoom(64);
        assert_eq!(app.selected_id(), None);
        draw(&mut screen, &app, Layout::wide(), false).unwrap();
        assert!(
            screen.pixels[..153]
                .iter()
                .all(|row| row[352..].iter().all(|&on| !on))
        );
        app.zoom(-64);
        assert!(app.selected().is_some());
        assert!(!app.selected_label_visible());
        draw(&mut screen, &app, Layout::wide(), false).unwrap();
        assert_elevation_pixels(
            &screen,
            app.selected().unwrap().1.physical.look.elevation_rad,
        );
        assert!(screen.pixels[7][375]); // Capsule is independent of the ID gate.

        let invalid = overhead_core::sgp4::chrono::NaiveDate::from_ymd_opt(2101, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert!(app.update(invalid).is_err());
        draw(&mut screen, &app, Layout::wide(), false).unwrap();
        assert!(
            screen.pixels[..153]
                .iter()
                .all(|row| row[352..].iter().all(|&on| !on))
        );
        app.update(time).unwrap();
        draw(&mut screen, &app, Layout::wide(), false).unwrap();
        assert_elevation_pixels(
            &screen,
            app.selected().unwrap().1.physical.look.elevation_rad,
        );
        assert!(screen.pixels[7][375]);
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
        if let Some((_, sample)) = app.selected() {
            glyphs::draw_capsule(&mut reference).unwrap();
            glyphs::draw_elevation(&mut reference, sample.physical.look.elevation_rad).unwrap();
        }
        for y in 0..240 {
            assert_eq!(screen.pixels[y][352..], reference.pixels[y][352..]);
        }
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
    fn notice_backing_is_opaque_bounded_and_absent_without_notices() {
        for notices in [
            [None, None, None],
            [Some("NO FRAME"), None, None],
            [Some("NO MARKERS"), None, None],
            [None, Some("TRACK ERROR"), Some("INGEST WARN")],
            [Some("NO MARKERS"), Some("TRACK ERROR"), Some("INGEST WARN")],
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
        assert_wide_notice(&healthy, false, &["NO MARKERS"]);
        assert_wide_notice(&failed, true, &["NO MARKERS", "TRACK ERROR", "INGEST WARN"]);
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
    fn selected_identity_is_bare_complete_and_only_25544_is_iss() {
        for (id, expected) in [
            (25544, "ISS"),
            (38771, "38771"),
            (100530, "100530"),
            (1_234_567, "1234567"),
            (0, "0"),
            (u64::MAX, "18446744073709551615"),
        ] {
            assert_eq!(selected_label(id).as_str(), expected);
        }
    }

    #[test]
    fn selected_label_fits_edges_corners_and_full_id_without_clipping() {
        let layout = Layout::wide();
        for id in [25544, 38771, 100530, 1_234_567, u64::MAX] {
            let label = selected_label(id);
            let text = Text::with_baseline(
                label.as_str(),
                Point::zero(),
                MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
                Baseline::Top,
            );
            let size = text.bounding_box().size;
            // Every integer x tests both sides of the flip threshold; y includes
            // all edges/corners and the vertically centered interior.
            for x in 3..=349 {
                for y in [3, 7, 120, 233, 237] {
                    let marker = Point::new(x, y);
                    let origin = selected_label_origin(marker, size, layout);
                    let bounds = Rectangle::new(origin, size);
                    assert!(layout.contains(origin, Viewport::Rectangle));
                    assert!(layout.contains(bounds.bottom_right().unwrap(), Viewport::Rectangle));
                    if x + 6 + size.width as i32 - 1 <= 349 {
                        assert_eq!(origin.x, x + 6);
                    } else {
                        assert_eq!(bounds.bottom_right().unwrap().x, x - 6);
                    }
                    assert_eq!(origin.y, (y - 5).clamp(3, 228));
                    // Compare with un-clipped reference text to detect lost
                    // characters, not merely out-of-bounds pixels.
                    let mut actual = Screen {
                        pixels: [[false; 400]; 240],
                    };
                    draw_selected_label(&mut actual, id, marker, layout).unwrap();
                    let mut expected = Screen {
                        pixels: [[false; 400]; 240],
                    };
                    text.translate(origin).draw(&mut expected).unwrap();
                    assert_eq!(actual.pixels, expected.pixels);
                }
            }
        }
    }

    #[test]
    fn selected_label_is_transparent_over_scene_graphics() {
        let layout = Layout::wide();
        let mut scene = Screen {
            pixels: [[false; 400]; 240],
        };
        // Put an arrow and another marker directly under the label footprint.
        let marker = Point::new(300, 120);
        draw_marker(
            &mut scene,
            Point::new(312, 120),
            false,
            layout,
            Viewport::Rectangle,
        )
        .unwrap();
        let arrow = EdgeIndicator {
            edge: ViewPoint {
                east: 1.0,
                north: 0.0,
            },
            direction: ViewPoint {
                east: 1.0,
                north: 0.0,
            },
        };
        draw_indicator(&mut scene, arrow, layout).unwrap();
        let before = scene.pixels;
        let mut ink = Screen {
            pixels: [[false; 400]; 240],
        };
        draw_selected_label(&mut ink, 1_234_567, marker, layout).unwrap();
        draw_selected_label(&mut scene, 1_234_567, marker, layout).unwrap();
        for (y, row) in before.iter().enumerate() {
            for (x, &was_on) in row.iter().enumerate() {
                assert_eq!(scene.pixels[y][x], was_on || ink.pixels[y][x]);
            }
        }
    }

    #[test]
    fn wide_label_follows_selection_and_clears_on_reselection_and_missing_frame() {
        let layout = Layout::wide();
        let mut app = app_with_ids(Viewport::Rectangle, false, &[25544, 100530]);
        let samples: Vec<_> = app.objects().map(|(_, s)| s.clone()).collect();
        let counts = app.counts();
        let mut screen = Screen {
            pixels: [[true; 400]; 240],
        };
        draw(&mut screen, &app, layout, false).unwrap();
        let iss = screen.pixels;
        app.select_next();
        assert_eq!(app.selected_id(), Some(100530));
        draw(&mut screen, &app, layout, false).unwrap();
        assert_ne!(screen.pixels, iss);
        let mut label_reference = Screen {
            pixels: [[false; 400]; 240],
        };
        Text::with_baseline(
            "100530",
            Point::new(182, 115),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
            Baseline::Top,
        )
        .draw(&mut label_reference)
        .unwrap();
        for (y, row) in screen.pixels.iter().enumerate() {
            for (x, &pixel) in row.iter().enumerate() {
                if (182..218).contains(&x) && (115..125).contains(&y) {
                    assert_eq!(pixel, label_reference.pixels[y][x]);
                } else {
                    assert_eq!(pixel, iss[y][x]); // Only the selected label changes.
                }
            }
        }
        let mut fresh = Screen {
            pixels: [[false; 400]; 240],
        };
        draw(&mut fresh, &app, layout, false).unwrap();
        assert_eq!(screen.pixels, fresh.pixels);
        assert_eq!(app.counts(), counts);
        assert_eq!(
            app.objects().map(|(_, s)| s.clone()).collect::<Vec<_>>(),
            samples
        );
        app.select_next();
        draw(&mut screen, &app, layout, false).unwrap();
        assert_eq!(screen.pixels, iss);

        let invalid = overhead_core::sgp4::chrono::NaiveDate::from_ymd_opt(2101, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert!(app.update(invalid).is_err());
        draw(&mut screen, &app, layout, false).unwrap();
        draw(&mut fresh, &app, layout, false).unwrap();
        assert_eq!(screen.pixels, fresh.pixels);
        assert_eq!(app.selected_id(), None);
        let label_origin = selected_label_origin(layout.center(), Size::new(18, 10), layout);
        assert!(
            screen.pixels[label_origin.y as usize..(label_origin.y + 10) as usize]
                .iter()
                .all(
                    |row| row[label_origin.x as usize..(label_origin.x + 36) as usize]
                        .iter()
                        .all(|&on| !on)
                )
        );
    }

    #[test]
    fn zooming_selected_marker_offscreen_removes_label_without_labelling_arrow() {
        let layout = Layout::wide();
        let mut app = app_with_viewport(Viewport::Rectangle);
        let later = app.time().unwrap() + overhead_core::sgp4::chrono::Duration::seconds(30);
        app.update(later).unwrap();
        assert_eq!(app.selected_id(), Some(25544));
        let mut screen = Screen {
            pixels: [[false; 400]; 240],
        };
        draw(&mut screen, &app, layout, false).unwrap();
        let before = screen.pixels;
        app.zoom(64);
        assert_eq!(app.selected_id(), None);
        assert_eq!(app.counts().visible, 0);
        assert_eq!(app.indicators().count(), 1);
        draw(&mut screen, &app, layout, false).unwrap();
        assert_ne!(screen.pixels, before);
        let mut fresh = Screen {
            pixels: [[false; 400]; 240],
        };
        draw(&mut fresh, &app, layout, false).unwrap();
        assert_eq!(screen.pixels, fresh.pixels);
        app.zoom(-64);
        assert_eq!(app.selected_id(), Some(25544));
        draw(&mut screen, &app, layout, false).unwrap();
        draw(&mut fresh, &app, layout, false).unwrap();
        assert_eq!(screen.pixels, fresh.pixels);
    }

    #[test]
    fn hiding_label_changes_only_text_not_diamond_scene_or_diagnostics() {
        use overhead_app::identification::LabelZoom;
        for viewport in [
            Viewport::Rectangle,
            Viewport::Square,
            Viewport::Circle,
            Viewport::BorderlessCircle,
        ] {
            let layout = if viewport == Viewport::Rectangle {
                Layout::wide()
            } else {
                Layout::new(240).unwrap()
            };
            let app = app_with_failure(viewport, true);
            let selected = app.selected_id();
            let physical = app.selected().unwrap().1.physical;
            let counts = app.counts();
            let mut screen = Screen {
                pixels: [[false; 400]; 240],
            };
            draw(&mut screen, &app, layout, true).unwrap();
            let labelled = screen.pixels;
            // Same lens, frame, markers, warnings and selection; only the label
            // gate changes. This also verifies constructor configuration.
            let app = app.with_label_zoom(LabelZoom::new(10.0, 15.0).unwrap());
            assert!(!app.selected_label_visible());
            draw(&mut screen, &app, layout, true).unwrap();
            assert_eq!(app.selected_id(), selected);
            assert_eq!(app.selected().unwrap().1.physical, physical);
            assert_eq!(app.counts(), counts);
            let marker = layout.marker(app.markers().next().unwrap().2);
            let mut diamond = Screen {
                pixels: [[false; 400]; 240],
            };
            draw_marker(&mut diamond, marker, true, layout, viewport).unwrap();
            for (y, row) in diamond.pixels.iter().enumerate() {
                for (x, &on) in row.iter().enumerate() {
                    if on {
                        assert!(screen.pixels[y][x]);
                    }
                }
            }
            if viewport == Viewport::Rectangle {
                assert_ne!(screen.pixels, labelled);
                let mut fresh = Screen {
                    pixels: [[false; 400]; 240],
                };
                draw(&mut fresh, &app, layout, true).unwrap();
                assert_eq!(screen.pixels, fresh.pixels); // No stale text.
                draw_selected_label(&mut screen, selected.unwrap(), marker, layout).unwrap();
            }
            assert_eq!(screen.pixels, labelled);
        }
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
