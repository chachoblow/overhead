//! The same UTC boundary contract applies at every physical pipeline entry.
//! Synthetic epoch edits test acceptance, not propagation accuracy in those eras.
use overhead_core::{
    CoordinateError, IngestError, PropagateError, Satellite, TemePosition, TimeError, teme_to_ecef,
    validate_utc_time,
};
use sgp4::chrono::{NaiveDateTime, TimeDelta};

fn utc(text: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S%.f").unwrap()
}

fn elements() -> sgp4::Elements {
    let mut sets: Vec<sgp4::Elements> =
        serde_json::from_str(include_str!("fixtures/iss-25544.json")).unwrap();
    sets.pop().unwrap()
}

#[test]
fn supported_time_boundaries_are_accepted_through_the_pipeline() {
    for text in [
        "1957-01-01T00:00:00",
        "2100-12-31T23:59:59.999999999",
        "2000-02-29T12:00:00",
        "2100-03-01T00:00:00",
        "2016-12-31T23:59:59.999999999",
        "2017-01-01T00:00:00",
    ] {
        let time = utc(text);
        let mut elements = elements();
        elements.datetime = time;
        let satellite = Satellite::from_elements(&elements).unwrap();
        assert_eq!(validate_utc_time(time), Ok(()));
        let state = satellite.state_at(time).unwrap();
        assert_eq!(state.datetime(), time);
        assert!(state.to_ecef().is_ok(), "{text}");
        assert!(
            teme_to_ecef(TemePosition::from_km([7000.0, 0.0, 0.0]), time).is_ok(),
            "{text}"
        );
    }
}

#[test]
fn unsupported_epochs_and_requested_times_are_rejected() {
    let satellite = Satellite::from_elements(&elements()).unwrap();
    for (text, expected) in [
        (
            "1956-12-31T23:59:59.999999999",
            TimeError::UnsupportedYear(1956),
        ),
        ("2101-01-01T00:00:00", TimeError::UnsupportedYear(2101)),
        ("2016-12-31T23:59:60", TimeError::LeapSecond),
        ("2016-12-31T23:59:60.5", TimeError::LeapSecond),
    ] {
        let time = utc(text);
        let mut elements = elements();
        elements.datetime = time;
        assert_eq!(validate_utc_time(time), Err(expected));
        let epoch_error = match Satellite::from_elements(&elements) {
            Err(error) => error,
            Ok(_) => panic!("unsupported epoch accepted: {text}"),
        };
        assert_eq!(
            epoch_error,
            match expected {
                TimeError::UnsupportedYear(year) => IngestError::ImplausibleEpoch(year),
                TimeError::LeapSecond => IngestError::LeapSecondEpoch,
            }
        );
        assert_eq!(
            satellite.state_at(time),
            Err(PropagateError::UnsupportedTime(expected))
        );
        assert_eq!(
            teme_to_ecef(TemePosition::from_km([7000.0, 0.0, 0.0]), time),
            Err(CoordinateError::UnsupportedTime)
        );
    }
}

#[test]
fn state_rotation_uses_the_absolute_propagation_time_not_the_epoch() {
    let satellite = Satellite::from_elements(&elements()).unwrap();
    for milliseconds in [-90_000, -500, 0, 500, 90_000] {
        let time = satellite.epoch() + TimeDelta::milliseconds(milliseconds);
        let state = satellite.state_at(time).unwrap();
        assert_eq!(state.datetime(), time);
        let expected = teme_to_ecef(state.position(), time).unwrap();
        assert_eq!(state.to_ecef().unwrap(), expected);
        if milliseconds != 0 {
            let wrong = teme_to_ecef(state.position(), satellite.epoch()).unwrap();
            let error = expected
                .km()
                .iter()
                .zip(wrong.km())
                .map(|(a, b)| (a - b).abs())
                .sum::<f64>();
            assert!(error > 0.01, "test must distinguish rotation at epoch");
        }
        // Copying or requesting another state cannot change the original binding.
        let copied = state;
        satellite.state_at(satellite.epoch()).unwrap();
        assert_eq!(copied.datetime(), time);
        assert_eq!(copied.to_ecef().unwrap(), expected);
    }
}

#[test]
fn elapsed_time_is_signed_subsecond_and_does_not_insert_leap_seconds() {
    let mut elements = elements();
    elements.datetime = utc("2017-01-01T00:00:00");
    let satellite = Satellite::from_elements(&elements).unwrap();
    for nanos in [-1_000_000_000, -500_000_000, 0, 500_000_000] {
        let state = satellite
            .state_at(satellite.epoch() + TimeDelta::nanoseconds(nanos))
            .unwrap();
        assert_eq!(state.minutes_since_epoch(), nanos as f64 / 60e9);
        assert_eq!(
            state.datetime(),
            satellite.epoch() + TimeDelta::nanoseconds(nanos)
        );
    }
}
