//! Fixed offline experiment, not an independent timing oracle or operating policy.
use std::{env, process::ExitCode};

use overhead_core::{
    GeodeticPosition, Satellite, ecef_to_look_angles,
    passes::{EvaluationBudget, PassEnd, PassStart, SearchConfig, search_satellite},
    sgp4::{
        Elements,
        chrono::{NaiveDateTime, TimeDelta},
    },
};
use serde::Serialize;

const USAGE: &str = "Usage: overhead-evaluate-intervals\n\nRun the fixed historical orbital detection-interval experiment and print JSON.\nNo inputs, network, clock, hardware, or production defaults. See tools/README.md.\nExit 0: experiment finished (detection misses are data); 1: invalid arguments or\nfailed/incomplete evaluation. --help/-h prints this help.";
const INTERVALS_MS: [i64; 6] = [1000, 5000, 10000, 30000, 60000, 120000];
const TOLERANCES_MS: [i64; 2] = [250, 5000];
// Deliberately ample experimental guard, NOT a proposed operating allowance.
const SEARCH_LIMIT: u64 = 200_000;

type Result<T> = std::result::Result<T, String>;

fn site(lat: f64, lon: f64, height: f64) -> GeodeticPosition {
    GeodeticPosition {
        latitude_rad: lat.to_radians(),
        longitude_rad: lon.to_radians(),
        altitude_km: height,
    }
}

struct Orbit {
    name: &'static str,
    class: &'static str,
    satellite: Satellite,
}

fn orbits() -> Result<Vec<Orbit>> {
    let elements: Vec<Elements> =
        serde_json::from_str(include_str!("../../../core/tests/fixtures/iss-25544.json"))
            .map_err(|e| e.to_string())?;
    let mut result = vec![Orbit {
        name: "ISS",
        class: "LEO",
        satellite: Satellite::from_elements(&elements[0]).map_err(|e| e.to_string())?,
    }];
    let lines: Vec<_> = include_str!("../../fixtures/pass-intervals.tle")
        .lines()
        .collect();
    let (records, remainder) = lines.as_chunks::<3>();
    if records.len() != 3 || !remainder.is_empty() {
        return Err("expected three named TLE records in the interval fixture".into());
    }
    for (lines, class) in records.iter().zip(["resonant-HEO", "GEO", "GNSS"]) {
        let elements = Elements::from_tle(None, lines[1].as_bytes(), lines[2].as_bytes())
            .map_err(|e| e.to_string())?;
        result.push(Orbit {
            name: lines[0],
            class,
            satellite: Satellite::from_elements(&elements).map_err(|e| e.to_string())?,
        });
    }
    Ok(result)
}

struct Case<'a> {
    name: String,
    orbit: &'a Orbit,
    observer: GeodeticPosition,
    start: NaiveDateTime,
    duration_ms: i64,
    threshold: f64,
    fine_ms: i64,
    construction: String,
}

fn elevation(orbit: &Orbit, observer: GeodeticPosition, at: NaiveDateTime) -> Result<f64> {
    let state = orbit
        .satellite
        .state_at(at)
        .map_err(|e| format!("{at}: {e}"))?;
    let ecef = state.to_ecef().map_err(|e| format!("{at}: {e}"))?;
    let angles = ecef_to_look_angles(ecef, observer).map_err(|e| format!("{at}: {e}"))?;
    Ok(angles.elevation_rad)
}

// Find an interior sampled extremum, then narrow its local 120-second
// neighborhood. This constructs adversarial inputs; it is not a pass oracle.
fn extremum(orbit: &Orbit, observer: GeodeticPosition, peak: bool) -> Result<NaiveDateTime> {
    let start = orbit.satellite.epoch() - TimeDelta::hours(12);
    let mut values = Vec::new();
    for i in 0..=1440 {
        values.push(elevation(
            orbit,
            observer,
            start + TimeDelta::seconds(i * 60),
        )?);
    }
    let score = |v: f64| if peak { v } else { -v };
    let index = (1..values.len() - 1)
        .filter(|&i| {
            score(values[i]) > score(values[i - 1]) && score(values[i]) > score(values[i + 1])
        })
        .max_by(|&a, &b| score(values[a]).total_cmp(&score(values[b])))
        .ok_or("no interior extremum in construction window")?;
    let mut lo = (index as i64 - 1) * 60_000;
    let mut hi = (index as i64 + 1) * 60_000;
    while hi - lo > 2 {
        let a = lo + (hi - lo) / 3;
        let b = hi - (hi - lo) / 3;
        if score(elevation(
            orbit,
            observer,
            start + TimeDelta::milliseconds(a),
        )?) < score(elevation(
            orbit,
            observer,
            start + TimeDelta::milliseconds(b),
        )?) {
            lo = a;
        } else {
            hi = b;
        }
    }
    Ok(start + TimeDelta::milliseconds((lo + hi) / 2))
}

