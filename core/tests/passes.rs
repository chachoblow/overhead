//! Synthetic search contracts, not independent orbital/timing accuracy checks.
//! Intervals and allowances here are test inputs, not operating defaults.
use core::f64::consts::FRAC_PI_2;
use overhead_core::TimeError;
use overhead_core::passes::{
    CrossingBracket, DEFAULT_CROSSING_TOLERANCE, DEFAULT_LOOK_AHEAD, DEFAULT_MIN_ELEVATION_RAD,
    EvaluationBudget, PassEnd, PassStart, PredictedPass, SearchConfig, SearchConfigError,
    SearchPhase, SearchReport, SearchStatus, SearchStop, StopReason, search_elevations,
};
use overhead_core::sgp4::chrono::{NaiveDateTime, TimeDelta};

fn utc(text: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S%.f").unwrap()
}

fn origin() -> NaiveDateTime {
    utc("2026-10-08T00:00:00")
}

fn at(seconds: i64) -> NaiveDateTime {
    origin() + TimeDelta::seconds(seconds)
}

fn seconds(time: NaiveDateTime) -> f64 {
    (time - origin()).num_nanoseconds().unwrap() as f64 / 1e9
}

fn config(window: i64, step: i64, tolerance: i64) -> SearchConfig {
    SearchConfig::new(
        origin(),
        TimeDelta::seconds(window),
        0.0,
        TimeDelta::seconds(step),
        TimeDelta::seconds(tolerance),
    )
    .unwrap()
}

#[derive(Debug, PartialEq)]
struct Run {
    passes: Vec<PredictedPass>,
    report: SearchReport<&'static str>,
    calls: Vec<NaiveDateTime>,
}

fn run(
    config: &SearchConfig,
    allowance: u64,
    budget: &mut EvaluationBudget,
    mut elevation: impl FnMut(NaiveDateTime) -> Result<f64, &'static str>,
) -> Run {
    let mut passes = Vec::new();
    let mut calls = Vec::new();
    let report = search_elevations(
        config,
        allowance,
        budget,
        |time| {
            calls.push(time);
            elevation(time)
        },
        |pass| passes.push(pass),
    );
    assert_eq!(report.evaluations, calls.len() as u64);
    assert_eq!(report.passes, passes.len() as u64);
    assert_eq!(
        report.upcoming_passes,
        passes
            .iter()
            .filter(|pass| matches!(pass.start, PassStart::Crossing(_)))
            .count() as u64,
    );
    Run {
        passes,
        report,
        calls,
    }
}

fn curve(config: &SearchConfig, mut elevation: impl FnMut(f64) -> f64) -> Run {
    let result = run(config, 100_000, &mut EvaluationBudget::new(100_000), |t| {
        Ok(elevation(seconds(t)))
    });
    assert!(result.report.is_complete(), "{:?}", result.report);
    assert_eq!(result.report.last_sample.unwrap().at, config.end());
    result
}

fn triangle(t: f64, rise: f64, fall: f64) -> f64 {
    ((t - rise).min(fall - t) * 0.01).clamp(-1.0, 1.0)
}

fn rising(pass: PredictedPass) -> CrossingBracket {
    match pass.start {
        PassStart::Crossing(bracket) => bracket,
        _ => panic!("expected arrival: {pass:?}"),
    }
}

fn falling(pass: PredictedPass) -> CrossingBracket {
    match pass.end {
        PassEnd::Crossing(bracket) => bracket,
        _ => panic!("expected departure: {pass:?}"),
    }
}

fn encloses(bracket: CrossingBracket, time: NaiveDateTime, tolerance: TimeDelta) {
    assert!(
        bracket.lower() <= time && time <= bracket.upper(),
        "{bracket:?}, {time}"
    );
    assert!(bracket.lower() < bracket.upper());
    assert!(bracket.width() <= tolerance, "{bracket:?}");
    assert!(bracket.meets_tolerance());
}

fn stopped(result: &Run) -> &SearchStop<&'static str> {
    match &result.report.status {
        SearchStatus::Incomplete(stop) | SearchStatus::Unsearched(stop) => stop,
        SearchStatus::Complete => panic!("expected interruption: {result:?}"),
    }
}

