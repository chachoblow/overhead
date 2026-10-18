//! Tests for the independent debug renderer. Synthetic faults/text are controls,
//! not new orbital fixtures or evidence of real catalogue failures.
extern crate std;
use super::*;
use overhead_app::projection::View;
use overhead_core::{
    GeodeticPosition, OmmElements,
    catalogue::{CatalogueBuilder, RecordOrigin},
    ecef_to_geodetic, ecef_to_look_angles,
    sgp4::chrono::{NaiveDate, TimeDelta},
};
use std::{format, string::ToString, vec, vec::Vec};

fn app(failures: usize, sampled: bool) -> RadarApp {
    let mut data: serde_json::Value =
        serde_json::from_str(include_str!("../../core/tests/fixtures/iss-25544.json")).unwrap();
    let mut builder = CatalogueBuilder::new();
    for record in 0..=failures {
        if record > 0 {
            data[0]["NORAD_CAT_ID"] = serde_json::json!(100000 - record as u64);
            data[0]["MEAN_MOTION"] = serde_json::json!(1e300);
        }
        let elements: Vec<OmmElements> = serde_json::from_value(data.clone()).unwrap();
        builder.push(
            RecordOrigin { group: 0, record },
            elements.into_iter().next().unwrap(),
        );
    }
    let catalogue = builder.finish().unwrap();
    let sat = catalogue.entries()[0].satellite();
    let time = sat.epoch();
    let subpoint = ecef_to_geodetic(sat.state_at(time).unwrap().to_ecef().unwrap()).unwrap();
    let mut app = RadarApp::new(
        catalogue,
        GeodeticPosition {
            altitude_km: 0.0,
            ..subpoint
        },
        View::rectangular(20.0, crate::Layout::wide().aspect_ratio()).unwrap(),
    )
    .unwrap();
    if sampled {
        app.update(time).unwrap();
    }
    app
}

struct Screen(Vec<BinaryColor>);
impl Screen {
    fn new(ink: bool) -> Self {
        Self(vec![
            if ink {
                BinaryColor::On
            } else {
                BinaryColor::Off
            };
            400 * 240
        ])
    }
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
        for Pixel(point, color) in pixels {
            assert!((0..400).contains(&point.x) && (0..240).contains(&point.y));
            self.0[point.y as usize * 400 + point.x as usize] = color;
        }
        Ok(())
    }
}
fn row(page: &Page, index: usize) -> &str {
    page.rows[index].as_str()
}
fn left(page: &Page, index: usize) -> &str {
    row(page, index)[..32].trim_end()
}
fn right(page: &Page, index: usize) -> &str {
    &row(page, index)[32..]
}

#[test]
fn healthy_rows_use_independently_propagated_true_values_and_fractional_utc() {
    let app = app(0, true);
    let page = Page::from_app(&app, false);
    let (entry, _) = app.selected().unwrap();
    let time = app.time().unwrap();
    let state = entry.satellite().state_at(time).unwrap();
    let ecef = state.to_ecef().unwrap();
    let geo = ecef_to_geodetic(ecef).unwrap();
    let look = ecef_to_look_angles(ecef, app.observer()).unwrap();
    let speed = state.velocity().iter().map(|v| v * v).sum::<f64>().sqrt();
    assert_eq!(row(&page, 0), "DEBUG / LOCAL INPUT / NO FETCH");
    assert_eq!(row(&page, 1), "REPLAY UTC 2026-10-04T12:43:41.833056Z");
    assert_eq!(row(&page, 2), "VERTICAL FOV 20.00 DEG");
    assert_eq!(left(&page, 3), "CATALOGUE 1");
    assert_eq!(right(&page, 3), "SKY >0 DEG 1");
    assert_eq!(left(&page, 4), "IN VIEW 1");
    assert_eq!(right(&page, 4), "FAILED 0");
    assert_eq!(row(&page, 5), "TRACKING / NO FAILURES");
    assert_eq!(row(&page, 6), "");
    assert_eq!(row(&page, 7), "");
    assert_eq!(row(&page, 8), "INGEST / NO WARNINGS");
    assert_eq!(row(&page, 9), "SELECTED NORAD 25544");
    assert_eq!(
        row(&page, 10),
        format!("NAME {}", entry.elements().object_name.as_ref().unwrap())
    );
    assert_eq!(row(&page, 11), "");
    assert_eq!(
        left(&page, 12),
        format!("ALT WGS84 {:.1} KM", geo.altitude_km)
    );
    assert_eq!(right(&page, 12), format!("RANGE {:.1} KM", look.range_km));
    assert_eq!(
        left(&page, 13),
        format!("ELEV {:.1} DEG", look.elevation_rad.to_degrees())
    );
    assert_eq!(right(&page, 13), format!("TEME SPEED {speed:.3} KM/S"));
    assert_eq!(
        left(&page, 14),
        format!(
            "NOM PERIOD {:.1} MIN",
            1440.0 / entry.elements().mean_motion
        )
    );
    assert_eq!(right(&page, 14), "EPOCH OFFSET +0.00 H");
    assert_eq!(
        row(&page, 15),
        "EPOCH: NOT FRESHNESS; PERIOD: NOT PASS DURATION"
    );
    assert_eq!(row(&page, 16), "");
    assert_eq!(row(&page, 17), "V RADAR  UP/DN/WHEEL ZOOM  TAB SELECT");
}