fn cases(orbits: &[Orbit]) -> Result<Vec<Case<'_>>> {
    let mut result = Vec::new();
    for orbit in orbits {
        for (name, observer) in [
            ("colorado", site(39.007, -104.883, 2.187)),
            ("sydney", site(-33.8688, 151.2093, 0.058)),
            ("equator-80E", site(0.0, 80.0, 0.0)),
        ] {
            result.push(Case {
                name: format!("{}-{name}-10deg", orbit.class),
                orbit,
                observer,
                start: orbit.satellite.epoch() - TimeDelta::hours(12),
                duration_ms: 86_400_000,
                threshold: 10_f64.to_radians(),
                fine_ms: 500,
                construction: "fixed 10-degree threshold; epoch -12h to +12h".into(),
            });
        }
    }
    for (orbit, observer, peak, label) in [
        (
            &orbits[0],
            site(0.0, -80.0, 0.0),
            true,
            "LEO-short-excursion",
        ),
        (&orbits[2], site(39.007, 80.0, 0.0), false, "GEO-short-gap"),
    ] {
        let at = extremum(orbit, observer, peak)?;
        let threshold = (elevation(orbit, observer, at - TimeDelta::seconds(2))?
            + elevation(orbit, observer, at + TimeDelta::seconds(2))?)
            / 2.0;
        result.push(Case {
            name: label.into(), orbit, observer, start: at - TimeDelta::seconds(300),
            duration_ms: 600_000, threshold, fine_ms: 10,
            construction: format!("threshold = mean elevation at local extremum {at}Z +/-2s; constructed ~4s event, not prevalence evidence"),
        });
        if peak {
            result.push(Case {
                name: "LEO-above-peak-no-pass".into(), orbit, observer,
                start: at - TimeDelta::seconds(300), duration_ms: 600_000,
                threshold: elevation(orbit, observer, at)? + 1e-7, fine_ms: 10,
                construction: format!("threshold = elevation at approximate peak {at}Z +1e-7 rad; near-graze no-pass control, not exact tangency"),
            });
        }
    }
    Ok(result)
}

#[derive(Clone, Copy, Debug)]
struct Crossing {
    rising: bool,
    lower_ms: f64,
    upper_ms: f64,
}

impl Crossing {
    fn overlaps(self, other: Self) -> bool {
        self.rising == other.rising
            && self.lower_ms <= other.upper_ms
            && other.lower_ms <= self.upper_ms
    }
}

// Independent of the search kernel: classify strict-side changes on a dense
// grid, with no bisection. Equality defers a transition until opposite strict
// sides exist; touching from above alone does not split an interval (0014).
fn transitions(samples: &[(i64, f64)], threshold: f64, stride: usize) -> Vec<Crossing> {
    let mut result = Vec::new();
    let mut previous = None;
    for &(at, value) in samples.iter().step_by(stride) {
        if value == threshold {
            continue;
        }
        let above = value > threshold;
        if let Some((before, was_above)) = previous
            && was_above != above
        {
            result.push(Crossing {
                rising: above,
                lower_ms: before as f64,
                upper_ms: at as f64,
            });
        }
        previous = Some((at, above));
    }
    result
}

#[derive(Debug, Serialize)]
struct Comparison {
    matched_crossings: usize,
    missed_crossings: usize,
    unmatched_candidate_crossings: usize,
    ambiguous_reference_crossings: usize,
    ambiguous_candidate_crossings: usize,
    missed_excursions: usize,
    missed_gaps: usize,
}

