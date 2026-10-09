//! Orbital adapter integration, not an independent pass-time oracle.
//! Existing Skyfield samples anchor elevation (fixtures/README.md). Fine scans
//! use the same orbital model with a separate, unrefined transition detector;
//! agreement is only evidence for these cases, not event completeness or a
//! detection default. All intervals and allowances below are test inputs.

use overhead_core::{
    CoordinateError, GeodeticPosition, ObservationError, PropagateError, Satellite,
    ecef_to_look_angles,
    passes::{
        CrossingBracket, EvaluationBudget, OrbitalEvaluationError, PassEnd, PassStart,
        PredictedPass, SearchConfig, SearchPhase, SearchReport, SearchStatus, StopReason,
        search_satellite,
    },
};
use serde_json::Value;
use sgp4::chrono::{NaiveDateTime, TimeDelta};

fn iss() -> Satellite {
    let elements: Vec<sgp4::Elements> =
        serde_json::from_str(include_str!("fixtures/iss-25544.json")).unwrap();
    Satellite::from_elements(&elements[0]).unwrap()
}

fn from_tle(line1: &str, line2: &str) -> Satellite {
    let elements = sgp4::Elements::from_tle(None, line1.as_bytes(), line2.as_bytes()).unwrap();
    Satellite::from_elements(&elements).unwrap()
}

fn site(lat: f64, lon: f64, height: f64) -> GeodeticPosition {
    GeodeticPosition {
        latitude_rad: lat.to_radians(),
        longitude_rad: lon.to_radians(),
        altitude_km: height,
    }
}

fn elevation(satellite: &Satellite, observer: GeodeticPosition, at: NaiveDateTime) -> f64 {
    ecef_to_look_angles(satellite.state_at(at).unwrap().to_ecef().unwrap(), observer)
        .unwrap()
        .elevation_rad
}

fn config(start: NaiveDateTime, seconds: i64, threshold_deg: f64, step: i64) -> SearchConfig {
    SearchConfig::new(
        start,
        TimeDelta::seconds(seconds),
        threshold_deg.to_radians(),
        TimeDelta::seconds(step),
        TimeDelta::milliseconds(250),
    )
    .unwrap()
}

fn collect(
    satellite: &Satellite,
    observer: GeodeticPosition,
    config: &SearchConfig,
    limit: u64,
    budget: &mut EvaluationBudget,
) -> (SearchReport<OrbitalEvaluationError>, Vec<PredictedPass>) {
    let mut records = Vec::new();
    let report = search_satellite(satellite, observer, config, limit, budget, |pass| {
        records.push(pass);
    });
    assert_eq!(report.passes, records.len() as u64);
    (report, records)
}

#[test]
fn adapter_elevations_match_all_existing_skyfield_samples() {
    let satellite = iss();
    let references: Value = serde_json::from_str(include_str!("fixtures/observer.json")).unwrap();
    let cases = references["iss_pipeline"].as_array().unwrap();
    assert_eq!(cases.len(), 12);
    for case in cases {
        let at =
            NaiveDateTime::parse_from_str(case["utc"].as_str().unwrap(), "%Y-%m-%dT%H:%M:%S%.f")
                .unwrap();
        let observer = &case["observer"];
        let observer = site(
            observer["latitude_deg"].as_f64().unwrap(),
            observer["longitude_deg"].as_f64().unwrap(),
            observer["altitude_km"].as_f64().unwrap(),
        );
        let config = config(at - TimeDelta::seconds(1), 1, 0.0, 1);
        let (report, _) = collect(
            &satellite,
            observer,
            &config,
            2,
            &mut EvaluationBudget::new(2),
        );
        assert!(report.is_complete(), "{case}: {report:?}");
        assert_eq!(report.evaluations, 2);
        let last = report.last_sample.unwrap();
        assert_eq!(last.at, at);
        assert!(
            (last.elevation_rad.to_degrees() - case["elevation_deg"].as_f64().unwrap()).abs()
                < 0.01,
            "{case}: {last:?}"
        );
    }
}

