//! Opt-in 1-bit RAM-target fixture for the isolated coastline measurement.
//! No display transfer, scheduling or shipping catalogue capacity is implied.
use core::convert::Infallible;
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};
use overhead_app::{RadarApp, projection::View};
use overhead_core::{
    GeodeticPosition, OmmElements,
    catalogue::{CatalogueBuilder, RecordOrigin},
};
use overhead_render::{Layout, measurement};

pub const SAMPLES: usize = 5;
pub const CASES: [(&str, f64, f64, f64); 11] = [
    ("sf-tight", 37.7749, -122.4194, 0.1),
    ("sf-1", 37.7749, -122.4194, 1.0),
    ("sf-5", 37.7749, -122.4194, 5.0),
    ("sf-100", 37.7749, -122.4194, 100.0),
    ("colorado-20", 39.007, -104.883, 20.0),
    ("polar-20", 89.9, 0.0, 20.0),
    ("dateline-5", 0.0, 179.9, 5.0),
    // Same segment-46 solver results as the host preflight; rounded to 6 dp.
    ("limb-20", 1.364900, -156.229044, 20.0),
    ("limb-100", 1.364900, -156.229044, 100.0),
    ("viewport-tight", 1.364900, 119.225793, 0.1),
    // Coordinates for this selected control are computed from the ISS fixture.
    ("iss-ground-20", 0.0, 0.0, 20.0),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Coast,
    FrameOn,
    FrameOff,
    Late,
    LateWarning,
}

pub fn app(name: &str, lat: f64, lon: f64, fov: f64) -> RadarApp {
    // Parsed once per scene, outside the timing/heap/stack measurement.
    let elements: alloc::vec::Vec<OmmElements> =
        serde_json::from_str(include_str!("../../core/tests/fixtures/iss-25544.json"))
            .expect("pinned ISS fixture");
    let mut builder = CatalogueBuilder::new();
    builder.push(
        RecordOrigin {
            group: 0,
            record: 0,
        },
        elements.into_iter().next().unwrap(),
    );
    let catalogue = builder.finish().expect("fixture catalogue");
    let time = catalogue.entries()[0].satellite().epoch();
    let observer = if name == "iss-ground-20" {
        let ecef = catalogue.entries()[0]
            .satellite()
            .state_at(time)
            .unwrap()
            .to_ecef()
            .unwrap();
        let ground = overhead_core::ecef_to_geodetic(ecef).unwrap();
        GeodeticPosition {
            altitude_km: 0.0,
            ..ground
        }
    } else {
        GeodeticPosition {
            latitude_rad: lat.to_radians(),
            longitude_rad: lon.to_radians(),
            altitude_km: 0.0,
        }
    };
    let view = View::rectangular(fov, Layout::wide().aspect_ratio()).unwrap();
    let mut app = RadarApp::new(catalogue, observer, view).unwrap();
    app.update(time).unwrap();
    app
}

/// Storage is owned by the caller, not allocated in a timed draw. Full 12 KB
/// target is a measurement baseline, NOT coast-attributable scratch.
pub struct Screen {
    pub pixels: [u8; 12_000],
    pub writes: u32,
    pub on_writes: u32,
}
impl Screen {
    pub const fn new() -> Self {
        Self {
            pixels: [0; 12_000],
            writes: 0,
            on_writes: 0,
        }
    }
    pub fn checksum(&self) -> u32 {
        self.pixels.iter().fold(0_u32, |hash, &byte| {
            hash.wrapping_mul(31).wrapping_add(u32::from(byte))
        })
    }
    pub fn reset_counts(&mut self) {
        self.writes = 0;
        self.on_writes = 0;
    }
    /// Coast-only has no frame clear; erase previous scene ink outside timing.
    pub fn clear_for_coast(&mut self) {
        self.pixels.fill(0);
        self.reset_counts();
    }
}
impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
}
impl OriginDimensions for Screen {
    fn size(&self) -> Size {
        Size::new(400, 240)
    }
}
impl DrawTarget for Screen {
    type Color = BinaryColor;
    type Error = Infallible;
    fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(
        &mut self,
        pixels: I,
    ) -> Result<(), Self::Error> {
        for Pixel(p, color) in pixels {
            assert!((0..400).contains(&p.x) && (0..240).contains(&p.y));
            let bit = 1 << (p.x as usize % 8);
            let byte = &mut self.pixels[p.y as usize * 50 + p.x as usize / 8];
            if color == BinaryColor::On {
                *byte |= bit;
                self.on_writes += 1;
            } else {
                *byte &= !bit;
            }
            self.writes += 1;
        }
        Ok(())
    }
}