// Match by direction AND intersecting uncertainty, never by index/nearest time.
// Only mutually unique overlaps are timing agreements. Ambiguous overlaps are
// reported separately rather than manufacturing a one-to-one association.
fn compare(reference: &[Crossing], candidate: &[Crossing]) -> Comparison {
    let r_degree: Vec<_> = reference
        .iter()
        .map(|r| candidate.iter().filter(|c| r.overlaps(**c)).count())
        .collect();
    let c_degree: Vec<_> = candidate
        .iter()
        .map(|c| reference.iter().filter(|r| c.overlaps(**r)).count())
        .collect();
    let matched = reference
        .iter()
        .enumerate()
        .filter(|(i, r)| {
            r_degree[*i] == 1
                && candidate
                    .iter()
                    .enumerate()
                    .any(|(j, c)| c_degree[j] == 1 && r.overlaps(*c))
        })
        .count();
    let missed_pairs = |rising| {
        reference
            .windows(2)
            .enumerate()
            .filter(|(i, pair)| {
                pair[0].rising == rising
                    && pair[1].rising != rising
                    && r_degree[*i] == 0
                    && r_degree[*i + 1] == 0
            })
            .count()
    };
    Comparison {
        matched_crossings: matched,
        missed_crossings: r_degree.iter().filter(|&&d| d == 0).count(),
        unmatched_candidate_crossings: c_degree.iter().filter(|&&d| d == 0).count(),
        ambiguous_reference_crossings: r_degree.iter().filter(|&&d| d > 0).count() - matched,
        ambiguous_candidate_crossings: c_degree.iter().filter(|&&d| d > 0).count() - matched,
        missed_excursions: missed_pairs(true),
        missed_gaps: missed_pairs(false),
    }
}

#[derive(Serialize)]
struct Row {
    detection_ms: i64,
    phase_ms: i64,
    tolerance_ms: i64,
    evaluations: u64,
    passes: usize,
    reference_crossings: usize,
    candidate_crossings: usize,
    max_bracket_width_ms: f64,
    comparison: Comparison,
}

#[derive(Serialize)]
struct CaseReport {
    name: String,
    satellite: String,
    norad_id: u64,
    orbit_class: String,
    epoch_utc: String,
    start_utc: String,
    duration_ms: i64,
    observer_lat_lon_height_deg_km: [f64; 3],
    threshold_deg: f64,
    construction: String,
    reference_step_ms: i64,
    reference_evaluations: usize,
    reference_crossings: usize,
    reference_above_at_start: bool,
    reference_above_at_end: bool,
    // Bounds from crossing brackets, not exact duration estimates.
    shortest_excursion_ms: Option<[f64; 2]>,
    shortest_gap_ms: Option<[f64; 2]>,
    reference_coarser_grid_comparison: Comparison,
    rows: Vec<Row>,
}

fn shortest(reference: &[Crossing], rising: bool) -> Option<[f64; 2]> {
    reference
        .windows(2)
        .filter(|p| p[0].rising == rising && p[1].rising != rising)
        .map(|p| {
            [
                (p[1].lower_ms - p[0].upper_ms).max(0.0),
                p[1].upper_ms - p[0].lower_ms,
            ]
        })
        .reduce(|a, b| [a[0].min(b[0]), a[1].min(b[1])])
}