#[test]
fn defaults_require_explicit_detection_and_preserve_accepted_values() {
    let step = TimeDelta::seconds(17);
    let config = SearchConfig::with_defaults(origin(), step).unwrap();
    assert_eq!(config.start(), origin());
    assert_eq!(config.end(), at(86_400));
    assert_eq!(config.look_ahead(), DEFAULT_LOOK_AHEAD);
    assert_eq!(config.detection_interval(), step);
    assert_eq!(config.crossing_tolerance(), DEFAULT_CROSSING_TOLERANCE);
    assert_eq!(config.min_elevation_rad(), DEFAULT_MIN_ELEVATION_RAD);
    assert!((config.min_elevation_rad().to_degrees() - 10.0).abs() < 1e-12);
}

#[test]
fn configuration_rejects_invalid_thresholds_and_nonpositive_durations() {
    let one = TimeDelta::seconds(1);
    for threshold in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.6, 1.6] {
        assert_eq!(
            SearchConfig::new(origin(), one, threshold, one, one),
            Err(SearchConfigError::InvalidMinimumElevation),
        );
    }
    for duration in [TimeDelta::zero(), TimeDelta::nanoseconds(-1)] {
        assert_eq!(
            SearchConfig::new(origin(), duration, 0.0, one, one),
            Err(SearchConfigError::NonPositiveLookAhead),
        );
        assert_eq!(
            SearchConfig::new(origin(), one, 0.0, duration, one),
            Err(SearchConfigError::NonPositiveDetectionInterval),
        );
        assert_eq!(
            SearchConfig::new(origin(), one, 0.0, one, duration),
            Err(SearchConfigError::NonPositiveCrossingTolerance),
        );
    }
    for threshold in [-FRAC_PI_2, -0.1, -0.0, 0.0, FRAC_PI_2] {
        assert!(SearchConfig::new(origin(), one, threshold, one, one).is_ok());
    }
}

#[test]
fn configuration_validates_start_and_checked_end_including_fractional_utc() {
    for (text, error) in [
        ("1956-12-31T23:59:59", TimeError::UnsupportedYear(1956)),
        ("2101-01-01T00:00:00", TimeError::UnsupportedYear(2101)),
        ("2016-12-31T23:59:60", TimeError::LeapSecond),
        ("2016-12-31T23:59:60.5", TimeError::LeapSecond),
    ] {
        assert_eq!(
            SearchConfig::with_defaults(utc(text), TimeDelta::seconds(1)),
            Err(SearchConfigError::StartTime(error)),
        );
    }
    let end = utc("2100-12-31T23:59:59.999999999");
    let ns = TimeDelta::nanoseconds(1);
    let valid = SearchConfig::new(end - ns, ns, 0.0, ns, ns).unwrap();
    assert_eq!(valid.end(), end);
    assert_eq!(
        SearchConfig::new(end, ns, 0.0, ns, ns),
        Err(SearchConfigError::EndTime(TimeError::UnsupportedYear(2101))),
    );
    assert_eq!(
        SearchConfig::new(origin(), TimeDelta::MAX, 0.0, ns, ns),
        Err(SearchConfigError::EndTimeOutOfRange),
    );
    let across_leap_date = SearchConfig::new(
        utc("2016-12-31T23:59:59.5"),
        TimeDelta::seconds(1),
        0.0,
        ns,
        ns,
    )
    .unwrap();
    assert_eq!(across_leap_date.end(), utc("2017-01-01T00:00:00.5"));
}

#[test]
fn exact_grid_endpoints_and_large_intervals_do_not_overflow() {
    for (window, step, expected) in [
        (10, 3, vec![0, 3, 6, 9, 10]),
        (10, 5, vec![0, 5, 10]),
        (10, 10, vec![0, 10]),
        (10, 11, vec![0, 10]),
    ] {
        let result = curve(&config(window, step, 1), |_| -0.1);
        assert_eq!(
            result.calls,
            expected.into_iter().map(at).collect::<Vec<_>>()
        );
        assert!(result.passes.is_empty());
    }
    let config = SearchConfig::new(
        origin(),
        TimeDelta::seconds(10),
        0.0,
        TimeDelta::MAX,
        TimeDelta::MAX,
    )
    .unwrap();
    let result = curve(&config, |t| (t - 4.0) * 0.01);
    assert_eq!(result.calls, vec![at(0), at(10)]);
    encloses(rising(result.passes[0]), at(4), TimeDelta::MAX);
}

