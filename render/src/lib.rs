//! Platform-independent 400×240, 1-bit radar renderer. No framebuffer or clock
//! ownership; all readouts come from the app's true, timestamped frame.
#![no_std]

use core::fmt::{self, Write};
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, Rectangle},
    text::Text,
};
use overhead_app::{
    RadarApp,
    projection::{EarthLimb, ViewPoint, Viewport},
};
use overhead_core::sgp4::chrono::Timelike;

pub const DISPLAY_SIZE: Size = Size::new(400, 240);

#[derive(Debug, Clone, Copy)]
pub struct Layout {
    radar_size: u32,
}

impl Layout {
    /// Provisional visual-comparison knob. Circle stays vertically centered;
    /// smaller values trade radar area for a wider information area.
    pub fn new(radar_size: u32) -> Result<Self, &'static str> {
        if !(180..=240).contains(&radar_size) {
            return Err("radar size must be in [180, 240] pixels");
        }
        Ok(Self { radar_size })
    }
    fn center(self) -> Point {
        Point::new(self.radar_size as i32 / 2, 120)
    }
    fn radius(self) -> i32 {
        self.radar_size as i32 / 2 - 3
    }
    fn contains(self, pixel: Point, viewport: Viewport) -> bool {
        let delta = pixel - self.center();
        let radius = self.radius();
        match viewport {
            Viewport::Circle | Viewport::BorderlessCircle => {
                delta.x * delta.x + delta.y * delta.y <= radius * radius
            }
            Viewport::Square => delta.x.abs() <= radius && delta.y.abs() <= radius,
        }
    }
    fn marker(self, p: ViewPoint) -> Point {
        let round = |v: f64| (v + if v < 0.0 { -0.5 } else { 0.5 }) as i32;
        self.center()
            + Point::new(
                round(p.east * f64::from(self.radius())),
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

    for (entry, _, point) in app.markers() {
        let marker = layout.marker(point);
        let selected = app.selected_id() == Some(entry.elements().norad_id);
        draw_marker(&mut display, marker, selected, layout, viewport)?;
    }

    let x = layout.radar_size as i32 + 10;
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
    // Sample both rows and columns for a continuous one-bit ellipse. Work is
    // bounded by viewport diameter, not the zoomed ellipse's enormous extent.
    // All clipping happens before float-to-pixel conversion, then again after
    // rounding so neither the outline nor a glyph can leak out of the radar.
    let pixels = (-radius..=radius)
        .flat_map(|offset| {
            let coordinate = f64::from(offset) / f64::from(radius);
            let rows = limb
                .east_at(coordinate)
                .into_iter()
                .flatten()
                .map(move |east| ViewPoint {
                    east,
                    north: coordinate,
                });
            let columns = limb
                .north_at(coordinate)
                .into_iter()
                .flatten()
                .map(move |north| ViewPoint {
                    east: coordinate,
                    north,
                });
            rows.chain(columns)
        })
        .filter_map(|point| {
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
            View::new(20.0).unwrap().with_viewport(viewport),
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