fn evaluate(case: &Case<'_>) -> Result<CaseReport> {
    let mut samples = Vec::new();
    for ms in (0..=case.duration_ms).step_by(case.fine_ms as usize) {
        samples.push((
            ms,
            elevation(
                case.orbit,
                case.observer,
                case.start + TimeDelta::milliseconds(ms),
            )?,
        ));
    }
    let fine = transitions(&samples, case.threshold, 1);
    let coarse = transitions(&samples, case.threshold, 2);
    let stability = compare(&fine, &coarse);
    // Do not silently use an unstable reference to judge the candidate searches.
    if stability.matched_crossings != fine.len() || fine.len() != coarse.len() {
        return Err(format!(
            "{}: reference grid changed crossing structure: {stability:?}",
            case.name
        ));
    }
    let mut rows = Vec::new();
    for tolerance_ms in TOLERANCES_MS {
        for detection_ms in INTERVALS_MS {
            // Whole seconds align with both reference grids; dedup short steps.
            let mut phases: Vec<_> = (0..4)
                .map(|q| (detection_ms * q / 4 / 1000) * 1000)
                .collect();
            phases.dedup();
            for phase_ms in phases {
                let start = case.start + TimeDelta::milliseconds(phase_ms);
                let config = SearchConfig::new(
                    start,
                    TimeDelta::milliseconds(case.duration_ms - phase_ms),
                    case.threshold,
                    TimeDelta::milliseconds(detection_ms),
                    TimeDelta::milliseconds(tolerance_ms),
                )
                .map_err(|e| e.to_string())?;
                let mut budget = EvaluationBudget::new(SEARCH_LIMIT);
                let mut records = Vec::new();
                let report = search_satellite(
                    &case.orbit.satellite,
                    case.observer,
                    &config,
                    SEARCH_LIMIT,
                    &mut budget,
                    |p| records.push(p),
                );
                if !report.is_complete() {
                    return Err(format!(
                        "{}: incomplete search: {:?}",
                        case.name, report.status
                    ));
                }
                let mut candidate = Vec::new();
                for record in &records {
                    let mut push = |rising, b: overhead_core::passes::CrossingBracket| {
                        candidate.push(Crossing {
                            rising,
                            lower_ms: (b.lower() - case.start).num_nanoseconds().unwrap() as f64
                                / 1e6,
                            upper_ms: (b.upper() - case.start).num_nanoseconds().unwrap() as f64
                                / 1e6,
                        });
                    };
                    if let PassStart::Crossing(b) = record.start {
                        push(true, b);
                    }
                    if let PassEnd::Crossing(b) = record.end {
                        push(false, b);
                    }
                }
                // Derive a reference for the exact shortened window: do not count
                // an arrival before the shifted start as a candidate miss.
                let index = (phase_ms / case.fine_ms) as usize;
                let reference = transitions(&samples[index..], case.threshold, 1);
                rows.push(Row {
                    detection_ms,
                    phase_ms,
                    tolerance_ms,
                    evaluations: report.evaluations,
                    passes: records.len(),
                    reference_crossings: reference.len(),
                    candidate_crossings: candidate.len(),
                    max_bracket_width_ms: candidate
                        .iter()
                        .map(|c| c.upper_ms - c.lower_ms)
                        .fold(0.0, f64::max),
                    comparison: compare(&reference, &candidate),
                });
            }
        }
    }
    Ok(CaseReport {
        name: case.name.clone(),
        satellite: case.orbit.name.into(),
        norad_id: case.orbit.satellite.norad_id(),
        orbit_class: case.orbit.class.into(),
        epoch_utc: format!("{}Z", case.orbit.satellite.epoch()),
        start_utc: format!("{}Z", case.start),
        duration_ms: case.duration_ms,
        observer_lat_lon_height_deg_km: [
            case.observer.latitude_rad.to_degrees(),
            case.observer.longitude_rad.to_degrees(),
            case.observer.altitude_km,
        ],
        threshold_deg: case.threshold.to_degrees(),
        construction: case.construction.clone(),
        reference_step_ms: case.fine_ms,
        reference_evaluations: samples.len(),
        reference_crossings: fine.len(),
        reference_above_at_start: samples[0].1 > case.threshold,
        reference_above_at_end: samples.last().unwrap().1 > case.threshold,
        shortest_excursion_ms: shortest(&fine, true),
        shortest_gap_ms: shortest(&fine, false),
        reference_coarser_grid_comparison: stability,
        rows,
    })
}

#[derive(Serialize)]
struct Experiment {
    schema_version: u32,
    scope: &'static str,
    phase_policy: &'static str,
    search_evaluation_guard: u64,
    cases: Vec<CaseReport>,
}

