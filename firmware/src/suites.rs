//! Small, separately flashable cost experiments. No production tuning defaults.
use core::fmt;

use crate::{
    CLASSES, Satellite, SearchConfig, Work, age_times, predict, search_configs, track_grid,
};

#[path = "selection.rs"]
mod selection;
pub use selection::{AGE_DAYS, SUITE_NAMES, sample_count};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    State,
    Geometry,
    Search,
}

impl Operation {
    pub fn name(self) -> &'static str {
        match self {
            Self::State => "state_at",
            Self::Geometry => "state_at+ecef+look_angles",
            Self::Search => "search_satellite",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Workload {
    pub operation: Operation,
    pub class: Option<usize>,
    pub slots: usize,
    /// Center of the tracking/search grid, relative to each fixture epoch.
    pub age_days: i64,
    pub window_s: i64,
    pub detection_s: i64,
    pub tolerance_ms: i64,
}

impl Workload {
    fn baseline(operation: Operation, class: Option<usize>, slots: usize) -> Self {
        Self {
            operation,
            class,
            slots,
            age_days: 0,
            window_s: 86400,
            detection_s: 60,
            tolerance_ms: 5000,
        }
    }

    /// Prepare outside the timer. Slots are references, not cloned satellites.
    pub fn prepare(self) -> Prepared {
        let slots = core::array::from_fn(|i| self.class.unwrap_or(i % 4));
        let configs = search_configs(
            self.age_days,
            self.window_s,
            self.detection_s,
            self.tolerance_ms,
        );
        Prepared {
            workload: self,
            slots,
            configs,
        }
    }
}

/// Stable CSV metadata shared by target output and offline expected-work files.
/// Detection/tolerance are zero for tracking (not applicable).
impl fmt::Display for Workload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let class = self
            .class
            .map(|i| CLASSES[i])
            .unwrap_or("mixed-repeated-fixtures");
        let (detection, tolerance) = if self.operation == Operation::Search {
            (self.detection_s, self.tolerance_ms)
        } else {
            (0, 0)
        };
        write!(
            f,
            "{},{},{},{},{},{},{}",
            self.operation.name(),
            class,
            self.slots,
            self.age_days,
            self.window_s,
            detection,
            tolerance
        )
    }
}

pub struct Prepared {
    workload: Workload,
    slots: [usize; 64],
    configs: [SearchConfig; 4],
}

impl Prepared {
    pub fn run(&self, satellites: &[Satellite; 4]) -> Work {
        let slots = &self.slots[..self.workload.slots];
        match self.workload.operation {
            Operation::Search => predict(satellites, slots, &self.configs),
            operation => track_grid(
                satellites,
                slots,
                operation == Operation::Geometry,
                age_times(self.workload.age_days),
            ),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Suite {
    index: usize,
}

impl Suite {
    pub fn parse(name: &str) -> Option<Self> {
        SUITE_NAMES
            .iter()
            .position(|&n| n == name)
            .map(|index| Self { index })
    }

    pub fn name(self) -> &'static str {
        SUITE_NAMES[self.index]
    }

    pub fn len(self) -> usize {
        match self.index {
            0 => 14,
            1 | 2 => 2,
            3..=6 => 14,
            _ => 12,
        }
    }

    pub fn is_empty(self) -> bool {
        false
    }

    pub fn workloads(self) -> impl Iterator<Item = Workload> {
        (0..self.len()).map(move |i| match self.index {
            0 if i < 12 => Workload::baseline(
                [Operation::State, Operation::Geometry, Operation::Search][i % 3],
                Some(i / 3),
                1,
            ),
            0..=2 => Workload::baseline(
                if i % 2 == 0 {
                    Operation::Geometry
                } else {
                    Operation::Search
                },
                None,
                [4, 16, 64][self.index],
            ),
            3..=6 => Workload {
                age_days: AGE_DAYS[i / 2],
                ..Workload::baseline(
                    if i % 2 == 0 {
                        Operation::Geometry
                    } else {
                        Operation::Search
                    },
                    Some(self.index - 3),
                    1,
                )
            },
            _ => Workload {
                window_s: [3600, 86400][i / 6],
                detection_s: [5, 30, 60][(i / 2) % 3],
                tolerance_ms: [250, 5000][i % 2],
                ..Workload::baseline(Operation::Search, Some(self.index - 7), 1)
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TIMES, TRACK_STEPS, TimeDelta, configs, satellites, track};

    #[test]
    fn selection_and_matrix() {
        assert!(Suite::parse("typo").is_none());
        for name in SUITE_NAMES {
            let suite = Suite::parse(name).unwrap();
            assert_eq!(suite.name(), name);
            assert_eq!(suite.workloads().count(), suite.len());
            for (i, workload) in suite.workloads().enumerate() {
                assert!(!suite.workloads().take(i).any(|other| other == workload));
                assert!(workload.slots <= 64);
            }
        }
        for invalid in ["0", "6", "-1", "", "1.5"] {
            assert_eq!(sample_count(invalid), None);
        }
        assert_eq!(sample_count("1"), Some(1));
        assert_eq!(sample_count("5"), Some(5));
        let search = Suite::parse("search-heo").unwrap();
        for window in [3600, 86400] {
            for detection in [5, 30, 60] {
                for tolerance in [250, 5000] {
                    assert_eq!(
                        search
                            .workloads()
                            .filter(|w| w.window_s == window
                                && w.detection_s == detection
                                && w.tolerance_ms == tolerance
                                && w.class == Some(1))
                            .count(),
                        1
                    );
                }
            }
        }
    }

    #[test]
    fn signed_age_grids_preserve_spacing_and_endpoints() {
        for age in AGE_DAYS {
            let grids = age_times(age);
            for (i, grid) in grids.iter().enumerate() {
                for step in 0..TRACK_STEPS {
                    assert_eq!(grid[step], TIMES[i][step] + TimeDelta::days(age));
                }
            }
        }
    }

    #[test]
    fn all_suites_complete_and_repeat_with_expected_accounting() {
        let sats = satellites();
        for name in SUITE_NAMES {
            for workload in Suite::parse(name).unwrap().workloads() {
                let prepared = workload.prepare();
                let work = prepared.run(&sats);
                assert_eq!(prepared.run(&sats), work, "{name}: {workload}");
                if workload.operation == Operation::Search {
                    assert_eq!(work.searches, workload.slots as u64);
                    assert!(
                        work.state_evaluations
                            >= (workload.window_s / workload.detection_s + 1) as u64
                                * workload.slots as u64
                    );
                    if workload.class.is_none() {
                        let multiplier = workload.slots as u64 / 4;
                        assert_eq!(
                            work,
                            Work {
                                state_evaluations: 5836 * multiplier,
                                searches: 4 * multiplier,
                                passes: 9 * multiplier
                            }
                        );
                    }
                } else {
                    assert_eq!(
                        work,
                        Work {
                            state_evaluations: 1441 * workload.slots as u64,
                            searches: 0,
                            passes: 0
                        }
                    );
                }
            }
        }
    }

    #[test]
    fn baseline_workloads_unchanged() {
        let sats = satellites();
        for workload in Suite::parse("baseline").unwrap().workloads() {
            let slots = [0, 1, 2, 3];
            let single = [workload.class.unwrap_or(0)];
            let slots = if workload.class.is_some() {
                &single[..]
            } else {
                &slots[..]
            };
            let old = match workload.operation {
                Operation::Search => predict(&sats, slots, &configs()),
                operation => track(&sats, slots, operation == Operation::Geometry),
            };
            assert_eq!(workload.prepare().run(&sats), old);
        }
    }
}
