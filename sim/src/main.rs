use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, PrimitiveStyle},
    text::Text,
};
use embedded_graphics_simulator::{
    BinaryColorTheme, OutputSettingsBuilder, SimulatorDisplay, Window,
};

fn main() -> Result<(), core::convert::Infallible> {
    // A framebuffer with the real panel's size and colour depth.
    let mut display = SimulatorDisplay::<BinaryColor>::new(Size::new(400, 240));

    // The radar circle: 240 px across, flush left.
    Circle::new(Point::new(0, 0), 240)
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(&mut display)?;

    // The readout panel's first label.
    Text::new(
        "OVERHEAD",
        Point::new(252, 16),
        MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
    )
    .draw(&mut display)?;

    // Scale 2x and use a grey-LCD palette, close to the Sharp look.
    let settings = OutputSettingsBuilder::new()
        .theme(BinaryColorTheme::LcdWhite)
        .scale(2)
        .build();
    Window::new("Overhead", &settings).show_static(&display);
    Ok(())
}
