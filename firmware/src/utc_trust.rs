//! Host-preflight UTC quality gate upstream of `TimeLossPreflight`.
//! Observations are injected *after* source screening (see `sntp_screen`);
//! this module does not prove input uncertainty or oscillator drift.
//! Bounds are explicit inputs; `prototype()` supplies 0065's values.

use crate::device_timing::{DiagnosticAction, LifecycleAction, TimeLossPreflight};
use overhead_app::RadarApp;
use overhead_core::{
    sgp4::chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime},
    validate_utc_time,
};

const TARGET_ERROR_US: u64 = 100_000;
/// Prototype drift bound `D` (0065): measured ≈+1.2 ppm on one board over
/// 3 h plus margin for temperature, aging and reference error. Not a
/// verified bound across operating conditions.
pub const PROTOTYPE_DRIFT_PPM: u64 = 20;
/// Prototype holdover cap `H` (0065). The budget check usually ends trust
/// first: with U0 = 30 ms, 20 ppm exhausts 100 ms after 3,500 s.
pub const PROTOTYPE_HOLDOVER_US: u64 = 3_600_000_000;
const PPM_DENOMINATOR: u128 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationOutcome {
    Accepted,
    NoRenewal,
    Revoked,
}

pub use crate::sntp_screen::ScreenedObservation;

/// A fake/externally supplied monotonic clock and explicit bounds; not wired
/// into the historical image or a live device loop. Completion notifications
/// must refer to the action most recently returned by `next`.
pub struct UtcTrustPreflight {
    lifecycle: TimeLossPreflight,
    // midpoint uptime, uncertainty, UTC micros, next UTC month start micros
    anchor: Option<(u64, u64, i64, i64)>,
    last_seen: Option<u64>,
    holdover_us: u64,
    drift_ppm: u64,
    fresh_frame_pending: bool,
    frame_requested: bool,
    completion_regressed: bool,
}

impl UtcTrustPreflight {
    /// `holdover_us > 0` is required; a concrete device value and measured
    /// drift bound are deliberately not supplied by this module.
    pub const fn new(holdover_us: u64, drift_ppm: u64) -> Option<Self> {
        if holdover_us == 0 {
            return None;
        }
        Some(Self {
            lifecycle: TimeLossPreflight::new(),
            anchor: None,
            last_seen: None,
            holdover_us,
            drift_ppm,
            fresh_frame_pending: false,
            frame_requested: false,
            completion_regressed: false,
        })
    }

    /// The 0065 prototype values; tests may still use explicit inputs.
    pub const fn prototype() -> Self {
        match Self::new(PROTOTYPE_HOLDOVER_US, PROTOTYPE_DRIFT_PPM) {
            Some(gate) => gate,
            None => panic!("prototype holdover must be nonzero"),
        }
    }

    fn revoke(&mut self, app: &mut RadarApp) {
        self.anchor = None;
        self.fresh_frame_pending = false;
        self.frame_requested = false;
        self.lifecycle.lose_utc(app);
    }

    /// Checked and rounded up; u128 avoids overflowing u64 age × ppm.
    fn within_budget(&self, age_us: u64, initial_us: u64) -> bool {
        // Reserve some of the target for positive holdover; an observation
        // already at the limit is not an acceptable new anchor.
        if age_us >= self.holdover_us || initial_us >= TARGET_ERROR_US {
            return false;
        }
        let product = u128::from(age_us) * u128::from(self.drift_ppm);
        let drift = product / PPM_DENOMINATOR + u128::from(product % PPM_DENOMINATOR != 0);
        u128::from(initial_us) + drift <= u128::from(TARGET_ERROR_US)
    }

    fn projected_utc(anchor_utc_us: i64, age_us: u64) -> Option<i64> {
        let utc_us = anchor_utc_us.checked_add(i64::try_from(age_us).ok()?)?;
        validate_utc_time(DateTime::from_timestamp_micros(utc_us)?.naive_utc()).ok()?;
        Some(utc_us)
    }

