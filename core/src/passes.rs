//! Bounded, allocation-free physical pass search over geometric elevation.
//!
//! The evaluator supplies radians at explicit UTC timestamps. It should be a
//! deterministic, continuous function of time; search does not verify continuity.
//! Regular samples detect transitions, then bisection refines their brackets.
//! Brief excursions/gaps and additional crossings inside a bracket can be missed:
//! `Complete` means the procedure finished, not proof of event completeness or a
//! unique crossing. See decisions 0012–0015 for the product/numerical contracts.
//!
//! No orbit, observer, clock, catalogue, allocation, or scheduler is implicit.
//! Records are streamed once their end is known or search stops. Callbacks own
//! their storage and must return normally; budgets count elevation evaluations,
//! not callback duration or wall-clock time.

use core::f64::consts::{FRAC_PI_2, PI};

use sgp4::chrono::{NaiveDateTime, TimeDelta};

use crate::{TimeError, validate_utc_time};

pub const DEFAULT_MIN_ELEVATION_RAD: f64 = PI / 18.0;
pub const DEFAULT_LOOK_AHEAD: TimeDelta = TimeDelta::hours(24);
pub const DEFAULT_CROSSING_TOLERANCE: TimeDelta = TimeDelta::seconds(5);

/// Why a search configuration cannot be constructed. No evaluations have run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchConfigError {
    StartTime(TimeError),
    NonPositiveLookAhead,
    EndTimeOutOfRange,
    EndTime(TimeError),
    InvalidMinimumElevation,
    NonPositiveDetectionInterval,
    NonPositiveCrossingTolerance,
}

impl core::fmt::Display for SearchConfigError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::StartTime(error) => write!(f, "search start: {error}"),
            Self::NonPositiveLookAhead => f.write_str("look-ahead must be positive"),
            Self::EndTimeOutOfRange => f.write_str("search end overflows UTC arithmetic"),
            Self::EndTime(error) => write!(f, "search end: {error}"),
            Self::InvalidMinimumElevation => {
                f.write_str("minimum elevation must be finite and within [-pi/2, pi/2]")
            }
            Self::NonPositiveDetectionInterval => {
                f.write_str("detection interval must be positive")
            }
            Self::NonPositiveCrossingTolerance => {
                f.write_str("crossing tolerance must be positive")
            }
        }
    }
}

/// Validated, read-only search inputs. No detection interval is implicit.
///
/// ```compile_fail
/// use overhead_core::passes::SearchConfig;
/// fn bypass_validation(config: &mut SearchConfig) {
///     config.min_elevation_rad = f64::NAN;
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchConfig {
    start: NaiveDateTime,
    end: NaiveDateTime,
    min_elevation_rad: f64,
    detection_interval: TimeDelta,
    crossing_tolerance: TimeDelta,
}

impl SearchConfig {
    /// Uses the shared UTC contract (1957–2100, no explicit leap seconds).
    /// Durations must be positive, but need not divide the window or be ordered
    /// relative to one another. Minimum elevation includes both ±pi/2 endpoints.
    pub fn new(
        start: NaiveDateTime,
        look_ahead: TimeDelta,
        min_elevation_rad: f64,
        detection_interval: TimeDelta,
        crossing_tolerance: TimeDelta,
    ) -> Result<Self, SearchConfigError> {
        validate_utc_time(start).map_err(SearchConfigError::StartTime)?;
        if look_ahead <= TimeDelta::zero() {
            return Err(SearchConfigError::NonPositiveLookAhead);
        }
        let end = start
            .checked_add_signed(look_ahead)
            .ok_or(SearchConfigError::EndTimeOutOfRange)?;
        validate_utc_time(end).map_err(SearchConfigError::EndTime)?;
        if !min_elevation_rad.is_finite() || !(-FRAC_PI_2..=FRAC_PI_2).contains(&min_elevation_rad)
        {
            return Err(SearchConfigError::InvalidMinimumElevation);
        }
        if detection_interval <= TimeDelta::zero() {
            return Err(SearchConfigError::NonPositiveDetectionInterval);
        }
        if crossing_tolerance <= TimeDelta::zero() {
            return Err(SearchConfigError::NonPositiveCrossingTolerance);
        }
        Ok(Self {
            start,
            end,
            min_elevation_rad,
            detection_interval,
            crossing_tolerance,
        })
    }

