//! Static 24×24 category icons, exact approved 0045 preview pixels.
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
use overhead_app::category::Category;

// Bit x is column x, row order is top to bottom. Fixed masks avoid dependence
// on line/primitive raster conventions. Masks contain 384 bytes of pixel data;
// this is not a target-placement or whole-device memory measurement.
const NAVIGATION: [u32; 24] = [
    0x000000, 0x000000, 0x000800, 0x000800, 0x000800, 0x000800, 0x000800, 0x000800, 0x000800,
    0x000800, 0x000800, 0x3ffffc, 0x000800, 0x000800, 0x000800, 0x000800, 0x000800, 0x000800,
    0x000800, 0x000800, 0x000800, 0x000800, 0x000000, 0x000000,
];
const STATION: [u32; 24] = [
    0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x007e00,
    0x004200, 0x004200, 0x3fc3fc, 0x004200, 0x004200, 0x004200, 0x007e00, 0x000000, 0x000000,
    0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x000000,
];
const WEATHER: [u32; 24] = [
    0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x000f00, 0x001080, 0x002040, 0x01e040,
    0x020060, 0x040018, 0x1c0008, 0x200004, 0x200004, 0x200004, 0x200004, 0x100008, 0x0ffff0,
    0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x000000,
];
const UNKNOWN: [u32; 24] = [
    0x000000, 0x000000, 0x000000, 0x003c00, 0x004200, 0x008100, 0x008100, 0x008000, 0x008000,
    0x004000, 0x004000, 0x002000, 0x001000, 0x000800, 0x000800, 0x000800, 0x000800, 0x000000,
    0x000000, 0x001800, 0x001800, 0x000000, 0x000000, 0x000000,
];

pub(super) fn draw<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    category: Option<Category>,
) -> Result<(), D::Error> {
    let Some(category) = category else {
        return Ok(());
    };
    Text::new(
        "Cat",
        Point::new(367, 17),
        MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
    )
    .draw(display)?;
    let rows = match category {
        Category::Station => &STATION,
        Category::Navigation => &NAVIGATION,
        Category::Weather => &WEATHER,
        Category::Unknown => &UNKNOWN,
    };
    display.draw_iter(rows.iter().enumerate().flat_map(|(y, row)| {
        (0..24).filter_map(move |x| {
            (row & (1 << x) != 0)
                .then_some(Pixel(Point::new(364 + x, 19 + y as i32), BinaryColor::On))
        })
    }))
}

// Independently expressed screen-local geometry, not the packed production
// masks. Full renderer lifecycle tests use this reference as well.
#[cfg(test)]
pub(super) fn reference_ink(category: Category, x: i32, y: i32) -> bool {
    match category {
        Category::Navigation => {
            (x == 11 && (2..=21).contains(&y)) || (y == 11 && (2..=21).contains(&x))
        }
        Category::Station => {
            ((y == 8 || y == 15) && (9..=14).contains(&x))
                || ((x == 9 || x == 14) && (8..=15).contains(&y))
                || (y == 11 && ((2..=9).contains(&x) || (14..=21).contains(&x)))
        }
        Category::Weather => match y {
            5 => (8..=11).contains(&x),
            6 => x == 7 || x == 12,
            7 => x == 6 || x == 13,
            8 => x == 6 || (13..=16).contains(&x),
            9 => x == 5 || x == 6 || x == 17,
            10 => x == 3 || x == 4 || x == 18,
            11 => x == 3 || (18..=20).contains(&x),
            12..=15 => x == 2 || x == 21,
            16 => x == 3 || x == 20,
            17 => (4..=19).contains(&x),
            _ => false,
        },
        Category::Unknown => match y {
            3 => (10..=13).contains(&x),
            4 => x == 9 || x == 14,
            5..=6 => x == 8 || x == 15,
            7..=8 => x == 15,
            9..=10 => x == 14,
            11 => x == 13,
            12 => x == 12,
            13..=16 => x == 11,
            19..=20 => x == 11 || x == 12,
            _ => false,
        },
    }
}
