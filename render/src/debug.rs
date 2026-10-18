//! 0047's radar-free, true-state tracking/data-health page. No I/O, clock or
//! presentation history; bounded ASCII buffers keep every field in its cells.
use core::fmt::{self, Write};

use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::Rectangle,
    text::Text,
};
use overhead_app::RadarApp;
use overhead_core::sgp4::chrono::NaiveDateTime;

use crate::DISPLAY_SIZE;

#[cfg(test)]
#[path = "debug_tests.rs"]
mod tests;

/// Draw the complete debug page without changing the app's wide geometry or
/// selection. The caller owns access policy and the separate normal renderer.
pub fn draw<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    app: &RadarApp,
    ingestion_warning: bool,
) -> Result<(), D::Error> {
    Page::from_app(app, ingestion_warning).draw(display)
}

struct Page {
    rows: [TextBuffer<64>; 18],
}

impl Page {
    fn from_app(app: &RadarApp, ingestion_warning: bool) -> Self {
        let mut page = Self {
            rows: [TextBuffer::new(); 18],
        };
        page.line(0, format_args!("DEBUG / LOCAL INPUT / NO FETCH"));
        if let Some(time) = app.time() {
            page.line(
                1,
                format_args!("REPLAY UTC {}T{}Z", time.date(), time.time()),
            );
        } else {
            page.line(1, format_args!("REPLAY UTC NO FRAME"));
        }
        page.line(
            2,
            format_args!("VERTICAL FOV {:.2} DEG", app.view().fov_deg()),
        );
        let counts = app.counts();
        page.pair(3, 0, format_args!("CATALOGUE {}", counts.catalogue));
        if app.time().is_some() {
            page.pair(3, 1, format_args!("SKY >0 DEG {}", counts.above_horizon));
            page.pair(4, 0, format_args!("IN VIEW {}", counts.visible));
            page.pair(4, 1, format_args!("FAILED {}", counts.failed));
            if let Some((entry, error)) = app
                .objects()
                .find_map(|(entry, result)| result.as_ref().err().map(|error| (entry, error)))
            {
                page.failure(entry.elements().norad_id, format_args!("{error}"));
            } else {
                page.line(5, format_args!("TRACKING / NO FAILURES"));
            }
        } else {
            page.pair(3, 1, format_args!("SKY >0 DEG --"));
            page.pair(4, 0, format_args!("IN VIEW --"));
            page.pair(4, 1, format_args!("FAILED --"));
            page.line(5, format_args!("TRACKING NOT SAMPLED / NO VALID FRAME"));
        }
        page.line(
            8,
            format_args!(
                "{}",
                if ingestion_warning {
                    "INGEST WARN / DETAILS: HOST STDERR"
                } else {
                    "INGEST / NO WARNINGS"
                }
            ),
        );
        if let Some((entry, sample)) = app.selected().filter(|_| app.time().is_some()) {
            page.line(
                9,
                format_args!("SELECTED NORAD {}", entry.elements().norad_id),
            );
            page.name(entry.elements().object_name.as_deref());
            let truth = sample.physical;
            page.selected_values([
                Some(truth.geodetic.altitude_km),
                Some(truth.look.range_km),
                Some(truth.look.elevation_rad.to_degrees()),
                Some(truth.teme_speed_km_s),
                app.selected_period_minutes(),
                app.time()
                    .map(|time| epoch_hours(time, entry.satellite().epoch())),
            ]);
            page.line(
                15,
                format_args!("EPOCH: NOT FRESHNESS; PERIOD: NOT PASS DURATION"),
            );
        } else {
            page.line(9, format_args!("NO SELECTED OBJECT"));
        }
        page.line(17, format_args!("V RADAR  UP/DN/WHEEL ZOOM  TAB SELECT"));
        page
    }

    fn selected_values(&mut self, values: [Option<f64>; 6]) {
        let formats = [
            ("ALT WGS84", 1, "KM", false),
            ("RANGE", 1, "KM", false),
            ("ELEV", 1, "DEG", false),
            ("TEME SPEED", 3, "KM/S", false),
            ("NOM PERIOD", 1, "MIN", false),
            ("EPOCH OFFSET", 2, "H", true),
        ];
        for (i, (value, (label, precision, unit, signed))) in
            values.into_iter().zip(formats).enumerate()
        {
            // Nominal period alone also requires a positive value.
            let value = value.filter(|v| i != 4 || *v > 0.0);
            let field = numeric(label, value, precision, unit, signed);
            self.pair(12 + i / 2, i % 2, format_args!("{}", field.as_str()));
        }
    }

    fn line(&mut self, row: usize, args: fmt::Arguments<'_>) {
        let mut text = TextBuffer::new();
        let _ = text.write_fmt(args);
        // All fixed labels, IDs and time formats have a known maximum width.
        debug_assert!(!text.shortened);
        self.rows[row] = text;
    }

    fn pair(&mut self, row: usize, column: usize, args: fmt::Arguments<'_>) {
        let mut text = TextBuffer::<32>::new();
        let _ = text.write_fmt(args);
        debug_assert!(!text.shortened);
        let line = &mut self.rows[row];
        while line.len < column * 32 {
            let _ = line.write_char(' ');
        }
        line.len = column * 32;
        let _ = line.write_str(text.as_str());
    }