#[test]
fn fractional_grid_has_no_drift_and_samples_end_only_once() {
    let start = utc("2026-10-08T00:00:00.999999999");
    let config = SearchConfig::new(
        start,
        TimeDelta::nanoseconds(109),
        0.0,
        TimeDelta::nanoseconds(17),
        TimeDelta::nanoseconds(1),
    )
    .unwrap();
    let result = curve(&config, |_| -0.1);
    let expected = [0, 17, 34, 51, 68, 85, 102, 109].map(|ns| start + TimeDelta::nanoseconds(ns));
    assert_eq!(result.calls, expected);
}

#[test]
fn ordinary_passes_have_refined_crossings_and_do_not_move_the_grid() {
    let config = config(100, 10, 1);
    let result = curve(&config, |t| {
        triangle(t, 13.0, 27.0).max(triangle(t, 63.0, 77.0))
    });
    assert_eq!(result.passes.len(), 2);
    for (pass, (rise, fall)) in result.passes.iter().zip([(13, 27), (63, 77)]) {
        encloses(rising(*pass), at(rise), config.crossing_tolerance());
        encloses(falling(*pass), at(fall), config.crossing_tolerance());
    }
    assert_eq!(result.report.upcoming_passes, 2);
    let grid: Vec<_> = result
        .calls
        .iter()
        .copied()
        .filter(|t| (*t - origin()).num_nanoseconds().unwrap() % 10_000_000_000 == 0)
        .collect();
    assert_eq!(grid, (0..=10).map(|i| at(i * 10)).collect::<Vec<_>>());
    assert_eq!(result.report.evaluations, 11 + 4 * 4); // Four halvings per crossing.
}

#[test]
fn already_above_is_not_an_arrival_and_can_end_before_a_later_arrival() {
    let config = config(40, 5, 1);
    let result = curve(&config, |t| ((8.0 - t) * 0.01).max(triangle(t, 22.0, 33.0)));
    assert_eq!(result.passes.len(), 2);
    assert_eq!(result.passes[0].start, PassStart::InProgress);
    encloses(
        falling(result.passes[0]),
        at(8),
        config.crossing_tolerance(),
    );
    encloses(
        rising(result.passes[1]),
        at(22),
        config.crossing_tolerance(),
    );
    encloses(
        falling(result.passes[1]),
        at(33),
        config.crossing_tolerance(),
    );
    assert_eq!(result.report.upcoming_passes, 1);
}

#[test]
fn complete_window_spanning_and_no_arrival_results_are_distinct() {
    let config = config(20, 5, 1);
    let above = curve(&config, |_| 0.1);
    assert_eq!(
        above.passes,
        vec![PredictedPass {
            start: PassStart::InProgress,
            end: PassEnd::BeyondWindow,
        }]
    );
    assert_eq!(above.report.upcoming_passes, 0);
    let below = curve(&config, |_| -0.1);
    assert!(below.passes.is_empty());
    assert_eq!(below.report.upcoming_passes, 0);
    let arrival = curve(&config, |t| (t - 7.0) * 0.01);
    encloses(
        rising(arrival.passes[0]),
        at(7),
        config.crossing_tolerance(),
    );
    assert_eq!(arrival.passes[0].end, PassEnd::BeyondWindow);
}