    /// RFC 5905 leaps occur only at the end of a UTC month. Linear holdover
    /// must not cross one: without a fresh observation after it, a
    /// leap could add a whole second of error. `None` if not representable.
    fn next_month_start_us(utc: NaiveDateTime) -> Option<i64> {
        let (year, month) = if utc.month() == 12 {
            (utc.year().checked_add(1)?, 1)
        } else {
            (utc.year(), utc.month() + 1)
        };
        Some(
            NaiveDate::from_ymd_opt(year, month, 1)?
                .and_hms_opt(0, 0, 0)?
                .and_utc()
                .timestamp_micros(),
        )
    }

    /// Inclusive and conservative: the anchor ends once projected UTC is
    /// within the whole error target of the month boundary.
    fn before_leap_boundary(projected_us: i64, boundary_us: i64) -> bool {
        projected_us
            .checked_add(TARGET_ERROR_US as i64)
            .is_some_and(|edge| edge < boundary_us)
    }

    fn still_trusted(&mut self, app: &mut RadarApp, now_us: u64) -> bool {
        if self.completion_regressed || self.last_seen.is_some_and(|last| now_us < last) {
            self.completion_regressed = false;
            self.revoke(app);
            return false;
        }
        self.last_seen = Some(now_us);
        let Some((midpoint, initial, anchor_utc, boundary)) = self.anchor else {
            return false;
        };
        let Some(age) = now_us.checked_sub(midpoint) else {
            self.revoke(app);
            return false;
        };
        let projected = Self::projected_utc(anchor_utc, age);
        if !self.within_budget(age, initial)
            || projected.is_none_or(|utc| !Self::before_leap_boundary(utc, boundary))
        {
            self.revoke(app);
            return false;
        }
        true
    }

    /// No reply, disconnected Wi-Fi, KoD or an unmatched/poor-quality packet:
    /// no renewal. An old anchor may still be used only until its original
    /// expiry. The adapter is responsible for honoring KoD rate/deny rules.
    pub fn no_renewal(&mut self, app: &mut RadarApp, now_us: u64) {
        self.still_trusted(app, now_us);
    }

    /// A *matched* server reply explicitly declaring unsynchronized time is
    /// different from no reply or a too-uncertain synchronized reply.
    pub fn source_unsynchronized(&mut self, app: &mut RadarApp) {
        self.revoke(app);
    }

    /// A matched reply announcing a pending leap (LI=1/2) revokes trust under
    /// 0063: linear holdover cannot keep the UTC budget across the leap.
    pub fn source_leap_pending(&mut self, app: &mut RadarApp) {
        self.revoke(app);
    }

    /// `now_us` is when the screened response is processed, not its midpoint.
    /// A late in-flight response only renews if its actual age and uncertainty
    /// still fit. Any accepted resync discards old app samples immediately.
    pub fn observe(
        &mut self,
        app: &mut RadarApp,
        now_us: u64,
        observation: ScreenedObservation,
    ) -> ObservationOutcome {
        if self.completion_regressed || self.last_seen.is_some_and(|last| now_us < last) {
            self.revoke(app);
            self.completion_regressed = false;
            return ObservationOutcome::Revoked;
        }
        self.still_trusted(app, now_us);
        let Some(age) = now_us.checked_sub(observation.midpoint_uptime_us) else {
            self.revoke(app);
            return ObservationOutcome::Revoked;
        };
        if validate_utc_time(observation.utc_at_midpoint).is_err() {
            self.revoke(app);
            return ObservationOutcome::Revoked;
        }
        let anchor_utc = observation.utc_at_midpoint.and_utc().timestamp_micros();
        let (Some(projected), Some(boundary)) = (
            Self::projected_utc(anchor_utc, age),
            Self::next_month_start_us(observation.utc_at_midpoint),
        ) else {
            self.revoke(app);
            return ObservationOutcome::Revoked;
        };
        // Too close to a possible leap is not an anchor; it is not evidence
        // against an older one either, which keeps its own boundary.
        if !self.within_budget(age, observation.uncertainty_us)
            || !Self::before_leap_boundary(projected, boundary)
        {
            return ObservationOutcome::NoRenewal;
        }
        app.invalidate_frame(); // no old selection/sample, even on a small resync
        if self
            .lifecycle
            .sync_trusted_utc(
                app,
                observation.midpoint_uptime_us,
                observation.utc_at_midpoint,
            )
            .is_err()
        {
            self.revoke(app);
            return ObservationOutcome::Revoked;
        }
        self.anchor = Some((
            observation.midpoint_uptime_us,
            observation.uncertainty_us,
            anchor_utc,
            boundary,
        ));
        self.fresh_frame_pending = true;
        self.frame_requested = false;
        ObservationOutcome::Accepted
    }