#[test]
fn no_frame_is_unsampled_not_zero_and_omits_all_selected_rows() {
    let app = app(1, false);
    assert_eq!(app.counts().failed, 0); // App API is not redefined for debug.
    let page = Page::from_app(&app, true);
    assert_eq!(row(&page, 1), "REPLAY UTC NO FRAME");
    assert_eq!(row(&page, 2), "VERTICAL FOV 20.00 DEG");
    assert_eq!(left(&page, 3), "CATALOGUE 2");
    assert_eq!(right(&page, 3), "SKY >0 DEG --");
    assert_eq!(left(&page, 4), "IN VIEW --");
    assert_eq!(right(&page, 4), "FAILED --");
    assert_eq!(row(&page, 5), "TRACKING NOT SAMPLED / NO VALID FRAME");
    assert_eq!(row(&page, 6), "");
    assert_eq!(row(&page, 7), "");
    assert_eq!(row(&page, 8), "INGEST WARN / DETAILS: HOST STDERR");
    assert_eq!(row(&page, 9), "NO SELECTED VISIBLE OBJECT");
    for i in 10..=16 {
        assert_eq!(row(&page, i), "");
    }
}

#[test]
fn mixed_failures_retain_selection_and_first_ascending_id_with_full_detail_guidance() {
    let app = app(2, true);
    let page = Page::from_app(&app, true);
    assert_eq!(app.counts().failed, 2);
    assert_eq!(left(&page, 3), "CATALOGUE 3");
    assert_eq!(left(&page, 4), "IN VIEW 1");
    assert_eq!(right(&page, 4), "FAILED 2");
    assert_eq!(row(&page, 5), "TRACK ERROR / FIRST NORAD 99998");
    assert!(row(&page, 6).starts_with("CAUSE propagation: "));
    assert!(row(&page, 7).contains("HOST STDERR"));
    assert_eq!(row(&page, 8), "INGEST WARN / DETAILS: HOST STDERR");
    assert_eq!(row(&page, 9), "SELECTED NORAD 25544");
    assert!(left(&page, 12).starts_with("ALT WGS84 "));
    assert!(right(&page, 14).starts_with("EPOCH OFFSET "));
}