#[test]
fn regular_equality_runs_touches_and_window_boundaries_follow_the_contract() {
    // A large tolerance leaves strict-side brackets intact for classification tests.
    type ExpectedPass = Option<(&'static str, &'static str)>;
    let cases: &[(&[f64], ExpectedPass)] = &[
        (&[-0.1, 0.0, 0.1], Some(("crossing", "beyond"))),
        (&[0.1, 0.0, -0.1], Some(("in-progress", "crossing"))),
        (&[-0.1, 0.0, -0.1], None),
        (&[0.1, 0.0, 0.1], Some(("in-progress", "beyond"))),
        (&[0.0, 0.1], Some(("boundary", "beyond"))),
        (&[0.0, 0.0, 0.1], Some(("boundary", "beyond"))),
        (&[0.0, -0.1, 0.1], Some(("crossing", "beyond"))),
        (&[0.0, 0.1, -0.1], Some(("boundary", "crossing"))),
        (&[-0.1, 0.0], None),
        (&[0.1, 0.0], Some(("in-progress", "uncertain"))),
        (&[0.0, 0.0, 0.0], None),
        (&[-0.1, 0.0, 0.0, -0.1], None),
        (&[-0.1, 0.0, 0.0, 0.1], Some(("crossing", "beyond"))),
        (&[0.1, 0.0, 0.0, 0.1], Some(("in-progress", "beyond"))),
        (&[0.1, 0.0, 0.0, -0.1], Some(("in-progress", "crossing"))),
        (&[0.1, 0.0, 0.0, 0.0], Some(("in-progress", "uncertain"))),
        (&[0.0, 0.1, 0.0], Some(("boundary", "uncertain"))),
    ];
    for &(samples, expected) in cases {
        let config = config(samples.len() as i64 - 1, 1, samples.len() as i64);
        let result = curve(&config, |t| samples[t as usize]);
        assert_eq!(
            result.passes.len(),
            usize::from(expected.is_some()),
            "{samples:?}"
        );
        if let Some((start, end)) = expected {
            let pass = result.passes[0];
            let actual_start = match pass.start {
                PassStart::InProgress => "in-progress",
                PassStart::BoundaryStart => "boundary",
                PassStart::Crossing(_) => "crossing",
            };
            let actual_end = match pass.end {
                PassEnd::Crossing(_) => "crossing",
                PassEnd::BeyondWindow => "beyond",
                PassEnd::BoundaryUncertain => "uncertain",
                PassEnd::Interrupted => panic!("complete search interrupted"),
            };
            assert_eq!((actual_start, actual_end), (start, end), "{samples:?}");
        }
    }
}

#[test]
fn equality_plateaus_refine_to_the_strictly_above_interval_boundary() {
    let config = config(10, 2, 1);
    for rising_direction in [true, false] {
        let result = curve(&config, |t| {
            let value = if t < 3.0 {
                t - 3.0
            } else if t > 7.0 {
                t - 7.0
            } else {
                0.0
            };
            value * if rising_direction { 0.01 } else { -0.01 }
        });
        assert_eq!(result.passes.len(), 1);
        let (bracket, boundary) = if rising_direction {
            (rising(result.passes[0]), 7)
        } else {
            (falling(result.passes[0]), 3)
        };
        encloses(bracket, at(boundary), config.crossing_tolerance());
    }
}

#[test]
fn angular_comparison_has_no_hidden_epsilon_and_accepts_geometric_extremes() {
    let threshold = 0.5_f64;
    let config = SearchConfig::new(
        origin(),
        TimeDelta::seconds(2),
        threshold,
        TimeDelta::seconds(1),
        TimeDelta::seconds(2),
    )
    .unwrap();
    let values = [
        f64::from_bits(threshold.to_bits() - 1),
        threshold,
        f64::from_bits(threshold.to_bits() + 1),
    ];
    let result = curve(&config, |t| values[t as usize]);
    assert_eq!(result.report.upcoming_passes, 1);
    assert_eq!(rising(result.passes[0]).lower(), at(0));
    assert_eq!(rising(result.passes[0]).upper(), at(2));
    for value in [-FRAC_PI_2, FRAC_PI_2] {
        let result = curve(&config, |_| value);
        assert_eq!(result.passes.len(), usize::from(value > threshold));
    }
    for threshold in [-FRAC_PI_2, FRAC_PI_2] {
        let config = SearchConfig::new(
            origin(),
            TimeDelta::seconds(2),
            threshold,
            TimeDelta::seconds(1),
            TimeDelta::seconds(1),
        )
        .unwrap();
        assert!(curve(&config, |_| threshold).passes.is_empty());
    }
}

#[test]
fn detected_short_passes_survive_even_when_shorter_than_tolerance() {
    let config = config(20, 10, 5);
    let result = curve(&config, |t| triangle(t, 9.0, 11.0));
    assert_eq!(result.passes.len(), 1);
    encloses(rising(result.passes[0]), at(9), config.crossing_tolerance());
    encloses(
        falling(result.passes[0]),
        at(11),
        config.crossing_tolerance(),
    );
}

