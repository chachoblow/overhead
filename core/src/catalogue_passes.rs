//! Allocating catalogue pass aggregation, enabled by `catalogue`.
//!
//! Searches synchronously in ascending NORAD ID order (deterministic traversal,
//! not a scheduling/fairness policy). Every accepted entry gets a report, even
//! after budget exhaustion. Completeness refers only to the accepted catalogue
//! and configured numerical procedure, not rejected input records or all events.

#[cfg(test)]
mod tests;

use alloc::vec::Vec;

use crate::{
    GeodeticPosition,
    catalogue::Catalogue,
    passes::{
        CrossingBracket, EvaluationBudget, OrbitalEvaluationError, PassStart, PredictedPass,
        SearchConfig, SearchReport, search_satellite,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub struct SatellitePasses {
    norad_id: u64,
    report: SearchReport<OrbitalEvaluationError>,
    passes: Vec<PredictedPass>,
}

impl SatellitePasses {
    pub fn norad_id(&self) -> u64 {
        self.norad_id
    }

    pub fn report(&self) -> &SearchReport<OrbitalEvaluationError> {
        &self.report
    }

    /// Includes in-progress, boundary-start, and interrupted records.
    pub fn passes(&self) -> &[PredictedPass] {
        &self.passes
    }
}

/// Reference to an observed upward crossing; indices address `satellites()` and
/// that satellite's `passes()`. No exact arrival time is implied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arrival {
    pub satellite_index: usize,
    pub pass_index: usize,
    pub bracket: CrossingBracket,
}

#[must_use = "incomplete catalogue searches cannot establish a global next pass"]
#[derive(Debug, Clone, PartialEq)]
pub struct CataloguePasses {
    satellites: Vec<SatellitePasses>,
    evaluations: u64,
}

impl CataloguePasses {
    /// Ascending NORAD ID, including incomplete and unsearched entries.
    pub fn satellites(&self) -> &[SatellitePasses] {
        &self.satellites
    }

    /// Attempts consumed by this search, not prior uses of the shared budget.
    pub fn evaluations(&self) -> u64 {
        self.evaluations
    }

    pub fn is_complete(&self) -> bool {
        self.satellites.iter().all(|s| s.report.is_complete())
    }

    /// Candidates for earliest *detected* arrival. Empty means no detected
    /// arrival; it means none within the window only if `is_complete()`.
    /// In-progress and equality-boundary starts are not upcoming arrivals.
    ///
    /// Keep every bracket whose lower endpoint is <= the smallest upper endpoint
    /// among all observed arrivals. Others are definitely later than at least
    /// one observed arrival. Touching endpoints remain ambiguous. This is not a
    /// transitive overlap group: a long bracket must not pull in later events.
    /// All retained brackets, including interrupted refinement, participate.
    ///
    /// A singleton resolves ordering only among detected arrivals. Multiple
    /// candidates have unresolved ordering, not necessarily simultaneous times.
    /// Neither case establishes catalogue-wide earliest if any work is incomplete.
    /// Short events/additional crossings may still be missed in complete work.
    /// Output order is NORAD ID then pass index, for display only.
    pub fn earliest_candidates(&self) -> Vec<Arrival> {
        let arrivals = || {
            self.satellites
                .iter()
                .enumerate()
                .flat_map(|(satellite_index, s)| {
                    s.passes
                        .iter()
                        .enumerate()
                        .filter_map(move |(pass_index, pass)| match pass.start {
                            PassStart::Crossing(bracket) => Some(Arrival {
                                satellite_index,
                                pass_index,
                                bracket,
                            }),
                            _ => None,
                        })
                })
        };
        let Some(earliest_upper) = arrivals().map(|a| a.bracket.upper()).min() else {
            return Vec::new();
        };
        arrivals()
            .filter(|a| a.bracket.lower() <= earliest_upper)
            .collect()
    }
}

/// Store all streamed records and reports using alloc; the underlying orbital
/// search remains allocation-free. No record/storage capacity, wall-time budget,
/// detection default, freshness cutoff, or background scheduling is introduced.
/// Continue after local limits/failures while shared budget permits; zero budget
/// still produces an unsearched report for every remaining catalogue entry.
pub fn search_catalogue(
    catalogue: &Catalogue,
    observer: GeodeticPosition,
    config: &SearchConfig,
    satellite_limit: u64,
    total_budget: &mut EvaluationBudget,
) -> CataloguePasses {
    let before = total_budget.used();
    let satellites = catalogue
        .entries()
        .iter()
        .map(|entry| {
            let mut passes = Vec::new();
            let report = search_satellite(
                entry.satellite(),
                observer,
                config,
                satellite_limit,
                total_budget,
                |pass| passes.push(pass),
            );
            SatellitePasses {
                norad_id: entry.satellite().norad_id(),
                report,
                passes,
            }
        })
        .collect();
    CataloguePasses {
        satellites,
        evaluations: total_budget.used() - before,
    }
}