    fn failure(&mut self, id: u64, cause: fmt::Arguments<'_>) {
        self.line(5, format_args!("TRACK ERROR / FIRST NORAD {id}"));
        let mut text = TextBuffer::<64>::new();
        let _ = write!(text, "CAUSE {cause}");
        if text.shortened {
            text.suffix("...");
        }
        self.line(
            7,
            format_args!(
                "{}",
                if text.changed {
                    "CAUSE SHORTENED/CHANGED; FULL DETAILS: HOST STDERR"
                } else if text.shortened {
                    "CAUSE CLIPPED; FULL CAUSES / OTHER IDS: HOST STDERR"
                } else {
                    "FULL CAUSES / OTHER IDS: HOST STDERR"
                }
            ),
        );
        self.rows[6] = text;
    }

    fn name(&mut self, name: Option<&str>) {
        let name = name.unwrap_or("");
        if name.is_empty() {
            self.line(10, format_args!("NAME UNNAMED"));
            return;
        }
        let (first, rest) = name_line(name);
        self.line(10, format_args!("NAME {}", first.as_str()));
        if !rest.is_empty() {
            let (mut second, rest) = name_line(rest);
            if !rest.is_empty() {
                second.suffix("... [MORE: HOST STDERR]");
            }
            self.line(11, format_args!("     {}", second.as_str()));
        }
        // Check even the omitted tail: a full original is available in the host
        // catalogue report, and control characters can never inject page rows.
        if name.chars().any(|ch| ascii(ch).1) {
            self.line(
                16,
                format_args!("NAME '?' REPLACEMENTS; ORIGINAL: HOST STDERR"),
            );
        }
    }

    fn draw<D: DrawTarget<Color = BinaryColor>>(&self, display: &mut D) -> Result<(), D::Error> {
        let mut display = display.clipped(&Rectangle::new(Point::zero(), DISPLAY_SIZE));
        display.clear(BinaryColor::Off)?;
        for (row, text) in self.rows.iter().enumerate() {
            Text::new(
                text.as_str(),
                Point::new(8, 19 + 12 * row as i32),
                MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
            )
            .draw(&mut display)?;
        }
        Ok(())
    }
}

fn epoch_hours(time: NaiveDateTime, epoch: NaiveDateTime) -> f64 {
    let delta = time - epoch;
    // Preserve signed subsecond differences, including those before the epoch.
    (delta.num_seconds() as f64 + f64::from(delta.subsec_nanos()) * 1e-9) / 3600.0
}

fn numeric(
    label: &str,
    value: Option<f64>,
    precision: usize,
    unit: &str,
    signed: bool,
) -> TextBuffer<32> {
    let mut text = TextBuffer::new();
    if let Some(value) = value.filter(|v| v.is_finite()) {
        if signed {
            let _ = write!(text, "{label} {value:+.precision$} {unit}");
        } else {
            let _ = write!(text, "{label} {value:.precision$} {unit}");
        }
        if text.shortened {
            text = TextBuffer::new();
            if signed {
                let _ = write!(text, "{label} {value:+.2e} {unit}");
            } else {
                let _ = write!(text, "{label} {value:.2e} {unit}");
            }
        }
    } else {
        let _ = write!(text, "{label} UNAVAILABLE");
    }
    debug_assert!(!text.shortened);
    text
}

/// Consume at most 59 sanitized cells, wrapping at an ordinary space when
/// possible. A token longer than a row is hard-wrapped at a character boundary.
fn name_line(name: &str) -> (TextBuffer<59>, &str) {
    let mut end = name.char_indices().nth(59).map_or(name.len(), |(i, _)| i);
    if end < name.len()
        && !name[end..].starts_with(' ')
        && let Some(space) = name[..end].rfind(' ').filter(|&i| i > 0)
    {
        end = space;
    }
    let mut text = TextBuffer::new();
    let _ = text.write_str(name[..end].trim_end_matches(' '));
    (text, name[end..].trim_start_matches(' '))
}

fn ascii(ch: char) -> (u8, bool) {
    if ch.is_ascii() && !ch.is_ascii_control() {
        (ch as u8, false)
    } else {
        (b'?', true)
    }
}

#[derive(Clone, Copy)]
struct TextBuffer<const N: usize> {
    bytes: [u8; N],
    len: usize,
    shortened: bool,
    changed: bool,
}

impl<const N: usize> TextBuffer<N> {
    const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
            shortened: false,
            changed: false,
        }
    }

    fn as_str(&self) -> &str {
        // Every stored character is printable ASCII.
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }

    fn suffix(&mut self, suffix: &str) {
        self.len = self.len.min(N - suffix.len());
        let _ = self.write_str(suffix);
    }
}

impl<const N: usize> Write for TextBuffer<N> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for ch in text.chars() {
            let (byte, changed) = ascii(ch);
            self.changed |= changed;
            if self.len < N {
                self.bytes[self.len] = byte;
                self.len += 1;
            } else {
                self.shortened = true;
            }
        }
        Ok(())
    }
}