#[test]
fn missed_excursions_and_gaps_are_detection_limits_not_duration_filters() {
    let coarse = config(20, 10, 1);
    let fine = config(20, 1, 1);
    let excursion = |t| triangle(t, 4.0, 6.0);
    assert!(curve(&coarse, excursion).passes.is_empty());
    assert_eq!(curve(&fine, excursion).passes.len(), 1);
    let gap = |t| -triangle(t, 4.0, 6.0);
    let merged = curve(&coarse, gap);
    assert_eq!(
        merged.passes,
        vec![PredictedPass {
            start: PassStart::InProgress,
            end: PassEnd::BeyondWindow,
        }]
    );
    let split = curve(&fine, gap);
    assert_eq!(split.passes.len(), 2);
    assert_eq!(split.report.upcoming_passes, 1);
}

#[test]
fn detection_interval_sweep_is_independent_of_crossing_tolerance() {
    for (step, detected) in [(1, 1), (2, 0), (5, 1), (10, 0)] {
        for tolerance in [
            TimeDelta::nanoseconds(1),
            TimeDelta::seconds(1),
            TimeDelta::seconds(10),
        ] {
            let config = SearchConfig::new(
                origin(),
                TimeDelta::seconds(20),
                0.0,
                TimeDelta::seconds(step),
                tolerance,
            )
            .unwrap();
            let result = curve(&config, |t| triangle(t, 4.0, 6.0));
            assert_eq!(result.report.upcoming_passes, detected);
            for pass in result.passes {
                encloses(rising(pass), at(4), tolerance);
                encloses(falling(pass), at(6), tolerance);
            }
        }
    }
}

#[test]
fn refinement_does_not_prove_unique_crossings_or_earliest_physical_arrival() {
    let config = config(10, 10, 1);
    // Three crossings at 2, 3 and 8; the short first pass is inside one bracket.
    let result = curve(&config, |t| (t - 2.0) * (t - 3.0) * (t - 8.0) / 1000.0);
    assert_eq!(result.passes.len(), 1);
    encloses(rising(result.passes[0]), at(8), config.crossing_tolerance());
    assert!(rising(result.passes[0]).lower() > at(2));
}

#[test]
fn refinement_terminates_at_nanosecond_precision_in_both_directions() {
    let config = SearchConfig::new(
        origin(),
        TimeDelta::nanoseconds(3),
        0.0,
        TimeDelta::nanoseconds(3),
        TimeDelta::nanoseconds(1),
    )
    .unwrap();
    for direction in [-1.0, 1.0] {
        let result = curve(&config, |t| (t * 1e9 - 1.5) * direction * 0.1);
        let bracket = if direction > 0.0 {
            rising(result.passes[0])
        } else {
            falling(result.passes[0])
        };
        assert_eq!(bracket.lower(), origin() + TimeDelta::nanoseconds(1));
        assert_eq!(bracket.upper(), origin() + TimeDelta::nanoseconds(2));
        assert!(bracket.meets_tolerance());
        assert_eq!(result.report.evaluations, 4);
    }
}

#[test]
fn full_supported_era_bracket_can_be_refined_without_duration_overflow() {
    let start = utc("1957-01-01T00:00:00");
    let end = utc("2100-12-31T23:59:59.999999999");
    let crossing = utc("2000-01-01T00:00:00");
    let config = SearchConfig::new(
        start,
        end - start,
        0.0,
        TimeDelta::MAX,
        TimeDelta::nanoseconds(1),
    )
    .unwrap();
    // A step tests integer arithmetic across the full era, not orbital physics.
    let result = run(&config, 100, &mut EvaluationBudget::new(100), |t| {
        Ok(if t < crossing { -0.1 } else { 0.1 })
    });
    assert!(result.report.is_complete());
    encloses(
        rising(result.passes[0]),
        crossing,
        config.crossing_tolerance(),
    );
    assert_eq!(result.report.last_sample.unwrap().at, end);
}

#[test]
fn zero_allowances_are_unsearched_and_denied_calls_are_not_evaluated() {
    for (satellite, total, reason) in [
        (0, 10, StopReason::SatelliteLimit),
        (10, 0, StopReason::TotalLimit),
        (0, 0, StopReason::TotalLimit),
    ] {
        let mut budget = EvaluationBudget::new(total);
        let result = run(&config(10, 5, 1), satellite, &mut budget, |_| {
            panic!("denied call ran")
        });
        assert!(matches!(result.report.status, SearchStatus::Unsearched(_)));
        assert_eq!(result.report.last_sample, None);
        assert!(result.passes.is_empty());
        assert_eq!(
            stopped(&result),
            &SearchStop {
                at: origin(),
                phase: SearchPhase::Sampling,
                reason,
            }
        );
        assert_eq!(budget.used(), 0);
        assert_eq!(budget.limit(), total);
        assert_eq!(budget.remaining(), total);
    }
}

