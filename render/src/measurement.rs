//! Opt-in renderer measurement entry points. Not a production failure policy or
//! a physical-display driver. Deliberately shares the normal frame/coast paths.
use core::cell::Cell;
use embedded_graphics::{
    draw_target::DrawTarget, geometry::OriginDimensions, pixelcolor::BinaryColor, prelude::*,
};
use overhead_app::RadarApp;

use crate::{DISPLAY_SIZE, Layout, coast, draw_frame, draw_frame_with_coast};

pub fn coast_only<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    app: &RadarApp,
) -> Result<bool, D::Error> {
    coast::draw(
        display,
        app.observer(),
        app.view(),
        Layout::wide(),
        coast::ASSET,
    )
}

/// The off control uses the same frame path but excludes coast before drawing.
pub fn frame_off<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    app: &RadarApp,
    warning: bool,
) -> Result<(), D::Error> {
    draw_frame(display, app, Layout::wide(), warning, None, None)
}

pub fn frame_on<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    app: &RadarApp,
    warning: bool,
) -> Result<(), D::Error> {
    draw_frame(
        display,
        app,
        Layout::wide(),
        warning,
        None,
        Some(coast::ASSET),
    )
}

/// Trigger after at least one actual coast On write and after at least 20,000
/// interpolation intervals. Returns false if this scene emitted no ink. This
/// seam belongs only to explicit measurement builds, never to normal drawing.
pub fn frame_late_failure<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    app: &RadarApp,
    warning: bool,
) -> Result<bool, D::Error> {
    let fired = Cell::new(false);
    draw_frame_with_coast(
        display,
        app,
        Layout::wide(),
        warning,
        None,
        Some(coast::ASSET),
        |display, observer, view, layout, asset| {
            let ink = Cell::new(false);
            let mut target = InkTarget { display, ink: &ink };
            let mut intervals = 0;
            coast::draw_with_hook(&mut target, observer, view, layout, asset, || {
                intervals += 1;
                let stop = intervals >= 20_000 && ink.get();
                fired.set(stop);
                stop
            })
        },
    )?;
    Ok(fired.get())
}

struct InkTarget<'a, D> {
    display: &'a mut D,
    ink: &'a Cell<bool>,
}

impl<D: DrawTarget<Color = BinaryColor>> OriginDimensions for InkTarget<'_, D> {
    fn size(&self) -> Size {
        DISPLAY_SIZE
    }
}

impl<D: DrawTarget<Color = BinaryColor>> DrawTarget for InkTarget<'_, D> {
    type Color = BinaryColor;
    type Error = D::Error;
    fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(
        &mut self,
        pixels: I,
    ) -> Result<(), Self::Error> {
        self.display.draw_iter(pixels.into_iter().inspect(|pixel| {
            if pixel.1 == BinaryColor::On {
                self.ink.set(true);
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::convert::Infallible;
    use overhead_app::projection::View;
    use overhead_core::{
        GeodeticPosition, OmmElements,
        catalogue::{CatalogueBuilder, RecordOrigin},
    };
    extern crate std;
    use std::vec::Vec;

    struct Screen([[bool; 400]; 240]);
    impl OriginDimensions for Screen {
        fn size(&self) -> Size {
            DISPLAY_SIZE
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
                self.0[p.y as usize][p.x as usize] = color == BinaryColor::On;
            }
            Ok(())
        }
    }

    #[test]
    fn measured_late_failure_repaints_exact_foreground() {
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
        let time = catalogue.entries()[0].satellite().epoch();
        let site = GeodeticPosition {
            latitude_rad: 89.9_f64.to_radians(),
            longitude_rad: 0.0,
            altitude_km: 0.0,
        };
        let view = View::rectangular(20.0, Layout::wide().aspect_ratio()).unwrap();
        let mut app = RadarApp::new(catalogue, site, view).unwrap();
        app.update(time).unwrap();
        let mut actual = Screen([[false; 400]; 240]);
        assert!(frame_late_failure(&mut actual, &app, true).unwrap());
        let mut expected = Screen([[false; 400]; 240]);
        draw_frame_with_coast(
            &mut expected,
            &app,
            Layout::wide(),
            true,
            None,
            Some(coast::ASSET),
            |_, _, _, _, _| Ok(false),
        )
        .unwrap();
        assert_eq!(actual.0, expected.0);
    }
}
