use super::*;
use crate::passes::{PassEnd, SearchStatus, search_elevations};
use alloc::vec;
use sgp4::chrono::{NaiveDate, TimeDelta};

// Synthetic brackets test interval ordering, not orbit accuracy. Individual
// windows vary here only to construct brackets via the public search kernel.
fn arrival(id: u64, lower: i64, upper: i64, incomplete: bool) -> SatellitePasses {
    let start = NaiveDate::from_ymd_opt(2026, 10, 8)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        + TimeDelta::seconds(lower);
    let width = TimeDelta::seconds(upper - lower);
    let config = SearchConfig::new(
        start,
        width,
        0.0,
        width,
        if incomplete {
            TimeDelta::nanoseconds(1)
        } else {
            width
        },
    )
    .unwrap();
    let mut passes = Vec::new();
    let report = search_elevations(
        &config,
        2,
        &mut EvaluationBudget::new(2),
        |at| Ok(if at == start { -0.1 } else { 0.1 }),
        |p| passes.push(p),
    );
    SatellitePasses {
        norad_id: id,
        report,
        passes,
    }
}

fn aggregate(satellites: Vec<SatellitePasses>) -> CataloguePasses {
    let evaluations = satellites.iter().map(|s| s.report.evaluations).sum();
    CataloguePasses {
        satellites,
        evaluations,
    }
}

fn ids(results: &CataloguePasses) -> Vec<u64> {
    results
        .earliest_candidates()
        .iter()
        .map(|a| results.satellites[a.satellite_index].norad_id)
        .collect()
}

#[test]
fn disjoint_order_uses_brackets_not_ids() {
    let r = aggregate(vec![arrival(1, 30, 40, false), arrival(2, 10, 20, false)]);
    assert!(r.is_complete());
    assert_eq!(ids(&r), [2]);
}

#[test]
fn touching_nested_and_identical_brackets_remain_ambiguous() {
    for intervals in [
        vec![(10, 20), (20, 30)],
        vec![(0, 40), (10, 20)],
        vec![(10, 20), (10, 20)],
    ] {
        let r = aggregate(
            intervals
                .iter()
                .enumerate()
                .map(|(i, &(l, u))| arrival(i as u64, l, u, false))
                .collect(),
        );
        assert_eq!(ids(&r), [0, 1]);
    }
}

#[test]
fn overlap_chains_do_not_pull_in_definitely_later_arrivals() {
    let r = aggregate(vec![
        arrival(1, 0, 10, false),
        arrival(2, 5, 100, false),
        arrival(3, 50, 60, false),
    ]);
    assert_eq!(ids(&r), [1, 2]);
    // A midpoint sort would wrongly pick the narrow 10..20 bracket alone.
    let r = aggregate(vec![arrival(1, 0, 100, true), arrival(2, 10, 20, false)]);
    assert!(!r.is_complete());
    assert_eq!(ids(&r), [1, 2]);
    assert!(!r.earliest_candidates()[0].bracket.meets_tolerance());
}

#[test]
fn no_arrival_is_distinct_from_incomplete_and_underway() {
    let mut satellite = arrival(1, 0, 10, false);
    for start in [PassStart::InProgress, PassStart::BoundaryStart] {
        satellite.passes[0].start = start;
        satellite.passes[0].end = PassEnd::BeyondWindow;
        satellite.report.upcoming_passes = 0;
        let r = aggregate(vec![satellite.clone()]);
        assert!(r.is_complete());
        assert!(r.earliest_candidates().is_empty());
        assert_eq!(r.satellites()[0].passes().len(), 1);
    }
    let mut interrupted = arrival(2, 20, 30, true);
    assert!(matches!(
        interrupted.report.status,
        SearchStatus::Incomplete(_)
    ));
    interrupted.passes.clear();
    interrupted.report.passes = 0;
    interrupted.report.upcoming_passes = 0;
    let r = aggregate(vec![satellite, interrupted]);
    assert!(!r.is_complete());
    assert!(r.earliest_candidates().is_empty());
}

#[test]
fn pass_indices_survive_nonarrival_records_and_multiple_arrivals() {
    let mut first = arrival(1, 20, 30, false);
    first.passes.insert(
        0,
        PredictedPass {
            start: PassStart::InProgress,
            end: PassEnd::Interrupted,
        },
    );
    first.passes.push(arrival(1, 40, 50, false).passes[0]);
    let r = aggregate(vec![first, arrival(2, 25, 35, false)]);
    assert_eq!(ids(&r), [1, 2]);
    assert_eq!(r.earliest_candidates()[0].pass_index, 1);
}