#[test]
fn exact_fit_is_complete_but_one_less_is_incomplete_even_without_arrivals() {
    let config = config(10, 5, 1);
    for (allowance, complete) in [(3, true), (2, false)] {
        let mut budget = EvaluationBudget::new(allowance);
        let result = run(&config, allowance, &mut budget, |_| Ok(-0.1));
        assert_eq!(result.report.is_complete(), complete);
        assert_eq!(budget.used(), allowance);
        assert_eq!(budget.remaining(), 0);
        assert!(result.passes.is_empty());
        if !complete {
            assert_eq!(stopped(&result).at, at(10));
            assert_eq!(result.report.last_sample.unwrap().at, at(5));
        }
    }
    // Same rule includes every required refinement, not just regular samples.
    let expected = curve(&config, |t| (t - 4.0) * 0.01);
    let allowance = expected.report.evaluations;
    let mut budget = EvaluationBudget::new(allowance);
    let exact = run(&config, allowance, &mut budget, |t| {
        Ok((seconds(t) - 4.0) * 0.01)
    });
    assert_eq!(exact, expected);
}

#[test]
fn first_evaluation_failure_is_incomplete_not_unsearched_or_complete_no_pass() {
    let mut budget = EvaluationBudget::new(10);
    let result = run(&config(10, 5, 1), 10, &mut budget, |_| {
        Err("failed at start")
    });
    assert!(matches!(result.report.status, SearchStatus::Incomplete(_)));
    assert_eq!(result.report.evaluations, 1);
    assert_eq!(budget.used(), 1);
    assert!(result.passes.is_empty());
    assert_eq!(result.report.last_sample, None);
    assert_eq!(
        stopped(&result),
        &SearchStop {
            at: origin(),
            phase: SearchPhase::Sampling,
            reason: StopReason::Evaluation("failed at start"),
        }
    );
}

#[test]
fn nonfinite_and_out_of_range_evaluations_are_failed_attempts() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.6, 1.6] {
        let mut budget = EvaluationBudget::new(10);
        let result = run(&config(10, 5, 1), 10, &mut budget, |_| Ok(invalid));
        assert!(matches!(result.report.status, SearchStatus::Incomplete(_)));
        assert_eq!(result.report.evaluations, 1);
        assert_eq!(budget.used(), 1);
        let StopReason::InvalidElevation(value) = stopped(&result).reason else {
            panic!("wrong reason: {result:?}");
        };
        assert_eq!(value.to_bits(), invalid.to_bits());
    }
}

#[test]
fn sampling_failure_preserves_completed_and_open_passes() {
    let config = config(30, 5, 5);
    for fail_at in [15, 25] {
        let result = run(&config, 100, &mut EvaluationBudget::new(100), |t| {
            if t == at(fail_at) {
                return Err("sampling failure");
            }
            Ok(triangle(seconds(t), 2.0, 8.0).max(triangle(seconds(t), 17.0, 23.0)))
        });
        assert_eq!(result.passes.len(), if fail_at == 15 { 1 } else { 2 });
        encloses(rising(result.passes[0]), at(2), config.crossing_tolerance());
        encloses(
            falling(result.passes[0]),
            at(8),
            config.crossing_tolerance(),
        );
        if fail_at == 25 {
            encloses(
                rising(result.passes[1]),
                at(17),
                config.crossing_tolerance(),
            );
            assert_eq!(result.passes[1].end, PassEnd::Interrupted);
        }
        assert_eq!(
            stopped(&result),
            &SearchStop {
                at: at(fail_at),
                phase: SearchPhase::Sampling,
                reason: StopReason::Evaluation("sampling failure"),
            }
        );
        assert_eq!(result.report.last_sample.unwrap().at, at(fail_at - 5));
    }
}