    /// 10° threshold, 24-hour look-ahead, 5-second crossing tolerance.
    /// Detection interval is still required; no work allowance is selected.
    pub fn with_defaults(
        start: NaiveDateTime,
        detection_interval: TimeDelta,
    ) -> Result<Self, SearchConfigError> {
        Self::new(
            start,
            DEFAULT_LOOK_AHEAD,
            DEFAULT_MIN_ELEVATION_RAD,
            detection_interval,
            DEFAULT_CROSSING_TOLERANCE,
        )
    }

    pub fn start(&self) -> NaiveDateTime {
        self.start
    }

    pub fn end(&self) -> NaiveDateTime {
        self.end
    }

    pub fn look_ahead(&self) -> TimeDelta {
        self.end - self.start
    }

    pub fn min_elevation_rad(&self) -> f64 {
        self.min_elevation_rad
    }

    pub fn detection_interval(&self) -> TimeDelta {
        self.detection_interval
    }

    pub fn crossing_tolerance(&self) -> TimeDelta {
        self.crossing_tolerance
    }
}

/// Shared evaluation allowance across any number of calls/satellites.
/// Does not impose a time budget or bound work inside caller callbacks.
#[derive(Debug, PartialEq, Eq)]
pub struct EvaluationBudget {
    limit: u64,
    used: u64,
}

impl EvaluationBudget {
    pub fn new(limit: u64) -> Self {
        Self { limit, used: 0 }
    }

    pub fn limit(&self) -> u64 {
        self.limit
    }

    pub fn used(&self) -> u64 {
        self.used
    }

    pub fn remaining(&self) -> u64 {
        self.limit - self.used
    }
}

/// A retained transition interval, not an exact crossing time. Direction comes
/// from its placement in [`PassStart`] or [`PassEnd`]. Narrow brackets do not
/// prove a unique crossing or eliminate the search's short-event detection limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossingBracket {
    lower: NaiveDateTime,
    upper: NaiveDateTime,
    meets_tolerance: bool,
}

impl CrossingBracket {
    fn new(lower: NaiveDateTime, upper: NaiveDateTime, tolerance: TimeDelta) -> Self {
        Self {
            lower,
            upper,
            meets_tolerance: upper - lower <= tolerance,
        }
    }

    pub fn lower(&self) -> NaiveDateTime {
        self.lower
    }

    pub fn upper(&self) -> NaiveDateTime {
        self.upper
    }

    pub fn width(&self) -> TimeDelta {
        self.upper - self.lower
    }