/// Separate one-second transition scan: no search kernel or refinement.
fn fine_transitions(
    satellite: &Satellite,
    observer: GeodeticPosition,
    config: &SearchConfig,
) -> Vec<(bool, NaiveDateTime, NaiveDateTime)> {
    let mut transitions = Vec::new();
    let mut previous_at = config.start();
    let mut previous_above =
        elevation(satellite, observer, previous_at) > config.min_elevation_rad();
    while previous_at < config.end() {
        let at = previous_at + TimeDelta::seconds(1).min(config.end() - previous_at);
        let above = elevation(satellite, observer, at) > config.min_elevation_rad();
        if above != previous_above {
            transitions.push((above, previous_at, at));
        }
        previous_at = at;
        previous_above = above;
    }
    transitions
}

fn assert_fine_scan_agreement(satellite: &Satellite, observer: GeodeticPosition) {
    // Includes negative propagation times and a full Earth rotation; rotating
    // at the element epoch instead of each query would not agree with this scan.
    let config = config(satellite.epoch() - TimeDelta::hours(12), 86400, 10.0, 30);
    let mut budget = EvaluationBudget::new(10000);
    let (report, records) = collect(satellite, observer, &config, 10000, &mut budget);
    assert!(report.is_complete(), "{report:?}");
    assert!(report.upcoming_passes > 0);
    assert_eq!(report.evaluations, budget.used());
    assert_eq!(report.last_sample.unwrap().at, config.end());

    let mut crossings = Vec::new();
    for pass in &records {
        if let PassStart::Crossing(bracket) = pass.start {
            crossings.push((true, bracket));
        }
        if let PassEnd::Crossing(bracket) = pass.end {
            crossings.push((false, bracket));
        }
    }
    let fine = fine_transitions(satellite, observer, &config);
    assert_eq!(crossings.len(), fine.len(), "{records:?}; fine: {fine:?}");
    for ((rising, bracket), (fine_rising, lower, upper)) in crossings.iter().zip(fine) {
        assert_eq!(*rising, fine_rising);
        assert!(bracket.meets_tolerance());
        assert!(bracket.width() <= config.crossing_tolerance());
        assert!(bracket.lower() <= upper && bracket.upper() >= lower);
        assert_bracket_sides(satellite, observer, &config, *bracket, *rising);
    }

    // Refinement asks for timestamps out of order; no mutable propagation cursor
    // or previous search may affect the results or evaluation accounting.
    let (again, repeated) = collect(
        satellite,
        observer,
        &config,
        10000,
        &mut EvaluationBudget::new(10000),
    );
    assert_eq!(again, report);
    assert_eq!(repeated, records);
}

fn assert_bracket_sides(
    satellite: &Satellite,
    observer: GeodeticPosition,
    config: &SearchConfig,
    bracket: CrossingBracket,
    rising: bool,
) {
    assert_eq!(
        elevation(satellite, observer, bracket.lower()) > config.min_elevation_rad(),
        !rising
    );
    assert_eq!(
        elevation(satellite, observer, bracket.upper()) > config.min_elevation_rad(),
        rising
    );
}

#[test]
fn iss_pass_brackets_agree_with_fine_scans_in_both_hemispheres() {
    let satellite = iss();
    for observer in [
        site(39.007, -104.883, 2.187),
        site(-33.8688, 151.2093, 0.058),
    ] {
        assert_fine_scan_agreement(&satellite, observer);
    }
}

#[test]
fn deep_space_pass_brackets_agree_with_fine_scan() {
    // Same nonresonant STR#3 verification satellite as tests/propagate.rs.
    // This is integration coverage, not an independent non-LEO geometry oracle.
    let satellite = from_tle(
        "1 11801U          80230.29629788  .01431103  00000-0  14311-1 0    13",
        "2 11801  46.7916 230.4354 7318036  47.4722  10.4117  2.28537848    13",
    );
    assert_fine_scan_agreement(&satellite, site(-33.8688, 151.2093, 0.058));
}

#[test]
fn orbital_windows_distinguish_arrivals_in_progress_and_unknown_ends() {
    let satellite = iss();
    // The independent Skyfield sample puts ISS at ~32.87 degrees at epoch here.
    let observer = site(0.0, -80.0, 0.0);
    for (offset, seconds, starts_underway, ends_beyond) in [
        (-600, 1200, false, false),
        (-600, 600, false, true),
        (0, 600, true, false),
        (0, 1, true, true),
    ] {
        let config = config(
            satellite.epoch() + TimeDelta::seconds(offset),
            seconds,
            10.0,
            30,
        );
        let (report, records) = collect(
            &satellite,
            observer,
            &config,
            1000,
            &mut EvaluationBudget::new(1000),
        );
        assert!(report.is_complete());
        assert_eq!(records.len(), 1);
        assert_eq!(report.upcoming_passes, u64::from(!starts_underway));
        if starts_underway {
            assert_eq!(records[0].start, PassStart::InProgress);
        } else {
            assert!(matches!(records[0].start, PassStart::Crossing(_)));
        }
        if ends_beyond {
            assert_eq!(records[0].end, PassEnd::BeyondWindow);
        } else {
            assert!(matches!(records[0].end, PassEnd::Crossing(_)));
        }
    }
}