#[test]
fn interrupted_boundary_and_in_progress_passes_do_not_invent_arrivals_or_ends() {
    let config = config(20, 5, 1);
    for starts_equal in [false, true] {
        let result = run(&config, 100, &mut EvaluationBudget::new(100), |t| {
            if t == at(10) {
                Err("failed")
            } else if starts_equal && t == origin() {
                Ok(0.0)
            } else {
                Ok(0.1)
            }
        });
        assert_eq!(
            result.passes,
            vec![PredictedPass {
                start: if starts_equal {
                    PassStart::BoundaryStart
                } else {
                    PassStart::InProgress
                },
                end: PassEnd::Interrupted,
            }]
        );
        assert_eq!(result.report.upcoming_passes, 0);
    }
}

#[test]
fn refinement_failure_retains_narrowed_coarse_brackets_in_both_directions() {
    for direction in [-1.0, 1.0] {
        let config = config(10, 10, 1);
        let mut budget = EvaluationBudget::new(100);
        let result = run(&config, 100, &mut budget, |t| {
            if seconds(t) == 2.5 {
                Err("refinement failure")
            } else {
                Ok((seconds(t) - 4.0) * direction * 0.01)
            }
        });
        let pass = result.passes[0];
        let bracket = if direction > 0.0 {
            assert_eq!(pass.end, PassEnd::Interrupted);
            rising(pass)
        } else {
            assert_eq!(pass.start, PassStart::InProgress);
            falling(pass)
        };
        assert_eq!(bracket.lower(), at(0));
        assert_eq!(bracket.upper(), at(5));
        assert!(!bracket.meets_tolerance());
        assert_eq!(result.report.evaluations, 4);
        assert_eq!(budget.used(), 4);
        // A last sample at the exact end must not imply successful completion.
        assert_eq!(result.report.last_sample.unwrap().at, config.end());
        assert!(!result.report.is_complete());
        assert_eq!(
            stopped(&result),
            &SearchStop {
                at: origin() + TimeDelta::milliseconds(2500),
                phase: if direction > 0.0 {
                    SearchPhase::RisingRefinement
                } else {
                    SearchPhase::FallingRefinement
                },
                reason: StopReason::Evaluation("refinement failure"),
            }
        );
    }
}

#[test]
fn budget_exhaustion_during_refinement_preserves_brackets_without_running_denied_calls() {
    for direction in [-1.0, 1.0] {
        for total_exhaustion in [false, true] {
            let (satellite_limit, total_limit) = if total_exhaustion { (10, 3) } else { (3, 10) };
            let mut budget = EvaluationBudget::new(total_limit);
            let result = run(&config(10, 10, 1), satellite_limit, &mut budget, |t| {
                Ok((seconds(t) - 4.0) * direction * 0.01)
            });
            let bracket = if direction > 0.0 {
                rising(result.passes[0])
            } else {
                falling(result.passes[0])
            };
            assert_eq!((bracket.lower(), bracket.upper()), (at(0), at(5)));
            assert!(!bracket.meets_tolerance());
            assert_eq!(result.calls, vec![at(0), at(10), at(5)]);
            assert_eq!(budget.used(), 3);
            assert_eq!(
                stopped(&result),
                &SearchStop {
                    at: origin() + TimeDelta::milliseconds(2500),
                    phase: if direction > 0.0 {
                        SearchPhase::RisingRefinement
                    } else {
                        SearchPhase::FallingRefinement
                    },
                    reason: if total_exhaustion {
                        StopReason::TotalLimit
                    } else {
                        StopReason::SatelliteLimit
                    },
                }
            );
        }
    }
}

#[test]
fn invalid_midpoint_preserves_the_original_detected_transition() {
    for direction in [-1.0, 1.0] {
        let result = run(
            &config(10, 10, 1),
            100,
            &mut EvaluationBudget::new(100),
            |t| {
                Ok(if t == at(5) {
                    f64::INFINITY
                } else {
                    (seconds(t) - 4.0) * direction * 0.01
                })
            },
        );
        let bracket = if direction > 0.0 {
            rising(result.passes[0])
        } else {
            falling(result.passes[0])
        };
        assert_eq!((bracket.lower(), bracket.upper()), (at(0), at(10)));
        assert!(!bracket.meets_tolerance());
        assert_eq!(result.report.evaluations, 3);
        assert_eq!(
            stopped(&result).reason,
            StopReason::InvalidElevation(f64::INFINITY)
        );
    }
}