    /// False only for an interrupted refinement; the wider bracket is usable
    /// evidence of a transition but not a tolerance-level timing estimate.
    pub fn meets_tolerance(&self) -> bool {
        self.meets_tolerance
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassStart {
    /// Strictly above at search start; the actual start is unknown.
    InProgress,
    /// Equality at search start, then above with no observed below sample.
    /// No upward crossing is established, even after an initial equality run.
    BoundaryStart,
    Crossing(CrossingBracket),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassEnd {
    Crossing(CrossingBracket),
    /// Above at the end of a successfully searched window, subject to detection
    /// limits. Does not claim the satellite will remain above indefinitely.
    BeyondWindow,
    /// Equality at window end does not establish a downward crossing.
    BoundaryUncertain,
    /// No downward crossing established before an incomplete search stopped.
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PredictedPass {
    pub start: PassStart,
    pub end: PassEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchPhase {
    Sampling,
    RisingRefinement,
    FallingRefinement,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StopReason<E> {
    SatelliteLimit,
    /// Takes precedence if both limits deny the same evaluation.
    TotalLimit,
    Evaluation(E),
    /// Nonfinite or outside the geometric elevation range [-pi/2, pi/2].
    InvalidElevation(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchStop<E> {
    /// Failed evaluation time, or the time of a budget-denied call (not run).
    pub at: NaiveDateTime,
    pub phase: SearchPhase,
    pub reason: StopReason<E>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchStatus<E> {
    /// All regular samples and required refinements finished.
    Complete,
    /// No evaluation was attempted (zero available allowance).
    Unsearched(SearchStop<E>),
    /// At least one evaluation was attempted, possibly failing immediately.
    Incomplete(SearchStop<E>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElevationSample {
    pub at: NaiveDateTime,
    pub elevation_rad: f64,
}

/// Always inspect completion as well as the streamed records before making
/// no-pass or earliest-arrival claims.
#[must_use = "inspect search status; streamed pass records may be incomplete"]
#[derive(Debug, Clone, PartialEq)]
pub struct SearchReport<E> {
    pub status: SearchStatus<E>,
    /// Attempts in this call, including failed/invalid evaluations and refinement.
    pub evaluations: u64,
    /// Last successful regular sample, not refinement. Even a sample at the
    /// window end does not imply completion: its transition may fail to refine.
    pub last_sample: Option<ElevationSample>,
    /// Number of records delivered to the sink, including partial/open passes.
    pub passes: u64,
    /// Records with an observed upward crossing, including coarse brackets.
    /// In-progress and boundary-start intervals are excluded.
    pub upcoming_passes: u64,
}

impl<E> SearchReport<E> {
    pub fn is_complete(&self) -> bool {
        matches!(self.status, SearchStatus::Complete)
    }
}

/// Search one elevation function, streaming passes in detected time order.
///
/// Samples the exact start, regular grid points, and exact end (once). Refinement
/// does not move that grid and may revisit timestamps already evaluated. The
/// evaluator must not depend on call order: refinement queries are not monotonic.
/// Equality is exact floating-point equality, with no hidden angular epsilon.
///
/// Equal regular samples defer classification until a strict side is observed:
/// touches from below do not create passes, and touches from above do not split
/// them. Opposite strict sides establish a transition across any equality run.
/// Bisection treats equality as not strictly above, locating the above interval's
/// boundary; it does not turn a below-side tangency into a pass.
///
/// A sink receives each record once. On interruption an established crossing is
/// retained even if refinement did not reach tolerance. No upcoming arrival is
/// found only when `report.is_complete() && report.upcoming_passes == 0`, subject
/// to detection limits. Results from incomplete calls cannot establish a global
/// earliest arrival. Ordering overlapping brackets across satellites is not
/// resolved by this single-function kernel.
///
/// ```
/// use overhead_core::passes::{EvaluationBudget, SearchConfig, search_elevations};
/// use overhead_core::sgp4::chrono::{NaiveDate, TimeDelta};
/// let start = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap()
///     .and_hms_opt(0, 0, 0).unwrap();
/// // An explicit illustrative interval/allowance, not an operating default.
/// let config = SearchConfig::with_defaults(start, TimeDelta::seconds(60)).unwrap();
/// let mut budget = EvaluationBudget::new(2000);
/// let report = search_elevations(
///     &config, 2000, &mut budget, |_| Ok::<_, ()>(-0.1), |_| {},
/// );
/// assert!(report.is_complete());
/// assert_eq!(report.upcoming_passes, 0);
/// ```
pub fn search_elevations<E>(
    config: &SearchConfig,
    satellite_limit: u64,
    total_budget: &mut EvaluationBudget,
    elevation_at: impl FnMut(NaiveDateTime) -> Result<f64, E>,
    mut on_pass: impl FnMut(PredictedPass),
) -> SearchReport<E> {
    let mut evaluator = Evaluator {
        config,
        satellite_limit,
        total_budget,
        elevation_at,
        evaluations: 0,
    };
    let mut state = SearchState::default();
    let mut at = config.start;
    let stop = loop {
        let elevation_rad = match evaluator.evaluate(at, SearchPhase::Sampling) {
            Ok(value) => value,
            Err(stop) => break Some(stop),
        };
        let sample = ElevationSample { at, elevation_rad };
        state.last_sample = Some(sample);
        if elevation_rad != config.min_elevation_rad {
            let above = elevation_rad > config.min_elevation_rad;
            match state.last_strict {
                None if above => {
                    state.open = Some(if at == config.start {
                        PassStart::InProgress
                    } else {
                        PassStart::BoundaryStart
                    });
                }
                Some(previous) if above != (previous.elevation_rad > config.min_elevation_rad) => {
                    let phase = if above {
                        SearchPhase::RisingRefinement
                    } else {
                        SearchPhase::FallingRefinement
                    };
                    let (bracket, stop) = evaluator.refine(previous.at, at, phase);
                    if above {
                        state.open = Some(PassStart::Crossing(bracket));
                    } else {
                        state.close(PassEnd::Crossing(bracket), &mut on_pass);
                    }
                    if stop.is_some() {
                        break stop;
                    }
                }
                _ => {}
            }
            state.last_strict = Some(sample);
        }
        if at == config.end {
            break None;
        }
        // Clamp before adding, so even TimeDelta::MAX is a valid interval and
        // cannot overflow arithmetic. The grid is independent of refinement.
        at += config.detection_interval.min(config.end - at);
    };
    let end = if stop.is_some() {
        PassEnd::Interrupted
    } else if state.last_sample.unwrap().elevation_rad == config.min_elevation_rad {
        PassEnd::BoundaryUncertain
    } else {
        PassEnd::BeyondWindow
    };
    state.close(end, &mut on_pass);
    let status = match stop {
        None => SearchStatus::Complete,
        Some(stop) if evaluator.evaluations == 0 => SearchStatus::Unsearched(stop),
        Some(stop) => SearchStatus::Incomplete(stop),
    };
    SearchReport {
        status,
        evaluations: evaluator.evaluations,
        last_sample: state.last_sample,
        passes: state.passes,
        upcoming_passes: state.upcoming_passes,
    }
}

#[derive(Default)]
struct SearchState {
    last_sample: Option<ElevationSample>,
    last_strict: Option<ElevationSample>,
    open: Option<PassStart>,
    passes: u64,
    upcoming_passes: u64,
}

impl SearchState {
    fn close(&mut self, end: PassEnd, on_pass: &mut impl FnMut(PredictedPass)) {
        if let Some(start) = self.open.take() {
            on_pass(PredictedPass { start, end });
            // Each record needs at least one evaluation; these cannot overflow
            // before the u64 evaluation allowances are exhausted.
            self.passes += 1;
            self.upcoming_passes += u64::from(matches!(start, PassStart::Crossing(_)));
        }
    }
}

struct Evaluator<'a, F> {
    config: &'a SearchConfig,
    satellite_limit: u64,
    total_budget: &'a mut EvaluationBudget,
    elevation_at: F,
    evaluations: u64,
}

impl<E, F: FnMut(NaiveDateTime) -> Result<f64, E>> Evaluator<'_, F> {
    fn evaluate(&mut self, at: NaiveDateTime, phase: SearchPhase) -> Result<f64, SearchStop<E>> {
        let denied = if self.total_budget.remaining() == 0 {
            Some(StopReason::TotalLimit)
        } else if self.evaluations == self.satellite_limit {
            Some(StopReason::SatelliteLimit)
        } else {
            None
        };
        if let Some(reason) = denied {
            return Err(SearchStop { at, phase, reason });
        }
        self.evaluations += 1;
        self.total_budget.used += 1;
        let elevation = (self.elevation_at)(at).map_err(|error| SearchStop {
            at,
            phase,
            reason: StopReason::Evaluation(error),
        })?;
        if !elevation.is_finite() || !(-FRAC_PI_2..=FRAC_PI_2).contains(&elevation) {
            return Err(SearchStop {
                at,
                phase,
                reason: StopReason::InvalidElevation(elevation),
            });
        }
        Ok(elevation)
    }

    fn refine(
        &mut self,
        mut lower: NaiveDateTime,
        mut upper: NaiveDateTime,
        phase: SearchPhase,
    ) -> (CrossingBracket, Option<SearchStop<E>>) {
        let tolerance = self.config.crossing_tolerance;
        while upper - lower > tolerance {
            // Config bounds every bracket to 1957–2100 (< i64::MAX ns).
            // Positive tolerance is at least 1 ns, so midpoint is strictly
            // interior whenever another iteration is required.
            let half_ns = (upper - lower).num_nanoseconds().unwrap() / 2;
            let midpoint = lower + TimeDelta::nanoseconds(half_ns);
            let elevation = match self.evaluate(midpoint, phase) {
                Ok(value) => value,
                Err(stop) => {
                    return (CrossingBracket::new(lower, upper, tolerance), Some(stop));
                }
            };
            let above = elevation > self.config.min_elevation_rad;
            if above == (phase == SearchPhase::RisingRefinement) {
                upper = midpoint;
            } else {
                lower = midpoint;
            }
        }
        (CrossingBracket::new(lower, upper, tolerance), None)
    }
}
