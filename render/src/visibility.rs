//! Static 24×24 visibility icons in the former category slot (0078): an eye
//! (sunlit, dark sky), a filled disc (in Earth's shadow) and a sun (daylight
//! at the observer).
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
use overhead_app::visibility::Visibility;

// Bit x is column x, row order is top to bottom, as in the category masks.
const VISIBLE: [u32; 24] = [
    0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x003e00, 0x00c180, 0x030060,
    0x041c10, 0x083e08, 0x103e04, 0x083e08, 0x041c10, 0x030060, 0x00c180, 0x003e00, 0x000000,
    0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x000000,
];
const ECLIPSED: [u32; 24] = [
    0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x003e00, 0x00ff80, 0x01ffc0, 0x01ffc0,
    0x03ffe0, 0x03ffe0, 0x03ffe0, 0x03ffe0, 0x03ffe0, 0x01ffc0, 0x01ffc0, 0x00ff80, 0x003e00,
    0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x000000,
];
const DAYLIGHT: [u32; 24] = [
    0x000000, 0x000000, 0x000800, 0x000800, 0x000800, 0x020020, 0x010040, 0x003e00, 0x006300,
    0x00c180, 0x008080, 0x1c809c, 0x008080, 0x00c180, 0x006300, 0x003e00, 0x010040, 0x020020,
    0x000800, 0x000800, 0x000800, 0x000000, 0x000000, 0x000000,
];

pub(super) fn draw<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    visibility: Option<Visibility>,
) -> Result<(), D::Error> {
    let Some(visibility) = visibility else {
        return Ok(());
    };
    Text::new(
        "Vis",
        Point::new(367, 17),
        MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
    )
    .draw(display)?;
    let rows = match visibility {
        Visibility::Visible => &VISIBLE,
        Visibility::Eclipsed => &ECLIPSED,
        Visibility::Daylight => &DAYLIGHT,
    };
    display.draw_iter(rows.iter().enumerate().flat_map(|(y, row)| {
        (0..24).filter_map(move |x| {
            (row & (1 << x) != 0)
                .then_some(Pixel(Point::new(364 + x, 19 + y as i32), BinaryColor::On))
        })
    }))
}

// Independently expressed geometry, not the packed production masks.
#[cfg(test)]
pub(super) fn reference_ink(visibility: Visibility, x: i32, y: i32) -> bool {
    let distance = libm::hypot(f64::from(x - 11), f64::from(y - 11));
    match visibility {
        // Pupil disc and two lids |y-11| = round(5(1-((x-11)/9)²)) for
        // x = 2..=20, joined vertically where neighbouring heights jump.
        Visibility::Visible => {
            let lid = |x: i32| libm::round(5.0 * (1.0 - (f64::from(x - 11) / 9.0).powi(2))) as i32;
            if distance <= 2.5 {
                return true;
            }
            if !(2..=20).contains(&x) {
                return false;
            }
            let h = lid(x);
            let low = if x == 2 || x == 20 {
                0
            } else {
                lid(x - 1).min(lid(x + 1))
            };
            let dy = (y - 11).abs();
            dy == h || (low + 1..=h).contains(&dy)
        }
        Visibility::Eclipsed => distance <= 6.5,
        // Ring of radius 4, axis rays 7..=9 and diagonal rays 5..=6 from (11,11).
        Visibility::Daylight => {
            let (dx, dy) = (x - 11, y - 11);
            (3.5..4.5).contains(&distance)
                || ((dx == 0 || dy == 0) && (7..=9).contains(&(dx.abs() + dy.abs())))
                || (dx.abs() == dy.abs() && (5..=6).contains(&dx.abs()))
        }
    }
}