#[test]
fn negative_elevations_are_not_clipped_to_radar_horizon() {
    let satellite = iss();
    let observer = site(0.0, 90.0, 0.0);
    for threshold in [-90.0, 0.0, 10.0] {
        let config = config(satellite.epoch(), 1, threshold, 1);
        let (report, records) = collect(
            &satellite,
            observer,
            &config,
            2,
            &mut EvaluationBudget::new(2),
        );
        assert!(report.is_complete());
        assert!(report.last_sample.unwrap().elevation_rad < 0.0);
        assert_eq!(report.upcoming_passes, 0);
        if threshold == -90.0 {
            assert_eq!(
                records,
                [PredictedPass {
                    start: PassStart::InProgress,
                    end: PassEnd::BeyondWindow
                }]
            );
        } else {
            assert!(records.is_empty());
        }
    }
}

#[test]
fn orbital_refinement_exhaustion_preserves_both_crossing_directions() {
    let satellite = iss();
    let observer = site(0.0, -80.0, 0.0);
    for (offset, rising, phase) in [
        (-600, true, SearchPhase::RisingRefinement),
        (0, false, SearchPhase::FallingRefinement),
    ] {
        let config = config(
            satellite.epoch() + TimeDelta::seconds(offset),
            600,
            10.0,
            600,
        );
        let mut budget = EvaluationBudget::new(100);
        let (report, records) = collect(&satellite, observer, &config, 2, &mut budget);
        let SearchStatus::Incomplete(stop) = report.status else {
            panic!("{report:?}")
        };
        assert_eq!(stop.reason, StopReason::SatelliteLimit);
        assert_eq!(stop.phase, phase);
        assert_eq!(stop.at, config.start() + TimeDelta::seconds(300));
        assert_eq!(report.evaluations, 2);
        assert_eq!(budget.used(), 2);
        assert_eq!(records.len(), 1);
        let bracket = if rising {
            assert_eq!(records[0].end, PassEnd::Interrupted);
            let PassStart::Crossing(bracket) = records[0].start else {
                panic!("{records:?}")
            };
            bracket
        } else {
            assert_eq!(records[0].start, PassStart::InProgress);
            let PassEnd::Crossing(bracket) = records[0].end else {
                panic!("{records:?}")
            };
            bracket
        };
        assert!(!bracket.meets_tolerance());
        assert_eq!(bracket.lower(), config.start());
        assert_eq!(bracket.upper(), config.end());
        assert_bracket_sides(&satellite, observer, &config, bracket, rising);
    }
}

#[test]
fn propagation_failure_retains_open_pass_and_original_error() {
    let satellite = from_tle(
        "1 33333U 05037B   05333.02012661  .25992681  00000-0  24476-3 0  1532",
        "2 33333  96.4736 157.9986 9950000 244.0492 110.6523  4.00004038 10700",
    );
    let config = config(satellite.epoch() + TimeDelta::minutes(20), 300, -90.0, 300);
    let mut budget = EvaluationBudget::new(10);
    let (report, records) = collect(&satellite, site(0.0, 0.0, 0.0), &config, 10, &mut budget);
    let SearchStatus::Incomplete(stop) = report.status else {
        panic!("{report:?}")
    };
    assert_eq!(stop.at, config.end());
    assert_eq!(stop.phase, SearchPhase::Sampling);
    assert_eq!(
        stop.reason,
        StopReason::Evaluation(OrbitalEvaluationError::Propagation(PropagateError::Sgp4(
            sgp4::Error::NegativeSemiLatusRectum { t: 25.0 }
        )))
    );
    assert_eq!(report.evaluations, 2);
    assert_eq!(budget.used(), 2);
    assert_eq!(report.last_sample.unwrap().at, config.start());
    assert_eq!(
        records,
        [PredictedPass {
            start: PassStart::InProgress,
            end: PassEnd::Interrupted
        }]
    );

    // A failing satellite doesn't prevent a later caller using the shared budget.
    let next = iss();
    let (report, _) = collect(
        &next,
        site(0.0, 0.0, 0.0),
        &self::config(next.epoch(), 1, 10.0, 1),
        2,
        &mut budget,
    );
    assert!(report.is_complete());
    assert_eq!(budget.used(), 4);
}

