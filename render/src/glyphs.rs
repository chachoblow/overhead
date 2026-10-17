//! The bounded, static elevation-only strip (decision 0037).
//! No presentation history or physical-state mutation belongs here.

use core::f64::consts::{FRAC_PI_2, FRAC_PI_4};
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle, RoundedRectangle, StrokeAlignment},
};

pub(super) fn draw_capsule<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
) -> Result<(), D::Error> {
    let capsule = RoundedRectangle::with_equal_corners(
        Rectangle::new(Point::new(355, 7), Size::new(42, 146)),
        Size::new(10, 10),
    )
    .into_styled(
        PrimitiveStyleBuilder::new()
            .stroke_color(BinaryColor::On)
            .stroke_width(1)
            .stroke_alignment(StrokeAlignment::Inside)
            .build(),
    );
    // Remove ink rather than painting white: each long side has one centered
    // six-pixel opening. The caller clears the complete frame before drawing.
    display.draw_iter(
        capsule
            .pixels()
            .filter(|Pixel(p, _)| !((p.x == 355 || p.x == 396) && (77..=82).contains(&p.y))),
    )
}

pub(super) fn draw_elevation<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    elevation_rad: f64,
) -> Result<(), D::Error> {
    // Do not turn corrupt/absent values into a plausible instrument reading.
    // Finite values saturate at the physical endpoints without changing state.
    if !elevation_rad.is_finite() || elevation_rad <= 0.0 {
        return Ok(());
    }
    let (sin, cos) = if elevation_rad >= FRAC_PI_2 {
        (1.0, 0.0)
    } else if elevation_rad == FRAC_PI_4 {
        // Preserve the included diagonal exactly despite sin/cos rounding.
        (1.0, 1.0)
    } else {
        libm::sincos(elevation_rad)
    };
    display.draw_iter((0..24).flat_map(|row| {
        (0..24).filter_map(move |column| {
            let u = f64::from(column) + 0.5;
            let v = 23.5 - f64::from(row);
            (v * cos <= u * sin)
                .then_some(Pixel(Point::new(364 + column, 60 + row), BinaryColor::On))
        })
    }))
}