#[test]
fn shared_budget_allows_later_searches_after_local_failure_or_exhaustion() {
    let config = config(10, 5, 1);
    let mut budget = EvaluationBudget::new(10);
    let failed = run(&config, 10, &mut budget, |_| Err("failed satellite"));
    assert!(!failed.report.is_complete());
    let exhausted = run(&config, 2, &mut budget, |_| Ok(0.1));
    assert_eq!(stopped(&exhausted).reason, StopReason::SatelliteLimit);
    assert_eq!(exhausted.passes[0].end, PassEnd::Interrupted);
    let successful = run(&config, 10, &mut budget, |_| Ok(0.1));
    assert!(successful.report.is_complete());
    assert_eq!(successful.passes[0].end, PassEnd::BeyondWindow);
    assert_eq!(budget.used(), 1 + 2 + 3);
    assert_eq!(budget.remaining(), 4);
}

#[test]
fn exhausted_shared_budget_identifies_partial_then_unsearched_satellites() {
    let config = config(10, 5, 1);
    let mut budget = EvaluationBudget::new(5);
    let first = run(&config, 10, &mut budget, |_| Ok(-0.1));
    let second = run(&config, 10, &mut budget, |_| Ok(-0.1));
    let third = run(&config, 10, &mut budget, |_| {
        panic!("global budget exhausted")
    });
    assert!(first.report.is_complete());
    assert!(matches!(second.report.status, SearchStatus::Incomplete(_)));
    assert_eq!(stopped(&second).reason, StopReason::TotalLimit);
    assert!(matches!(third.report.status, SearchStatus::Unsearched(_)));
    assert_eq!(stopped(&third).reason, StopReason::TotalLimit);
    assert_eq!(budget.used(), 5);
    assert_eq!(budget.remaining(), 0);
}

#[test]
fn every_evaluation_can_fail_or_be_budget_denied_without_losing_established_events() {
    let config = config(40, 5, 1);
    let elevation = |t| triangle(t, 2.0, 13.0).max(triangle(t, 22.0, 33.0));
    let complete = curve(&config, elevation);
    for cutoff in 0..complete.calls.len() {
        for fail_instead_of_deny in [false, true] {
            let mut attempts = 0;
            let allowance = if fail_instead_of_deny {
                100
            } else {
                cutoff as u64
            };
            let result = run(&config, allowance, &mut EvaluationBudget::new(100), |t| {
                let fail = fail_instead_of_deny && attempts == cutoff;
                attempts += 1;
                if fail {
                    Err("injected failure")
                } else {
                    Ok(elevation(seconds(t)))
                }
            });
            assert!(!result.report.is_complete());
            let attempted = cutoff + usize::from(fail_instead_of_deny);
            assert_eq!(result.calls, complete.calls[..attempted]);
            assert_eq!(stopped(&result).at, complete.calls[cutoff]);
            // These grid samples establish each arrival before refinement runs.
            let established = [at(5), at(25)]
                .iter()
                .filter(|sample| complete.calls[..cutoff].contains(sample))
                .count();
            assert_eq!(result.passes.len(), established);
            for (index, pass) in result.passes.iter().enumerate() {
                let (rise, fall) = [(2, 13), (22, 33)][index];
                let start = rising(*pass);
                assert!(start.lower() <= at(rise) && at(rise) <= start.upper());
                assert_eq!(
                    start.meets_tolerance(),
                    start.width() <= config.crossing_tolerance()
                );
                if let PassEnd::Crossing(end) = pass.end {
                    assert!(end.lower() <= at(fall) && at(fall) <= end.upper());
                    assert_eq!(
                        end.meets_tolerance(),
                        end.width() <= config.crossing_tolerance()
                    );
                } else {
                    assert_eq!(pass.end, PassEnd::Interrupted);
                    assert_eq!(index + 1, result.passes.len());
                }
            }
        }
    }
}

#[test]
fn repeated_searches_produce_identical_records_and_evaluation_traces() {
    let config = config(100, 5, 1);
    let elevation = |t| triangle(t, 7.0, 18.0).max(triangle(t, 33.0, 72.0));
    assert_eq!(curve(&config, elevation), curve(&config, elevation));
}