#[test]
fn nonfinite_propagation_is_an_evaluation_failure_not_no_pass() {
    let mut elements: Vec<sgp4::Elements> =
        serde_json::from_str(include_str!("fixtures/iss-25544.json")).unwrap();
    elements[0].drag_term = 1e300;
    let satellite = Satellite::from_elements(&elements[0]).unwrap();
    let mut budget = EvaluationBudget::new(10);
    let (report, records) = collect(
        &satellite,
        site(0.0, 0.0, 0.0),
        &config(satellite.epoch(), 60, 10.0, 30),
        10,
        &mut budget,
    );
    let SearchStatus::Incomplete(stop) = report.status else {
        panic!("{report:?}")
    };
    assert_eq!(stop.at, satellite.epoch());
    assert_eq!(stop.phase, SearchPhase::Sampling);
    assert_eq!(
        stop.reason,
        StopReason::Evaluation(OrbitalEvaluationError::Propagation(
            PropagateError::NonFinite
        ))
    );
    assert_eq!(report.evaluations, 1);
    assert_eq!(budget.used(), 1);
    assert_eq!(report.last_sample, None);
    assert!(records.is_empty());
}

#[test]
fn invalid_observer_errors_are_budgeted_and_zero_allowance_stays_unsearched() {
    let satellite = iss();
    let observer = site(91.0, 0.0, 0.0);
    let config = config(satellite.epoch(), 60, 10.0, 30);
    for (local_limit, total_limit) in [(10, 10), (0, 10), (10, 0)] {
        let mut budget = EvaluationBudget::new(total_limit);
        let (report, records) = collect(&satellite, observer, &config, local_limit, &mut budget);
        assert!(records.is_empty());
        assert_eq!(report.last_sample, None);
        match report.status {
            SearchStatus::Incomplete(stop) => {
                assert_eq!((local_limit, total_limit), (10, 10));
                assert_eq!(stop.at, config.start());
                assert_eq!(stop.phase, SearchPhase::Sampling);
                assert_eq!(
                    stop.reason,
                    StopReason::Evaluation(OrbitalEvaluationError::Observation(
                        ObservationError::Coordinate(CoordinateError::InvalidLatitude)
                    ))
                );
                assert_eq!(report.evaluations, 1);
                assert_eq!(budget.used(), 1);
            }
            SearchStatus::Unsearched(stop) => {
                assert_eq!(
                    stop.reason,
                    if total_limit == 0 {
                        StopReason::TotalLimit
                    } else {
                        StopReason::SatelliteLimit
                    }
                );
                assert_eq!(report.evaluations, 0);
                assert_eq!(budget.used(), 0);
            }
            SearchStatus::Complete => panic!("invalid observer must not establish no-pass"),
        }
    }
}

#[test]
fn shared_budget_preserves_partial_orbit_and_leaves_next_unsearched() {
    let satellite = iss();
    let observer = site(0.0, -80.0, 0.0);
    let config = config(satellite.epoch(), 60, 10.0, 30);
    let mut budget = EvaluationBudget::new(1);
    let (report, records) = collect(&satellite, observer, &config, 100, &mut budget);
    assert_eq!(report.evaluations, 1);
    let SearchStatus::Incomplete(stop) = report.status else {
        panic!("{report:?}")
    };
    assert_eq!(stop.reason, StopReason::TotalLimit);
    assert_eq!(stop.at, config.start() + TimeDelta::seconds(30));
    assert_eq!(
        records,
        [PredictedPass {
            start: PassStart::InProgress,
            end: PassEnd::Interrupted
        }]
    );
    let (next, records) = collect(&satellite, observer, &config, 100, &mut budget);
    let SearchStatus::Unsearched(stop) = next.status else {
        panic!("{next:?}")
    };
    assert_eq!(stop.reason, StopReason::TotalLimit);
    assert_eq!(next.evaluations, 0);
    assert_eq!(budget.used(), 1);
    assert!(records.is_empty());
}