/// This separate entry point lets the coast-off image reference only the
/// coast-free renderer path. Check actual ELF sections before using it as a
/// compiled-size control; the unbuilt image is not proof of dead-code removal.
pub fn run_off(screen: &mut Screen, app: &RadarApp) {
    measurement::frame_off(screen, app, false).unwrap();
}

/// Returns whether the intended path completed; false means late injection
/// did not trigger (for example, no geography ink in this scene).
pub fn run(screen: &mut Screen, app: &RadarApp, mode: Mode) -> bool {
    match mode {
        Mode::Coast => measurement::coast_only(screen, app).unwrap(),
        Mode::FrameOn => measurement::frame_on(screen, app, false).unwrap(),
        Mode::FrameOff => {
            run_off(screen, app);
            true
        }
        Mode::Late => measurement::frame_late_failure(screen, app, false).unwrap(),
        Mode::LateWarning => measurement::frame_late_failure(screen, app, true).unwrap(),
    }
}

/// Untimed byte-for-byte control, with the same selected/warning app truth.
pub fn verify_late(screen: &Screen, reference: &mut Screen, app: &RadarApp, warning: bool) {
    reference.pixels.fill(0);
    reference.reset_counts();
    measurement::frame_failure_control(reference, app, warning).unwrap();
    assert_eq!(
        screen.pixels, reference.pixels,
        "late failure lost foreground or notice pixels"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_cases_are_bounded_and_failure_is_detectable() {
        let (mut selected, mut empty) = (0, 0);
        for (name, lat, lon, fov) in CASES {
            let app = app(name, lat, lon, fov);
            if app.selected_id().is_some() {
                selected += 1;
            } else {
                empty += 1;
            }
            let mut screen = Screen::new();
            assert!(run(&mut screen, &app, Mode::Coast));
            assert!(screen.on_writes > 0, "{name} must draw coast ink");
            assert!(run(&mut screen, &app, Mode::FrameOn));
            assert!(run(&mut screen, &app, Mode::FrameOff));
            let expected = screen.pixels;
            let mut separate = Screen::new();
            run_off(&mut separate, &app);
            assert_eq!(separate.pixels, expected);
        }
        assert!(
            selected > 0 && empty > 0,
            "fixture must exercise selected and empty frames"
        );
        for (name, lat, lon, selected) in [
            ("polar-20", 89.9, 0.0, false),
            ("iss-ground-20", 0.0, 0.0, true),
        ] {
            let app = app(name, lat, lon, 20.0);
            assert_eq!(
                app.selected_id().is_some(),
                selected,
                "{name} control state"
            );
            let mut plain = Screen::new();
            assert!(run(&mut plain, &app, Mode::Late), "{name} post-ink failure");
            let mut reference = Screen::new();
            verify_late(&plain, &mut reference, &app, false);
            let mut warned = Screen::new();
            assert!(
                run(&mut warned, &app, Mode::LateWarning),
                "{name} warned failure"
            );
            verify_late(&warned, &mut reference, &app, true);
            assert_ne!(plain.pixels, warned.pixels, "{name} warning must have ink");
        }
    }
}
