//! Ingestion tests: the checked-in OMM fixture must validate, and corrupt
//! element sets must be rejected with a specific error.
//!
//! Fixture provenance: tests/fixtures/README.md.

use overhead_core::{IngestError, Satellite};
use sgp4::chrono::Datelike;

fn fixture_elements() -> sgp4::Elements {
    let mut sets: Vec<sgp4::Elements> =
        serde_json::from_str(include_str!("fixtures/iss-25544.json"))
            .expect("fixture must be a valid OMM JSON array");
    assert_eq!(sets.len(), 1, "fixture holds exactly one element set");
    sets.pop().unwrap()
}

#[test]
fn fixture_parses_and_validates() {
    let elements = fixture_elements();
    let satellite = Satellite::from_elements(&elements).expect("fixture must validate");
    assert_eq!(satellite.norad_id(), 25544);
    assert_eq!(satellite.epoch(), elements.datetime);
    assert_eq!(satellite.epoch().year(), 2026);
}

#[test]
fn rejects_eccentricity_of_one_or_more() {
    let mut elements = fixture_elements();
    elements.eccentricity = 1.1;
    match Satellite::from_elements(&elements) {
        Err(IngestError::EccentricityOutOfRange(value)) => assert_eq!(value, 1.1),
        Err(other) => panic!("wrong error: {other}"),
        Ok(_) => panic!("eccentricity 1.1 must be rejected"),
    }
}

#[test]
fn rejects_negative_eccentricity() {
    let mut elements = fixture_elements();
    elements.eccentricity = -0.1;
    assert!(matches!(
        Satellite::from_elements(&elements),
        Err(IngestError::EccentricityOutOfRange(_))
    ));
}

#[test]
fn rejects_non_positive_mean_motion() {
    let mut elements = fixture_elements();
    elements.mean_motion = 0.0;
    assert!(matches!(
        Satellite::from_elements(&elements),
        Err(IngestError::NonPositiveMeanMotion(_))
    ));
}

#[test]
fn rejects_out_of_range_inclination() {
    let mut elements = fixture_elements();
    elements.inclination = 200.0;
    assert!(matches!(
        Satellite::from_elements(&elements),
        Err(IngestError::InclinationOutOfRange(_))
    ));
}

#[test]
fn rejects_non_finite_fields() {
    let mut elements = fixture_elements();
    elements.drag_term = f64::NAN;
    assert!(matches!(
        Satellite::from_elements(&elements),
        Err(IngestError::NonFinite)
    ));
}

#[test]
fn rejects_implausible_epoch() {
    let mut elements = fixture_elements();
    elements.datetime = elements.datetime.with_year(1900).unwrap();
    assert!(matches!(
        Satellite::from_elements(&elements),
        Err(IngestError::ImplausibleEpoch(1900))
    ));
}

#[test]
fn rejects_omm_with_missing_fields() {
    let result: Result<sgp4::Elements, _> =
        serde_json::from_str(r#"{"OBJECT_NAME": "BROKEN", "NORAD_CAT_ID": 1}"#);
    assert!(result.is_err(), "OMM without orbital fields must not parse");
}

#[test]
fn rejects_malformed_json() {
    let result: Result<Vec<sgp4::Elements>, _> = serde_json::from_str("[{\"EPOCH\": ");
    assert!(result.is_err());
}