fn run() -> Result<String> {
    let orbits = orbits()?;
    let cases = cases(&orbits)?;
    let reports = cases.iter().map(evaluate).collect::<Result<Vec<_>>>()?;
    serde_json::to_string_pretty(&Experiment {
        schema_version: 1,
        scope: "same-model integration/detection experiment; finite reference grids cannot prove completeness or independent/real-world timing accuracy; no operating defaults or wall-time benchmarks",
        phase_policy: "start shifted by floor(interval*q/4) whole seconds, q=0..3, deduplicated; common fixed end; exact-window reference; not exhaustive phase coverage",
        search_evaluation_guard: SEARCH_LIMIT, cases: reports,
    }).map_err(|e| e.to_string())
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() == 1 && matches!(args[0].as_str(), "--help" | "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let result = if args.is_empty() {
        run()
    } else {
        Err(format!("unexpected arguments\n{USAGE}"))
    };
    match result {
        Ok(report) => {
            println!("{report}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crossing(rising: bool, lower_ms: f64, upper_ms: f64) -> Crossing {
        Crossing {
            rising,
            lower_ms,
            upper_ms,
        }
    }

    #[test]
    fn matching_requires_direction_overlap_and_unique_association() {
        let r = [
            crossing(true, 1.0, 2.0),
            crossing(false, 4.0, 5.0),
            crossing(true, 8.0, 9.0),
        ];
        let c = [crossing(true, 1.5, 2.5), crossing(false, 20.0, 21.0)];
        let m = compare(&r, &c);
        assert_eq!(m.matched_crossings, 1);
        assert_eq!(m.missed_crossings, 2);
        assert_eq!(m.unmatched_candidate_crossings, 1);
        assert_eq!(m.missed_excursions, 0);
        assert_eq!(m.missed_gaps, 1);
        let m = compare(&r, &[crossing(true, 0.0, 10.0)]);
        assert_eq!(m.matched_crossings, 0);
        assert_eq!(m.ambiguous_reference_crossings, 2);
        assert_eq!(m.ambiguous_candidate_crossings, 1);
        assert_eq!(
            compare(&r[..1], &[crossing(false, 1.0, 2.0)]).matched_crossings,
            0
        );
        assert_eq!(
            compare(&r[..1], &[r[0], r[0]]).ambiguous_reference_crossings,
            1
        );
    }

    #[test]
    fn dense_scan_handles_equality_and_clipped_window_without_inventing_arrival() {
        let samples = [(0, -1.0), (1, 0.0), (2, 1.0), (3, 0.0), (4, 1.0), (5, -1.0)];
        let t = transitions(&samples, 0.0, 1);
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].lower_ms, t[0].upper_ms), (0.0, 2.0));
        assert_eq!(shortest(&t, true), Some([2.0, 5.0]));
        assert_eq!(transitions(&samples[2..], 0.0, 1).len(), 1);
        assert!(transitions(&[(0, -1.0), (1, 0.0), (2, -1.0)], 0.0, 1).is_empty());
    }

    #[test]
    fn shortest_duration_bounds_include_uncertainty_from_every_event() {
        let crossings = [
            crossing(true, 0.0, 10.0),
            crossing(false, 11.0, 20.0),
            crossing(true, 30.0, 31.0),
            crossing(false, 40.0, 41.0),
        ];
        assert_eq!(shortest(&crossings, true), Some([1.0, 11.0]));
    }

    #[test]
    fn constructed_orbital_events_expose_phase_sensitive_misses_and_gap_merging() {
        let orbits = orbits().unwrap();
        let cases = cases(&orbits).unwrap();
        assert_eq!(cases.len(), 15);
        for case in &cases[12..] {
            let report = evaluate(case).unwrap();
            if case.name.contains("no-pass") {
                assert_eq!(report.reference_crossings, 0);
                assert!(report.rows.iter().all(|r| r.passes == 0));
                continue;
            }
            assert_eq!(report.reference_crossings, 2);
            let gap = case.name.contains("gap");
            let duration = if gap {
                report.shortest_gap_ms
            } else {
                report.shortest_excursion_ms
            }
            .unwrap();
            assert!(duration[0] > 3900.0 && duration[1] < 4100.0, "{duration:?}");
            assert!(
                report
                    .rows
                    .iter()
                    .filter(|r| r.detection_ms == 1000)
                    .all(|r| r.comparison.matched_crossings == 2)
            );
            assert!(
                report
                    .rows
                    .iter()
                    .any(|r| r.detection_ms == 5000 && r.comparison.matched_crossings == 2)
            );
            assert!(
                report
                    .rows
                    .iter()
                    .any(|r| r.detection_ms == 5000 && r.comparison.missed_crossings == 2)
            );
            for row in &report.rows {
                assert!(row.max_bracket_width_ms <= row.tolerance_ms as f64);
                assert_eq!(row.comparison.unmatched_candidate_crossings, 0);
                if row.comparison.missed_crossings == 2 {
                    assert_eq!(row.passes, usize::from(gap));
                    assert_eq!(row.comparison.missed_gaps, usize::from(gap));
                    assert_eq!(row.comparison.missed_excursions, usize::from(!gap));
                }
            }
            // Refinement changes work/bracket width, not regular-grid detection.
            let (tight, loose) = report.rows.split_at(report.rows.len() / 2);
            for (a, b) in tight.iter().zip(loose) {
                assert_eq!(a.candidate_crossings, b.candidate_crossings);
                assert_eq!(
                    a.comparison.matched_crossings,
                    b.comparison.matched_crossings
                );
                assert!(a.evaluations >= b.evaluations);
            }
        }
    }
}
