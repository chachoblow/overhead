#![no_std]
#![cfg_attr(
    all(feature = "catalogue-memory", target_arch = "xtensa"),
    feature(asm_experimental_arch)
)]
//! Allocation-free kernel workloads plus opt-in catalogue/memory experiments.
//! Shared with host checks; not an application scheduler or device catalogue.
use core::hint::black_box;
use overhead_core::{
    GeodeticPosition, Satellite, ecef_to_look_angles,
    passes::{EvaluationBudget, SearchConfig, search_satellite},
    sgp4::chrono::{NaiveDateTime, TimeDelta},
};

pub mod suites;

#[cfg(feature = "catalogue-memory")]
extern crate alloc;
#[cfg(feature = "catalogue-memory")]
pub mod catalogue_memory;
#[cfg(feature = "catalogue-memory")]
pub mod heap_meter;
#[cfg(all(feature = "catalogue-memory", any(target_arch = "xtensa", test)))]
pub mod stack_watermark;

pub const TRACK_STEPS: usize = 1441;
pub const SEARCH_LIMIT: u64 = 200_000;
pub const CLASSES: [&str; 4] = ["LEO", "resonant-HEO", "GEO", "GNSS"];
include!(concat!(env!("OUT_DIR"), "/fixtures.rs"));

pub fn satellites() -> [Satellite; 4] {
    elements().map(|e| Satellite::from_elements(&e).expect("historical fixture initialization"))
}

pub fn observer() -> GeodeticPosition {
    GeodeticPosition {
        latitude_rad: 39.007_f64.to_radians(),
        longitude_rad: (-104.883_f64).to_radians(),
        altitude_km: 2.187,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Work {
    pub state_evaluations: u64,
    pub searches: u64,
    pub passes: u64,
}

/// Indices repeat references to four fixtures; never distinct catalogue entries.
pub fn track(satellites: &[Satellite; 4], slots: &[usize], geometry: bool) -> Work {
    track_grid(satellites, slots, geometry, &TIMES)
}

#[allow(clippy::needless_range_loop)] // Time-major indexing matches the host workload.
fn track_grid(
    satellites: &[Satellite; 4],
    slots: &[usize],
    geometry: bool,
    times: &[[NaiveDateTime; TRACK_STEPS]; 4],
) -> Work {
    let observer = black_box(observer());
    for step in 0..TRACK_STEPS {
        for &i in slots {
            let state = black_box(&satellites[i])
                .state_at(black_box(times[i][step]))
                .expect("tracking propagation");
            if geometry {
                let ecef = state.to_ecef().expect("tracking ECEF");
                black_box(ecef_to_look_angles(ecef, observer).expect("tracking look angles"));
            } else {
                black_box(state);
            }
        }
    }
    Work {
        state_evaluations: (TRACK_STEPS * slots.len()) as u64,
        searches: 0,
        passes: 0,
    }
}

pub fn configs() -> [SearchConfig; 4] {
    search_configs(0, 86400, 60, 5000)
}

fn search_configs(
    age_days: i64,
    window_s: i64,
    detection_s: i64,
    tolerance_ms: i64,
) -> [SearchConfig; 4] {
    core::array::from_fn(|i| {
        SearchConfig::new(
            age_times(age_days)[i][0],
            TimeDelta::seconds(window_s),
            10_f64.to_radians(),
            TimeDelta::seconds(detection_s),
            TimeDelta::milliseconds(tolerance_ms),
        )
        .expect("experimental configuration")
    })
}

pub fn predict(satellites: &[Satellite; 4], slots: &[usize], configs: &[SearchConfig; 4]) -> Work {
    let mut work = Work {
        state_evaluations: 0,
        searches: 0,
        passes: 0,
    };
    let mut budget = EvaluationBudget::new(SEARCH_LIMIT * slots.len() as u64);
    for &i in slots {
        let report = search_satellite(
            black_box(&satellites[i]),
            black_box(observer()),
            black_box(&configs[i]),
            SEARCH_LIMIT,
            &mut budget,
            |pass| {
                black_box(pass);
                work.passes += 1;
            },
        );
        assert!(
            report.is_complete(),
            "incomplete search: {:?}",
            report.status
        );
        work.searches += 1;
        work.state_evaluations += report.evaluations;
        let _ = black_box(report);
    }
    assert_eq!(work.state_evaluations, budget.used());
    work
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_identity_and_time_grid() {
        let sats = satellites();
        assert_eq!(
            sats.each_ref().map(|s| s.norad_id()),
            [25544, 8195, 24208, 28129]
        );
        for (i, sat) in sats.iter().enumerate() {
            assert_eq!(TIMES[i][0], sat.epoch() - TimeDelta::hours(12));
            assert_eq!(TIMES[i][1440], sat.epoch() + TimeDelta::hours(12));
            for times in TIMES[i].windows(2) {
                assert_eq!(times[1] - times[0], TimeDelta::seconds(60));
            }
        }
    }

    #[test]
    fn counts_match_recorded_host_baseline() {
        let sats = satellites();
        let configs = configs();
        for (i, (evaluations, passes)) in [(1497, 7), (1449, 1), (1441, 0), (1449, 1)]
            .into_iter()
            .enumerate()
        {
            assert_eq!(
                predict(&sats, &[i], &configs),
                Work {
                    state_evaluations: evaluations,
                    searches: 1,
                    passes
                }
            );
            for geometry in [false, true] {
                assert_eq!(
                    track(&sats, &[i], geometry),
                    Work {
                        state_evaluations: 1441,
                        searches: 0,
                        passes: 0
                    }
                );
            }
        }
        let slots: [usize; 64] = core::array::from_fn(|i| i % 4);
        for count in [4, 16, 64] {
            assert_eq!(
                predict(&sats, &slots[..count], &configs),
                Work {
                    state_evaluations: 5836 * (count as u64 / 4),
                    searches: count as u64,
                    passes: 9 * (count as u64 / 4),
                }
            );
        }
    }
}