    /// Evaluate expiry *before* frame, retained refresh or idle-COM selection.
    pub fn next(&mut self, app: &mut RadarApp, now_us: u64, retained: bool) -> LifecycleAction {
        self.still_trusted(app, now_us);
        let action = self.lifecycle.next(app, now_us, retained);
        if self.anchor.is_some() && action == LifecycleAction::RenderNoFrame {
            self.revoke(app); // inner scheduler found a clock/transfer regression
        }
        self.frame_requested = matches!(
            action,
            LifecycleAction::Scheduled(DiagnosticAction::Frame { .. })
        );
        if self.fresh_frame_pending
            && action == LifecycleAction::Scheduled(DiagnosticAction::RefreshRetained)
        {
            // Retained pixels predate the new anchor. Never resend them while
            // the new frame's transfer has not succeeded.
            return self.lifecycle.next(app, now_us, false);
        }
        action
    }

    pub fn next_phase(&self) -> bool {
        self.lifecycle.next_phase()
    }

    fn note_completion(&mut self, completion_us: u64) {
        if self.last_seen.is_some_and(|last| completion_us < last) {
            self.completion_regressed = true;
        }
        self.last_seen = Some(self.last_seen.unwrap_or(completion_us).max(completion_us));
    }

    pub fn no_frame_succeeded(&mut self, completion_us: u64, phase: bool) {
        self.lifecycle.no_frame_succeeded(completion_us, phase);
        self.note_completion(completion_us);
    }

    pub fn transfer_succeeded(&mut self, completion_us: u64, phase: bool, full: bool) {
        self.lifecycle
            .transfer_succeeded(completion_us, phase, full);
        self.note_completion(completion_us);
        if self.frame_requested {
            self.fresh_frame_pending = false;
        }
        self.frame_requested = false;
    }

