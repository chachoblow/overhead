#![cfg(feature = "omm")]

use overhead_core::{IngestError, OmmElements, Satellite, sgp4};
use serde_json::{Value, json};

const ISS: &str = include_str!("fixtures/iss-25544.json");
const METADATA: [(&str, &str, &str); 4] = [
    ("CENTER_NAME", "EARTH", "MARS"),
    ("REF_FRAME", "TEME", "GCRF"),
    ("TIME_SYSTEM", "UTC", "TAI"),
    ("MEAN_ELEMENT_THEORY", "SGP4", "DSST"),
];

fn object() -> Value {
    serde_json::from_str::<Value>(ISS).unwrap()[0].clone()
}

#[test]
fn omitted_and_explicit_defaults_preserve_elements() {
    let expected: Vec<sgp4::Elements> = serde_json::from_str(ISS).unwrap();
    let parsed: Vec<OmmElements> = serde_json::from_str(ISS).unwrap();
    assert_eq!(parsed[0].elements(), &expected[0]);
    let mut data = object();
    for (field, supported, _) in METADATA {
        data[field] = json!(supported);
        // Partially specified metadata also defaults only the absent fields.
        let parsed: OmmElements = serde_json::from_value(data.clone()).unwrap();
        assert_eq!(parsed.elements(), &expected[0]);
        Satellite::from_elements(parsed.elements()).unwrap();
        assert_eq!(parsed.into_elements(), expected[0]);
    }
}

#[test]
fn incompatible_or_malformed_metadata_is_rejected() {
    for (field, supported, unsupported) in METADATA {
        for value in [
            json!(unsupported),
            json!(supported.to_lowercase()),
            json!(""),
            Value::Null,
            json!(42),
            json!({}),
        ] {
            let mut data = object();
            data[field] = value;
            let result = serde_json::from_value::<OmmElements>(data.clone());
            assert!(result.is_err(), "accepted {field}: {data}");
        }
        let mut data = object();
        data[field] = json!(unsupported);
        let error = serde_json::from_value::<OmmElements>(data).unwrap_err();
        assert!(error.to_string().contains(field), "{error}");
    }
}

#[test]
fn unrelated_metadata_is_ignored_but_orbital_validation_is_still_required() {
    let mut data = object();
    data["COMMENT"] = json!("extra metadata does not change the model");
    let parsed: OmmElements = serde_json::from_value(data.clone()).unwrap();
    Satellite::from_elements(parsed.elements()).unwrap();

    data["MEAN_MOTION"] = json!(0);
    let parsed: OmmElements = serde_json::from_value(data).unwrap();
    assert!(matches!(
        Satellite::from_elements(parsed.elements()),
        Err(IngestError::NonPositiveMeanMotion(0.0))
    ));
}

#[test]
fn duplicate_metadata_and_missing_orbital_fields_are_rejected() {
    let data = serde_json::to_string(&object()).unwrap();
    for (field, supported, unsupported) in METADATA {
        let duplicate = format!(
            "{{\"{field}\":\"{supported}\",\"{field}\":\"{unsupported}\",{}",
            &data[1..]
        );
        assert!(serde_json::from_str::<OmmElements>(&duplicate).is_err());
    }
    let mut data = object();
    data.as_object_mut().unwrap().remove("EPOCH");
    assert!(serde_json::from_value::<OmmElements>(data).is_err());
}
