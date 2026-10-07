#![cfg(feature = "catalogue")]

use overhead_core::{
    OmmElements,
    catalogue::{CatalogueBuilder, CatalogueDiagnostic, RecordOrigin},
};
use serde_json::{Value, json};

const ISS: &str = include_str!("fixtures/iss-25544.json");

fn object() -> Value {
    serde_json::from_str::<Value>(ISS).unwrap()[0].clone()
}

fn origin(group: usize, record: usize) -> RecordOrigin {
    RecordOrigin { group, record }
}

fn push(builder: &mut CatalogueBuilder, source: RecordOrigin, value: Value) {
    let elements: OmmElements = serde_json::from_value(value).unwrap();
    builder.push(source, elements);
}

#[test]
fn single_record_is_ready_for_propagation_and_empty_build_fails() {
    assert!(CatalogueBuilder::new().finish().is_err());
    let mut builder = CatalogueBuilder::new();
    push(&mut builder, origin(0, 0), object());
    let catalogue = builder.finish().unwrap();
    assert!(catalogue.diagnostics().is_empty());
    let entry = &catalogue.entries()[0];
    assert_eq!(entry.satellite().norad_id(), 25544);
    assert_eq!(entry.selected_from(), origin(0, 0));
    entry
        .satellite()
        .state_at(entry.satellite().epoch())
        .unwrap();
}

#[test]
fn newest_valid_epoch_wins_and_membership_is_unique_sorted_and_includes_older_groups() {
    let old = object();
    let mut newer = old.clone();
    newer["EPOCH"] = json!("2026-10-05T00:00:00");
    newer["MEAN_ANOMALY"] = json!(140.0);
    let mut invalid = newer.clone();
    invalid["EPOCH"] = json!("2026-10-06T00:00:00");
    invalid["MEAN_MOTION"] = json!(0);
    let records = [
        (origin(2, 0), old.clone()),
        (origin(0, 1), newer),
        (origin(0, 0), old),
        (origin(1, 0), invalid),
    ];
    for order in [[0, 1, 2, 3], [3, 2, 1, 0]] {
        let mut builder = CatalogueBuilder::new();
        for i in order {
            push(&mut builder, records[i].0, records[i].1.clone());
        }
        let catalogue = builder.finish().unwrap();
        assert_eq!(catalogue.entries().len(), 1);
        let entry = &catalogue.entries()[0];
        assert_eq!(entry.selected_from(), origin(0, 1));
        assert_eq!(entry.elements().mean_anomaly, 140.0);
        assert_eq!(entry.groups(), &[0, 2]);
        assert!(matches!(
            catalogue.diagnostics(),
            [CatalogueDiagnostic::InvalidRecord {
                origin: RecordOrigin {
                    group: 1,
                    record: 0
                },
                norad_id: 25544,
                ..
            }]
        ));
    }
}

#[test]
fn equivalent_orbits_ignore_nonorbital_metadata_and_use_lowest_origin() {
    let mut alternate = object();
    alternate["OBJECT_NAME"] = json!("Other name");
    alternate["OBJECT_ID"] = json!("Other designator");
    alternate["ELEMENT_SET_NO"] = json!(12);
    alternate["REV_AT_EPOCH"] = json!(13);
    alternate["CLASSIFICATION_TYPE"] = json!("C");
    alternate["MEAN_MOTION"] = json!("15.48731752");
    alternate["CENTER_NAME"] = json!("EARTH");
    for reverse in [false, true] {
        let mut records = [(origin(1, 0), alternate.clone()), (origin(0, 3), object())];
        if reverse {
            records.reverse();
        }
        let mut builder = CatalogueBuilder::new();
        for (source, value) in records {
            push(&mut builder, source, value);
        }
        let catalogue = builder.finish().unwrap();
        assert_eq!(catalogue.entries().len(), 1);
        let entry = &catalogue.entries()[0];
        assert_eq!(entry.selected_from(), origin(0, 3));
        assert_eq!(entry.elements().object_name.as_deref(), Some("ISS (ZARYA)"));
        assert_eq!(entry.groups(), &[0, 1]);
        assert!(catalogue.diagnostics().is_empty());
    }
}