#[test]
fn redraws_clear_stale_names_values_causes_and_never_change_app_truth() {
    let mut app = app(2, true);
    let time = app.time().unwrap();
    let counts = app.counts();
    let view = app.view();
    let selection = app.selected_id();
    let samples: Vec<_> = app.objects().map(|(_, sample)| sample.clone()).collect();
    let mut screen = Screen::new(true);
    draw(&mut screen, &app, true).unwrap();
    let expected = screen.0.clone();
    draw(&mut screen, &app, true).unwrap();
    assert_eq!(screen.0, expected);
    assert_eq!(app.time(), Some(time));
    assert_eq!(app.counts(), counts);
    assert_eq!(app.view(), view);
    assert_eq!(app.selected_id(), selection);
    assert_eq!(
        app.objects()
            .map(|(_, sample)| sample.clone())
            .collect::<Vec<_>>(),
        samples
    );
    for y in 0..240 {
        for x in 0..400 {
            if !(8..392).contains(&x) || !(12..226).contains(&y) {
                assert_eq!(screen.0[y * 400 + x], BinaryColor::Off);
            }
        }
    }
    // Invalid UTC clears app selection; debug must clear all old selected ink
    // and causes on the *same* display, not retain a previous valid page.
    let invalid = NaiveDate::from_ymd_opt(2101, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    assert!(app.update(invalid).is_err());
    draw(&mut screen, &app, false).unwrap();
    let mut clean = Screen::new(false);
    draw(&mut clean, &app, false).unwrap();
    assert_eq!(screen.0, clean.0);
    for y in 132..216 {
        assert!(
            screen.0[y * 400..(y + 1) * 400]
                .iter()
                .all(|c| *c == BinaryColor::Off)
        );
    }
    app.update(time).unwrap();
    draw(&mut screen, &app, true).unwrap();
    assert_eq!(screen.0, expected);
}

#[test]
fn rendered_text_matches_fixed_cells_without_radar_or_separator() {
    let app = app(0, false);
    let mut actual = Screen::new(true);
    draw(&mut actual, &app, true).unwrap();
    let mut expected = Screen::new(false);
    for (row, column, text) in [
        (0, 0, "DEBUG / LOCAL INPUT / NO FETCH"),
        (1, 0, "REPLAY UTC NO FRAME"),
        (2, 0, "VERTICAL FOV 20.00 DEG"),
        (3, 0, "CATALOGUE 1"),
        (3, 1, "SKY >0 DEG --"),
        (4, 0, "IN VIEW --"),
        (4, 1, "FAILED --"),
        (5, 0, "TRACKING NOT SAMPLED / NO VALID FRAME"),
        (8, 0, "INGEST WARN / DETAILS: HOST STDERR"),
        (9, 0, "NO SELECTED VISIBLE OBJECT"),
        (17, 0, "V RADAR  UP/DN/WHEEL ZOOM  TAB SELECT"),
    ] {
        Text::new(
            text,
            Point::new(8 + column * 192, 19 + row * 12),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        )
        .draw(&mut expected)
        .unwrap();
    }
    assert_eq!(actual.0, expected.0);
}

#[test]
fn causes_are_explicitly_bounded_sanitized_and_keep_complete_id() {
    let mut page = Page::from_app(&app(0, false), false);
    let fits = "A".repeat(58); // CAUSE prefix + 58 = exactly 64 cells.
    page.failure(u64::MAX, format_args!("{fits}"));
    assert_eq!(
        row(&page, 5),
        "TRACK ERROR / FIRST NORAD 18446744073709551615"
    );
    assert_eq!(row(&page, 6), format!("CAUSE {fits}"));
    assert_eq!(row(&page, 7), "FULL CAUSES / OTHER IDS: HOST STDERR");
    page.failure(1, format_args!("{fits}B"));
    assert_eq!(row(&page, 6), format!("CAUSE {}...", "A".repeat(55)));
    assert_eq!(
        row(&page, 7),
        "CAUSE CLIPPED; FULL CAUSES / OTHER IDS: HOST STDERR"
    );
    page.failure(2, format_args!("short\ncause π?"));
    assert_eq!(row(&page, 6), "CAUSE short?cause ??");
    assert_eq!(
        row(&page, 7),
        "CAUSE SHORTENED/CHANGED; FULL DETAILS: HOST STDERR"
    );
    page.failure(3, format_args!("{fits}B\n")); // Changed even in omitted tail.
    assert!(row(&page, 6).ends_with("..."));
    assert_eq!(
        row(&page, 7),
        "CAUSE SHORTENED/CHANGED; FULL DETAILS: HOST STDERR"
    );
}

#[test]
fn names_wrap_at_spaces_hard_wrap_long_tokens_and_mark_every_loss() {
    let make = |name: Option<&str>| {
        let mut page = Page::from_app(&app(0, false), false);
        page.name(name);
        page
    };
    for name in [None, Some("")] {
        assert_eq!(row(&make(name), 10), "NAME UNNAMED");
    }
    let a = "A".repeat(59);
    let page = make(Some(&a));
    assert_eq!(row(&page, 10), format!("NAME {a}"));
    assert_eq!(row(&page, 11), "");
    for separator in ["", " ", "   "] {
        let page = make(Some(&format!("{a}{separator}{a}")));
        assert_eq!(row(&page, 10), format!("NAME {a}"));
        assert_eq!(row(&page, 11), format!("     {a}"));
    }
    let page = make(Some(&format!("{a}{a}B")));
    assert_eq!(
        row(&page, 11),
        format!("     {}... [MORE: HOST STDERR]", "A".repeat(36))
    );
    let page = make(Some(&format!("{} WORD NEXT", "A".repeat(55))));
    assert_eq!(row(&page, 10), format!("NAME {}", "A".repeat(55)));
    assert_eq!(row(&page, 11), "     WORD NEXT");
    let page = make(Some("LITERAL ?"));
    assert_eq!(row(&page, 16), "");
    let page = make(Some("SYNTHETIC λ\n\t\u{7f} NAME"));
    assert_eq!(row(&page, 10), "NAME SYNTHETIC ???? NAME");
    assert_eq!(
        row(&page, 16),
        "NAME '?' REPLACEMENTS; ORIGINAL: HOST STDERR"
    );
    let page = make(Some(&format!("{a}{a}λ")));
    assert!(row(&page, 11).ends_with("... [MORE: HOST STDERR]"));
    assert!(!row(&page, 16).is_empty());
    let page = make(Some(&"λ".repeat(118)));
    assert_eq!(row(&page, 10), format!("NAME {}", "?".repeat(59)));
    assert_eq!(row(&page, 11), format!("     {}", "?".repeat(59)));
}

#[test]
fn missing_and_invalid_selected_fields_are_local_without_stale_readings() {
    let mut page = Page::from_app(&app(0, true), false);
    for invalid_period in [
        None,
        Some(0.0),
        Some(-1.0),
        Some(f64::NAN),
        Some(f64::INFINITY),
    ] {
        page.selected_values([
            Some(400.0),
            Some(1234.5),
            Some(45.0),
            None,
            invalid_period,
            Some(-0.25),
        ]);
        assert_eq!(left(&page, 12), "ALT WGS84 400.0 KM");
        assert_eq!(right(&page, 12), "RANGE 1234.5 KM");
        assert_eq!(left(&page, 13), "ELEV 45.0 DEG");
        assert_eq!(right(&page, 13), "TEME SPEED UNAVAILABLE");
        assert_eq!(left(&page, 14), "NOM PERIOD UNAVAILABLE");
        assert_eq!(right(&page, 14), "EPOCH OFFSET -0.25 H");
        assert_eq!(row(&page, 9), "SELECTED NORAD 25544");
    }
    page.selected_values([None; 6]);
    assert_eq!(left(&page, 12), "ALT WGS84 UNAVAILABLE");
    assert_eq!(right(&page, 12), "RANGE UNAVAILABLE");
    assert_eq!(left(&page, 13), "ELEV UNAVAILABLE");
    assert_eq!(right(&page, 14), "EPOCH OFFSET UNAVAILABLE");
    let recovered = Page::from_app(&app(0, true), false);
    assert_ne!(row(&page, 13), row(&recovered, 13));
}

#[test]
fn empty_view_with_sky_objects_omits_values_without_claiming_empty_sky() {
    let mut app = app(0, true);
    let time = app.time().unwrap() + TimeDelta::seconds(60);
    app.zoom(64);
    app.update(time).unwrap();
    assert_eq!(app.counts().visible, 0);
    assert_eq!(app.counts().above_horizon, 1);
    let page = Page::from_app(&app, false);
    assert_eq!(right(&page, 3), "SKY >0 DEG 1");
    assert_eq!(left(&page, 4), "IN VIEW 0");
    assert_eq!(row(&page, 9), "NO SELECTED VISIBLE OBJECT");
    for i in 10..=16 {
        assert_eq!(row(&page, i), "");
    }
}

#[test]
fn numeric_fields_keep_units_precision_sign_and_explicit_scientific_fallback() {
    assert_eq!(
        numeric("ALT WGS84", Some(1234.567), 1, "KM", false).as_str(),
        "ALT WGS84 1234.6 KM"
    );
    assert_eq!(
        numeric("RANGE", Some(0.0), 1, "KM", false).as_str(),
        "RANGE 0.0 KM"
    );
    assert_eq!(
        numeric("ELEV", Some(-0.25), 1, "DEG", false).as_str(),
        "ELEV -0.2 DEG"
    );
    assert_eq!(
        numeric("TEME SPEED", Some(7.12349), 3, "KM/S", false).as_str(),
        "TEME SPEED 7.123 KM/S"
    );
    assert_eq!(
        numeric("EPOCH OFFSET", Some(-192.1251), 2, "H", true).as_str(),
        "EPOCH OFFSET -192.13 H"
    );
    // 32 cells exactly must stay fixed-decimal; the next digit triggers fallback.
    assert_eq!(
        numeric("ALT WGS84", Some(1e16), 1, "KM", false).as_str(),
        "ALT WGS84 10000000000000000.0 KM"
    );
    assert_eq!(
        numeric("ALT WGS84", Some(1e17), 1, "KM", false).as_str(),
        "ALT WGS84 1.00e17 KM"
    );
    assert_eq!(
        numeric("ALT WGS84", Some(1e18), 1, "KM", false).as_str(),
        "ALT WGS84 1.00e18 KM"
    );
    assert_eq!(
        numeric("TEME SPEED", Some(1.234567e100), 3, "KM/S", false).as_str(),
        "TEME SPEED 1.23e100 KM/S"
    );
    assert_eq!(
        numeric("EPOCH OFFSET", Some(-1e100), 2, "H", true).as_str(),
        "EPOCH OFFSET -1.00e100 H"
    );
    assert_eq!(
        numeric("EPOCH OFFSET", Some(1e100), 2, "H", true).as_str(),
        "EPOCH OFFSET +1.00e100 H"
    );
    for (label, precision, unit, signed) in [
        ("ALT WGS84", 1, "KM", false),
        ("RANGE", 1, "KM", false),
        ("ELEV", 1, "DEG", false),
        ("TEME SPEED", 3, "KM/S", false),
        ("NOM PERIOD", 1, "MIN", false),
        ("EPOCH OFFSET", 2, "H", true),
    ] {
        for value in [
            None,
            Some(f64::NAN),
            Some(f64::INFINITY),
            Some(f64::NEG_INFINITY),
        ] {
            assert_eq!(
                numeric(label, value, precision, unit, signed).as_str(),
                format!("{label} UNAVAILABLE")
            );
        }
        for value in [f64::MAX, -f64::MAX, f64::MIN_POSITIVE, -0.0, 0.0] {
            let field = numeric(label, Some(value), precision, unit, signed);
            assert!(!field.shortened);
            assert!(field.as_str().len() <= 32);
            assert!(field.as_str().ends_with(unit));
        }
    }
}

#[test]
fn twenty_digit_counts_and_ids_fit_without_saturation_or_column_overlap() {
    let mut page = Page::from_app(&app(0, false), false);
    for (r, c, label) in [
        (3, 0, "CATALOGUE"),
        (3, 1, "SKY >0 DEG"),
        (4, 0, "IN VIEW"),
        (4, 1, "FAILED"),
    ] {
        // Replace existing rows before testing maximum-width synthetic counts.
        if c == 0 {
            page.rows[r] = TextBuffer::new();
        }
        page.pair(r, c, format_args!("{label} {}", u64::MAX));
    }
    assert_eq!(left(&page, 3), "CATALOGUE 18446744073709551615");
    assert_eq!(right(&page, 3), "SKY >0 DEG 18446744073709551615");
    assert_eq!(left(&page, 4), "IN VIEW 18446744073709551615");
    assert_eq!(right(&page, 4), "FAILED 18446744073709551615");
    page.line(9, format_args!("SELECTED NORAD {}", u64::MAX));
    assert!(row(&page, 9).ends_with(&u64::MAX.to_string()));
}

#[test]
fn fractional_replay_and_signed_epoch_offset_never_round_time_first() {
    let mut app = app(0, true);
    let epoch = app.time().unwrap();
    for nanos in [
        1,
        123_456_789,
        -1,
        -123_456_789,
        17_999_999_999,
        -17_999_999_999,
    ] {
        let time = epoch + TimeDelta::nanoseconds(nanos);
        assert!((epoch_hours(time, epoch) - nanos as f64 / 3.6e12).abs() < 1e-15);
    }
    for (delta, sign) in [(17_900, "+"), (-17_900, "-")] {
        app.update(epoch + TimeDelta::milliseconds(delta)).unwrap();
        let page = Page::from_app(&app, false);
        assert_eq!(right(&page, 14), format!("EPOCH OFFSET {sign}0.00 H"));
    }
    let time = NaiveDate::from_ymd_opt(2026, 10, 4)
        .unwrap()
        .and_hms_nano_opt(12, 43, 41, 123_456_789)
        .unwrap();
    app.update(time).unwrap();
    assert_eq!(
        row(&Page::from_app(&app, false), 1),
        "REPLAY UTC 2026-10-04T12:43:41.123456789Z"
    );
}