    pub fn transfer_failed(&mut self) {
        self.lifecycle.transfer_failed();
        self.frame_requested = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use overhead_core::sgp4::chrono::{NaiveDate, TimeDelta};

    fn epoch() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 26)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
    }

    fn observation(at: u64, uncertainty: u64) -> ScreenedObservation {
        ScreenedObservation {
            midpoint_uptime_us: at,
            utc_at_midpoint: epoch(),
            uncertainty_us: uncertainty,
        }
    }

    fn app() -> RadarApp {
        crate::coast_measurement::app("iss-ground-20", 0.0, 0.0, 5.0)
    }

    #[test]
    fn cold_boot_expiry_between_frames_and_full_no_frame_retry() {
        let mut app = app();
        let mut gate = UtcTrustPreflight::new(200_000, 20).unwrap();
        assert_eq!(gate.next(&mut app, 0, true), LifecycleAction::RenderNoFrame);
        assert_eq!(app.time(), None);
        gate.no_frame_succeeded(1_000, gate.next_phase());
        assert_eq!(
            gate.observe(&mut app, 10_000, observation(10_000, 20_000)),
            ObservationOutcome::Accepted
        );
        assert!(matches!(
            gate.next(&mut app, 10_000, true),
            LifecycleAction::Scheduled(DiagnosticAction::Frame { .. })
        ));
        app.update(epoch()).unwrap();
        gate.transfer_succeeded(20_000, gate.next_phase(), true);
        gate.no_renewal(&mut app, 180_000);
        assert!(matches!(
            gate.next(&mut app, 209_999, true),
            LifecycleAction::Scheduled(DiagnosticAction::Frame { full: false, .. })
        ));
        assert_eq!(
            gate.next(&mut app, 210_000, true),
            LifecycleAction::RenderNoFrame
        );
        assert_eq!(app.time(), None);
        assert_eq!(app.selected_id(), None);
        assert_eq!(app.objects().count(), 0);
        gate.transfer_failed();
        assert_eq!(
            gate.next(&mut app, 210_001, true),
            LifecycleAction::RenderNoFrame
        );
        gate.no_frame_succeeded(262_000, gate.next_phase());
        assert!(!matches!(
            gate.next(&mut app, 262_001, true),
            LifecycleAction::Scheduled(DiagnosticAction::Frame { .. })
        ));
    }

    #[test]
    fn poor_response_does_not_extend_anchor_unsync_revokes_and_recovery_is_fresh() {
        let mut app = app();
        let mut gate = UtcTrustPreflight::new(500_000, 10).unwrap();
        assert_eq!(
            gate.observe(&mut app, 100, observation(100, 10_000)),
            ObservationOutcome::Accepted
        );
        let LifecycleAction::Scheduled(DiagnosticAction::Frame { sample, .. }) =
            gate.next(&mut app, 100, false)
        else {
            panic!("first frame");
        };
        assert_eq!(sample.utc_unix_us, epoch().and_utc().timestamp_micros());
        app.update(epoch()).unwrap();
        gate.transfer_succeeded(110, gate.next_phase(), true);
        assert_eq!(
            gate.observe(&mut app, 200_000, observation(200_000, 100_000)),
            ObservationOutcome::NoRenewal
        );
        assert!(app.time().is_some());
        gate.no_renewal(&mut app, 400_000); // reconnect without a new sync
        assert!(matches!(
            gate.next(&mut app, 500_099, true),
            LifecycleAction::Scheduled(DiagnosticAction::Frame { .. })
        ));
        assert_eq!(
            gate.next(&mut app, 500_100, true),
            LifecycleAction::RenderNoFrame
        );
        let mut recovered = observation(510_000, 10_000);
        recovered.utc_at_midpoint = epoch() + TimeDelta::seconds(10);
        assert_eq!(
            gate.observe(&mut app, 510_001, recovered),
            ObservationOutcome::Accepted
        );
        assert_eq!(app.time(), None);
        let LifecycleAction::Scheduled(DiagnosticAction::Frame { sample, .. }) =
            gate.next(&mut app, 510_001, true)
        else {
            panic!("fresh frame");
        };
        assert_eq!(
            sample.utc_unix_us,
            recovered.utc_at_midpoint.and_utc().timestamp_micros() + 1
        );
        assert_eq!(sample.skipped, 0);
        gate.source_leap_pending(&mut app);
        assert_eq!(app.time(), None);
        assert_eq!(
            gate.next(&mut app, 510_001, true),
            LifecycleAction::RenderNoFrame
        );
        assert_eq!(
            gate.observe(&mut app, 510_001, recovered),
            ObservationOutcome::Accepted
        );
        gate.source_unsynchronized(&mut app);
        assert_eq!(
            gate.next(&mut app, 510_002, true),
            LifecycleAction::RenderNoFrame
        );
    }

    #[test]
    fn budget_age_overflow_regression_and_invalid_time_fail_closed() {
        assert!(UtcTrustPreflight::new(0, 0).is_none());
        let mut app = app();
        let mut gate = UtcTrustPreflight::new(4_000_000_000, 20).unwrap();
        assert_eq!(
            gate.observe(&mut app, 0, observation(0, 20_000)),
            ObservationOutcome::Accepted
        );
        assert!(matches!(
            gate.next(&mut app, 3_999_999_999, false),
            LifecycleAction::Scheduled(_)
        ));
        assert_eq!(
            gate.next(&mut app, 4_000_000_000, true),
            LifecycleAction::RenderNoFrame
        );
        let mut bad = observation(4_000_000_001, 1);
        bad.utc_at_midpoint = NaiveDate::from_ymd_opt(2101, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert_eq!(
            gate.observe(&mut app, 4_000_000_001, bad),
            ObservationOutcome::Revoked
        );
        assert_eq!(
            gate.observe(&mut app, 4_000_000_010, observation(4_000_000_010, 0)),
            ObservationOutcome::Accepted
        );
        assert_eq!(
            gate.next(&mut app, 4_000_000_009, true),
            LifecycleAction::RenderNoFrame
        );
        assert_eq!(
            gate.next(&mut app, 4_000_000_011, true),
            LifecycleAction::RenderNoFrame
        );
        let mut other = UtcTrustPreflight::new(u64::MAX, u64::MAX).unwrap();
        assert_eq!(
            other.observe(&mut app, 1, observation(0, 0)),
            ObservationOutcome::NoRenewal
        );
        assert_eq!(
            other.next(&mut app, u64::MAX, true),
            LifecycleAction::RenderNoFrame
        );
    }

    #[test]
    fn budget_rounds_up_and_holdover_never_crosses_a_possible_leap() {
        let mut app = app();
        let mut gate = UtcTrustPreflight::new(2_000_000, 1).unwrap();
        assert_eq!(
            gate.observe(&mut app, 0, observation(0, 99_999)),
            ObservationOutcome::Accepted
        );
        assert!(matches!(
            gate.next(&mut app, 1_000_000, false),
            LifecycleAction::Scheduled(_)
        ));
        assert_eq!(
            gate.next(&mut app, 1_000_001, true),
            LifecycleAction::RenderNoFrame
        );

        // Month end: trust ends 100 ms before the boundary (inclusive), even
        // with a long H and no drift; a later observation recovers.
        let mut gate = UtcTrustPreflight::new(10_000_000, 0).unwrap();
        let at = |h, m, s, us| {
            NaiveDate::from_ymd_opt(2026, 10, 31)
                .unwrap()
                .and_hms_micro_opt(h, m, s, us)
                .unwrap()
        };
        let mut late = observation(2_000_000, 1);
        late.utc_at_midpoint = at(23, 59, 59, 800_000);
        assert_eq!(
            gate.observe(&mut app, 2_000_000, late),
            ObservationOutcome::Accepted
        );
        assert!(matches!(
            gate.next(&mut app, 2_099_999, false),
            LifecycleAction::Scheduled(_)
        ));
        assert_eq!(
            gate.next(&mut app, 2_100_000, true),
            LifecycleAction::RenderNoFrame
        );
        late.midpoint_uptime_us = 2_100_000;
        late.utc_at_midpoint = at(23, 59, 59, 900_000);
        assert_eq!(
            gate.observe(&mut app, 2_100_000, late),
            ObservationOutcome::NoRenewal
        );
        let mut after = observation(2_300_000, 1);
        after.utc_at_midpoint = NaiveDate::from_ymd_opt(2026, 11, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert_eq!(
            gate.observe(&mut app, 2_300_000, after),
            ObservationOutcome::Accepted
        );
        // December rolls into the next year; 2100's boundary is still bounded.
        let mut last = observation(3_000_000, 1);
        last.utc_at_midpoint = NaiveDate::from_ymd_opt(2100, 12, 31)
            .unwrap()
            .and_hms_opt(23, 0, 0)
            .unwrap();
        assert_eq!(
            gate.observe(&mut app, 3_000_000, last),
            ObservationOutcome::Accepted
        );
    }

    #[test]
    fn prototype_values_end_trust_by_budget_or_cap() {
        let mut app = app();
        // U0 = 30 ms: 70 ms at 20 ppm lasts exactly 3,500 s (inclusive).
        let mut gate = UtcTrustPreflight::prototype();
        assert_eq!(
            gate.observe(&mut app, 0, observation(0, 30_000)),
            ObservationOutcome::Accepted
        );
        assert!(matches!(
            gate.next(&mut app, 3_500_000_000, false),
            LifecycleAction::Scheduled(_)
        ));
        assert_eq!(
            gate.next(&mut app, 3_500_000_001, true),
            LifecycleAction::RenderNoFrame
        );
        // U0 = 1 ms would allow 4,950 s; the 3,600 s cap ends it first.
        let mut gate = UtcTrustPreflight::prototype();
        assert_eq!(
            gate.observe(&mut app, 0, observation(0, 1_000)),
            ObservationOutcome::Accepted
        );
        assert!(matches!(
            gate.next(&mut app, 3_599_999_999, false),
            LifecycleAction::Scheduled(_)
        ));
        assert_eq!(
            gate.next(&mut app, 3_600_000_000, true),
            LifecycleAction::RenderNoFrame
        );
    }

    #[test]
    fn failed_fresh_resync_cannot_refresh_previous_tracking_pixels() {
        let mut app = app();
        let mut gate = UtcTrustPreflight::new(20_000_000, 0).unwrap();
        assert_eq!(
            gate.observe(&mut app, 0, observation(0, 1)),
            ObservationOutcome::Accepted
        );
        assert!(matches!(
            gate.next(&mut app, 0, false),
            LifecycleAction::Scheduled(DiagnosticAction::Frame { .. })
        ));
        app.update(epoch()).unwrap();
        gate.transfer_succeeded(52_000, gate.next_phase(), true);
        let mut jumped = observation(8_000_000, 1);
        jumped.utc_at_midpoint = epoch() + TimeDelta::seconds(10);
        assert_eq!(
            gate.observe(&mut app, 8_000_000, jumped),
            ObservationOutcome::Accepted
        );
        assert_eq!(app.time(), None);
        assert!(matches!(
            gate.next(&mut app, 8_000_000, true),
            LifecycleAction::Scheduled(DiagnosticAction::Frame { .. })
        ));
        gate.transfer_failed();
        assert_eq!(
            gate.next(&mut app, 8_000_001, true),
            LifecycleAction::Scheduled(DiagnosticAction::Wait)
        );
        assert!(matches!(
            gate.next(&mut app, 8_050_000, true),
            LifecycleAction::Scheduled(DiagnosticAction::Frame { full: true, .. })
        ));
        gate.transfer_succeeded(8_102_000, gate.next_phase(), true);
        assert!(!matches!(
            gate.next(&mut app, 8_102_001, true),
            LifecycleAction::Scheduled(DiagnosticAction::RefreshRetained)
        ));
    }

    #[test]
    fn future_midpoint_and_regressed_completion_cannot_preserve_tracking() {
        let mut app = app();
        let mut gate = UtcTrustPreflight::new(1_000_000, 10).unwrap();
        assert_eq!(
            gate.observe(&mut app, 100, observation(101, 1)),
            ObservationOutcome::Revoked
        );
        assert_eq!(
            gate.observe(&mut app, 200, observation(200, 1)),
            ObservationOutcome::Accepted
        );
        assert!(matches!(
            gate.next(&mut app, 200, false),
            LifecycleAction::Scheduled(DiagnosticAction::Frame { .. })
        ));
        app.update(epoch()).unwrap();
        gate.transfer_succeeded(199, gate.next_phase(), true); // impossible completion before poll
        assert_eq!(
            gate.next(&mut app, 201, true),
            LifecycleAction::RenderNoFrame
        );
        assert_eq!(app.time(), None);
        assert_eq!(
            gate.observe(&mut app, 202, observation(202, 1)),
            ObservationOutcome::Accepted
        );
        assert!(matches!(
            gate.next(&mut app, 202, true),
            LifecycleAction::Scheduled(DiagnosticAction::Frame { .. })
        ));
    }

    #[test]
    fn in_flight_midpoint_uses_actual_age_and_reanchor_clears_samples() {
        let mut app = app();
        let mut gate = UtcTrustPreflight::new(100_000, 0).unwrap();
        assert_eq!(
            gate.observe(&mut app, 10_000, observation(0, 20_000)),
            ObservationOutcome::Accepted
        );
        let LifecycleAction::Scheduled(DiagnosticAction::Frame { sample, .. }) =
            gate.next(&mut app, 10_000, false)
        else {
            panic!("frame");
        };
        assert_eq!(
            sample.utc_unix_us,
            epoch().and_utc().timestamp_micros() + 10_000
        );
        app.update(epoch()).unwrap();
        gate.transfer_succeeded(20_000, gate.next_phase(), true);
        assert_eq!(
            gate.observe(&mut app, 50_000, observation(49_000, 20_000)),
            ObservationOutcome::Accepted
        );
        assert_eq!(app.time(), None);
        assert_eq!(
            gate.observe(&mut app, 51_000, observation(50_000, 100_001)),
            ObservationOutcome::NoRenewal
        );
        assert_eq!(
            gate.next(&mut app, 149_000, true),
            LifecycleAction::RenderNoFrame
        );
    }
}