#[test]
fn each_orbital_field_can_cause_an_equal_epoch_conflict() {
    for field in [
        "MEAN_MOTION",
        "ECCENTRICITY",
        "INCLINATION",
        "RA_OF_ASC_NODE",
        "ARG_OF_PERICENTER",
        "MEAN_ANOMALY",
        "BSTAR",
        "MEAN_MOTION_DOT",
        "MEAN_MOTION_DDOT",
        "EPHEMERIS_TYPE",
    ] {
        let mut different = object();
        different[field] = if field == "EPHEMERIS_TYPE" {
            json!(1)
        } else {
            json!(different[field].as_f64().unwrap() + 0.000001)
        };
        let mut builder = CatalogueBuilder::new();
        push(&mut builder, origin(1, 0), different);
        push(&mut builder, origin(0, 0), object());
        let error = match builder.finish() {
            Err(error) => error,
            Ok(_) => panic!("accepted conflict in {field}"),
        };
        assert!(
            matches!(&error.diagnostics[..], [CatalogueDiagnostic::EpochConflict {
            norad_id: 25544, origins, ..
        }] if origins == &[origin(0, 0), origin(1, 0)])
        );
    }
}

#[test]
fn conflicts_exclude_only_the_affected_id_and_do_not_fall_back_to_older_elements() {
    let mut different = object();
    different["MEAN_ANOMALY"] = json!(140);
    let mut older = object();
    older["EPOCH"] = json!("2026-10-03T00:00:00");
    let mut other = object();
    other["NORAD_CAT_ID"] = json!(42);
    let mut builder = CatalogueBuilder::new();
    push(&mut builder, origin(0, 0), object());
    push(&mut builder, origin(0, 1), different);
    push(&mut builder, origin(1, 0), older);
    push(&mut builder, origin(2, 0), other);
    let catalogue = builder.finish().unwrap();
    assert_eq!(catalogue.entries().len(), 1);
    assert_eq!(catalogue.entries()[0].satellite().norad_id(), 42);
    assert!(matches!(
        catalogue.diagnostics(),
        [CatalogueDiagnostic::EpochConflict { .. }]
    ));
}

#[test]
fn newer_unambiguous_epoch_resolves_old_conflicts_in_every_arrival_order() {
    let mut different = object();
    different["MEAN_ANOMALY"] = json!(140);
    let mut newer = object();
    newer["EPOCH"] = json!("2026-10-05T00:00:00");
    let records = [object(), different, newer];
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let mut builder = CatalogueBuilder::new();
        for i in order {
            push(&mut builder, origin(i, 0), records[i].clone());
        }
        let catalogue = builder.finish().unwrap();
        assert_eq!(catalogue.entries()[0].selected_from(), origin(2, 0));
        assert_eq!(catalogue.entries()[0].groups(), &[0, 1, 2]);
        assert!(catalogue.diagnostics().is_empty());
    }
}

#[test]
fn conflict_is_not_cleared_by_an_equivalent_record_or_an_invalid_newer_one() {
    let mut different = object();
    different["MEAN_ANOMALY"] = json!(140);
    let mut invalid = object();
    invalid["EPOCH"] = json!("2026-10-06T00:00:00");
    invalid["ECCENTRICITY"] = json!(2);
    let records = [object(), different, object(), invalid];
    for order in [[0, 1, 2, 3], [3, 2, 1, 0], [1, 0, 3, 2]] {
        let mut builder = CatalogueBuilder::new();
        for i in order {
            push(&mut builder, origin(i, 0), records[i].clone());
        }
        let error = match builder.finish() {
            Err(error) => error,
            Ok(_) => panic!("accepted unresolved conflict"),
        };
        assert!(matches!(&error.diagnostics[..], [
            CatalogueDiagnostic::InvalidRecord { .. },
            CatalogueDiagnostic::EpochConflict { origins, .. }
        ] if origins == &[origin(0, 0), origin(1, 0), origin(2, 0)]));
    }
}

#[test]
fn all_invalid_fails_with_diagnostics_and_entries_sort_by_norad_id() {
    let mut invalid = object();
    invalid["MEAN_MOTION"] = json!(0);
    let mut builder = CatalogueBuilder::new();
    push(&mut builder, origin(0, 0), invalid);
    let error = match builder.finish() {
        Err(error) => error,
        Ok(_) => panic!("accepted an empty catalogue"),
    };
    assert_eq!(error.diagnostics.len(), 1);

    let mut builder = CatalogueBuilder::new();
    for (record, id) in [25544, 100_000, 42].into_iter().enumerate() {
        let mut value = object();
        value["NORAD_CAT_ID"] = json!(id);
        push(&mut builder, origin(0, record), value);
    }
    let catalogue = builder.finish().unwrap();
    let ids: Vec<_> = catalogue
        .entries()
        .iter()
        .map(|e| e.satellite().norad_id())
        .collect();
    assert_eq!(ids, [42, 25544, 100_000]);
}
