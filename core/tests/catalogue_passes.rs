#![cfg(feature = "catalogue")]

use overhead_core::{
    GeodeticPosition, OmmElements, PropagateError,
    catalogue::{Catalogue, CatalogueBuilder, RecordOrigin},
    catalogue_passes::search_catalogue,
    passes::{
        EvaluationBudget, OrbitalEvaluationError, PassEnd, PassStart, SearchConfig, SearchStatus,
        StopReason, search_satellite,
    },
    sgp4::chrono::TimeDelta,
};
use serde_json::{Value, json};

fn catalogue(fail_first: bool) -> Catalogue {
    let original: Value = serde_json::from_str(include_str!("fixtures/iss-25544.json")).unwrap();
    let mut builder = CatalogueBuilder::new();
    for id in [3, 1, 2] {
        let mut record = original[0].clone();
        record["NORAD_CAT_ID"] = json!(id);
        if fail_first && id == 1 {
            record["BSTAR"] = json!(1e300);
        }
        let elements: OmmElements = serde_json::from_value(record).unwrap();
        builder.push(
            RecordOrigin {
                group: 0,
                record: id as usize,
            },
            elements,
        );
    }
    builder.finish().unwrap()
}

fn observer() -> GeodeticPosition {
    GeodeticPosition {
        latitude_rad: 39.007_f64.to_radians(),
        longitude_rad: -104.883_f64.to_radians(),
        altitude_km: 2.187,
    }
}

fn config(c: &Catalogue, threshold: f64, duration: i64) -> SearchConfig {
    SearchConfig::new(
        c.entries()[0].satellite().epoch(),
        TimeDelta::seconds(duration),
        threshold.to_radians(),
        TimeDelta::seconds(60),
        TimeDelta::seconds(5),
    )
    .unwrap()
}

#[test]
fn orbital_catalogue_matches_individual_searches_and_preserves_ambiguous_arrivals() {
    let c = catalogue(false);
    let config = config(&c, 10.0, 86400);
    let mut budget = EvaluationBudget::new(10000);
    let results = search_catalogue(&c, observer(), &config, 3000, &mut budget);
    assert!(results.is_complete());
    assert_eq!(results.evaluations(), budget.used());
    assert_eq!(results.earliest_candidates().len(), 3); // identical test orbits
    for (entry, stored) in c.entries().iter().zip(results.satellites()) {
        let mut passes = Vec::new();
        let report = search_satellite(
            entry.satellite(),
            observer(),
            &config,
            3000,
            &mut EvaluationBudget::new(3000),
            |p| passes.push(p),
        );
        assert_eq!(stored.norad_id(), entry.satellite().norad_id());
        assert_eq!(stored.report(), &report);
        assert_eq!(stored.passes(), passes);
    }
    assert_eq!(
        results,
        search_catalogue(
            &c,
            observer(),
            &config,
            3000,
            &mut EvaluationBudget::new(10000)
        )
    );
}

#[test]
fn exhausted_shared_budget_keeps_partial_and_all_unsearched_details() {
    let c = catalogue(false);
    let config = config(&c, -90.0, 60);
    let mut budget = EvaluationBudget::new(4);
    // Pre-consume one evaluation; aggregation reports only its own usage.
    let _ = search_satellite(
        c.entries()[0].satellite(),
        observer(),
        &config,
        1,
        &mut budget,
        |_| {},
    );
    let r = search_catalogue(&c, observer(), &config, 100, &mut budget);
    assert_eq!(r.evaluations(), 3);
    assert_eq!(budget.used(), 4);
    assert!(!r.is_complete());
    assert_eq!(
        r.satellites()
            .iter()
            .map(|s| s.norad_id())
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert!(r.satellites()[0].report().is_complete());
    assert_eq!(r.satellites()[0].passes()[0].end, PassEnd::BeyondWindow);
    assert!(matches!(
        r.satellites()[1].report().status,
        SearchStatus::Incomplete(_)
    ));
    assert_eq!(r.satellites()[1].passes()[0].end, PassEnd::Interrupted);
    assert!(
        matches!(&r.satellites()[2].report().status, SearchStatus::Unsearched(s) if s.reason == StopReason::TotalLimit && s.at == config.start())
    );
    assert!(r.satellites()[2].passes().is_empty());
    assert!(r.earliest_candidates().is_empty());
}

#[test]
fn local_limits_continue_and_zero_allowances_are_not_no_pass() {
    let c = catalogue(false);
    let config = config(&c, -90.0, 60);
    for (local, total) in [(1, 10), (0, 10), (10, 0)] {
        let r = search_catalogue(
            &c,
            observer(),
            &config,
            local,
            &mut EvaluationBudget::new(total),
        );
        assert!(!r.is_complete());
        assert_eq!(r.satellites().len(), 3);
        for s in r.satellites() {
            if local == 1 {
                assert!(
                    matches!(&s.report().status, SearchStatus::Incomplete(stop) if stop.reason == StopReason::SatelliteLimit)
                );
                assert_eq!(s.passes()[0].start, PassStart::InProgress);
            } else {
                assert!(matches!(s.report().status, SearchStatus::Unsearched(_)));
                assert_eq!(s.report().evaluations, 0);
            }
        }
    }
}

#[test]
fn failed_satellite_retains_typed_error_and_does_not_prevent_later_results() {
    let c = catalogue(true);
    let config = config(&c, -90.0, 60);
    let r = search_catalogue(&c, observer(), &config, 10, &mut EvaluationBudget::new(10));
    assert!(!r.is_complete());
    assert_eq!(r.evaluations(), 5);
    assert!(
        matches!(&r.satellites()[0].report().status, SearchStatus::Incomplete(stop)
        if stop.reason == StopReason::Evaluation(OrbitalEvaluationError::Propagation(PropagateError::NonFinite)))
    );
    assert!(r.satellites()[0].passes().is_empty());
    for s in &r.satellites()[1..] {
        assert!(s.report().is_complete());
        assert_eq!(s.passes()[0].end, PassEnd::BeyondWindow);
    }
}

#[test]
fn completed_no_arrival_can_coexist_with_in_progress() {
    let c = catalogue(false);
    for threshold in [-90.0, 90.0] {
        let r = search_catalogue(
            &c,
            observer(),
            &config(&c, threshold, 60),
            2,
            &mut EvaluationBudget::new(6),
        );
        assert!(r.is_complete());
        assert!(r.earliest_candidates().is_empty());
        assert_eq!(
            r.satellites()[0].passes().len(),
            usize::from(threshold < 0.0)
        );
    }
}
