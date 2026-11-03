//! Static visibility icons in the former category slot (0078): an eye
//! (sunlit, dark sky), a filled disc (in Earth's shadow) and a sun (daylight
//! at the observer). Same 23px footprint and centre column (x=376) as the data
//! glyphs, centred 16px below the label baseline like them: (376,33).
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
use overhead_app::visibility::Visibility;

// Bit x is column x, row order is top to bottom, as in the category masks.
const VISIBLE: [u32; 24] = [
    0x000000, 0x000000, 0x000000, 0x000000, 0x000000, 0x007c00, 0x038380, 0x040040, 0x080020,
    0x103810, 0x207c08, 0x40fe04, 0x80fe02, 0x40fe04, 0x207c08, 0x103810, 0x080020, 0x040040,
    0x038380, 0x007c00, 0x000000, 0x000000, 0x000000, 0x000000,
];
const ECLIPSED: [u32; 24] = [
    0x000000, 0x000000, 0x00fe00, 0x03ff80, 0x07ffc0, 0x0fffe0, 0x1ffff0, 0x3ffff8, 0x3ffff8,
    0x7ffffc, 0x7ffffc, 0x7ffffc, 0x7ffffc, 0x7ffffc, 0x7ffffc, 0x7ffffc, 0x3ffff8, 0x3ffff8,
    0x1ffff0, 0x0fffe0, 0x07ffc0, 0x03ff80, 0x00fe00, 0x000000,
];
const DAYLIGHT: [u32; 24] = [
    0x000000, 0x001000, 0x001000, 0x001000, 0x001000, 0x080020, 0x040040, 0x007c00, 0x008200,
    0x010100, 0x020080, 0x020080, 0xf2009e, 0x020080, 0x020080, 0x010100, 0x008200, 0x007c00,
    0x040040, 0x080020, 0x001000, 0x001000, 0x001000, 0x001000,
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
                .then_some(Pixel(Point::new(364 + x, 21 + y as i32), BinaryColor::On))
        })
    }))
}

// Independently expressed geometry about local (12,12), not the packed
// production masks.
#[cfg(test)]
pub(super) fn reference_ink(visibility: Visibility, x: i32, y: i32) -> bool {
    let distance = libm::hypot(f64::from(x - 12), f64::from(y - 12));
    match visibility {
        // Pupil disc and two lids abs(y-12) = round(7(1-((x-12)/11)²)) for
        // x = 1..=23, joined vertically where neighbouring heights jump.
        Visibility::Visible => {
            let lid = |x: i32| libm::round(7.0 * (1.0 - (f64::from(x - 12) / 11.0).powi(2))) as i32;
            if distance <= 3.5 {
                return true;
            }
            if !(1..=23).contains(&x) {
                return false;
            }
            let h = lid(x);
            let low = if x == 1 || x == 23 {
                0
            } else {
                lid(x - 1).min(lid(x + 1))
            };
            let dy = (y - 12).abs();
            dy == h || (low + 1..=h).contains(&dy)
        }
        Visibility::Eclipsed => distance <= 10.5,
        // Ring of radius 5, axis rays 8..=11 and diagonal rays 6..=7.
        Visibility::Daylight => {
            let (dx, dy) = (x - 12, y - 12);
            (4.5..5.5).contains(&distance)
                || ((dx == 0 || dy == 0) && (8..=11).contains(&(dx.abs() + dy.abs())))
                || (dx.abs() == dy.abs() && (6..=7).contains(&dx.abs()))
        }
    }
}
