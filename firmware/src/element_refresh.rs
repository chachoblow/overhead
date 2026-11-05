//! 0075 refresh schedule for fetched elements. Host-tested and clock-agnostic:
//! times are monotonic seconds supplied by the caller; no socket, TLS or UTC.
//!
//! CelesTrak updates GP data every 2 hours, answers a repeat download before
//! an update with 403, and firewalls an address after more than 50 HTTP errors
//! (301/403/404) in 2 hours. So the boundary that matters is whether the GET
//! was sent:
//! - before it (DNS, TCP, TLS setup or a handshake that did not verify-fail)
//!   no HTTP error can be counted, so a short backoff retries;
//! - after it, or on a verification failure, asking again cannot help before
//!   the next slot: a 200 already counted as a download, and an error status
//!   or a bad body will repeat. Those wait one full interval.
//!
//! A handshake failure repeated back to back (for example a TLS 1.3-only
//! server, which this image cannot date-check) also moves to the slot rather
//! than retrying hourly forever.

/// 0002: about every 8 hours.
pub const DEVICE_INTERVAL_S: u64 = 8 * 3600;
/// Shortest permitted interval: CelesTrak's 2-hour update cycle.
pub const MIN_INTERVAL_S: u64 = 2 * 3600;
/// Backoff after consecutive pre-request failures, then capped at the last.
pub const BACKOFF_S: [u64; 4] = [5 * 60, 15 * 60, 30 * 60, 60 * 60];

/// How one fetch attempt ended, classified by the request boundary above.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// HTTP 200, complete body, catalogue ingested and handed over.
    Success,
    /// Failed before the GET was sent and before any TLS handshake reply:
    /// DNS, TCP connect, session setup or a timeout in those stages.
    Transport,
    /// The TLS handshake failed without a certificate-verification failure.
    Handshake,
    /// Certificate verification or dates failed, or anything after the GET
    /// was sent failed (status, head, truncation, parse, ingest, timeout).
    Rejected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Schedule {
    interval_s: u64,
    next_due_s: u64,
    /// Consecutive pre-request failures (Transport or first Handshake).
    backoff_streak: usize,
    last_was_handshake: bool,
}

impl Schedule {
    /// Due immediately (the boot fetch). The interval is clamped to at least
    /// [`MIN_INTERVAL_S`].
    pub const fn new(interval_s: u64) -> Self {
        Self {
            interval_s: if interval_s < MIN_INTERVAL_S {
                MIN_INTERVAL_S
            } else {
                interval_s
            },
            next_due_s: 0,
            backoff_streak: 0,
            last_was_handshake: false,
        }
    }

    pub const fn interval_s(&self) -> u64 {
        self.interval_s
    }

    pub const fn next_due_s(&self) -> u64 {
        self.next_due_s
    }

    pub const fn due(&self, now_s: u64) -> bool {
        now_s >= self.next_due_s
    }

    /// Record an attempt that ended at `now_s`; returns the next due time.
    pub fn record(&mut self, now_s: u64, outcome: Outcome) -> u64 {
        let repeated_handshake = outcome == Outcome::Handshake && self.last_was_handshake;
        self.last_was_handshake = outcome == Outcome::Handshake;
        let delay = match outcome {
            Outcome::Transport | Outcome::Handshake if !repeated_handshake => {
                let delay = BACKOFF_S[self.backoff_streak.min(BACKOFF_S.len() - 1)];
                self.backoff_streak += 1;
                delay
            }
            _ => {
                self.backoff_streak = 0;
                self.interval_s
            }
        };
        if repeated_handshake {
            // Next slot; a later handshake failure starts a fresh pair.
            self.last_was_handshake = false;
        }
        self.next_due_s = now_s.saturating_add(delay);
        self.next_due_s
    }

    /// 0079: at the first trusted UTC after a boot that loaded a stored
    /// catalogue fetched at `fetched_unix_s`. A copy younger than the interval
    /// moves the next fetch to the end of its interval (as if the device had
    /// kept running) and returns that delay from `now_s`; a copy that old, or
    /// dated after `now_unix_s`, leaves the fetch due now (`None`).
    pub fn resume_stored(
        &mut self,
        now_s: u64,
        fetched_unix_s: i64,
        now_unix_s: i64,
    ) -> Option<u64> {
        let age = u64::try_from(now_unix_s.checked_sub(fetched_unix_s)?).ok()?;
        let remaining = self.interval_s.checked_sub(age).filter(|&r| r > 0)?;
        self.next_due_s = now_s.saturating_add(remaining);
        Some(remaining)
    }
}

/// Parses an optional build-time interval in minutes (`REFRESH_MINUTES`).
/// Absent means the device interval; anything unparsable or under the
/// 2-hour minimum is refused rather than silently changed.
pub fn interval_from_minutes(value: Option<&str>) -> Result<u64, &'static str> {
    let Some(value) = value else {
        return Ok(DEVICE_INTERVAL_S);
    };
    let minutes: u64 = value
        .trim()
        .parse()
        .map_err(|_| "REFRESH_MINUTES is not a whole number")?;
    let seconds = minutes.checked_mul(60).ok_or("REFRESH_MINUTES too large")?;
    if seconds < MIN_INTERVAL_S {
        return Err("REFRESH_MINUTES below CelesTrak's 120-minute update cycle");
    }
    Ok(seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: u64 = 3600;

    #[test]
    fn boot_fetch_is_due_at_once_and_success_waits_one_interval() {
        let mut s = Schedule::new(DEVICE_INTERVAL_S);
        assert!(s.due(0));
        assert_eq!(s.record(100, Outcome::Success), 100 + 8 * H);
        assert!(!s.due(100 + 8 * H - 1));
        assert!(s.due(100 + 8 * H));
    }

    #[test]
    fn interval_is_clamped_to_the_update_cycle() {
        assert_eq!(Schedule::new(60).interval_s(), MIN_INTERVAL_S);
        assert_eq!(Schedule::new(3 * H).interval_s(), 3 * H);
    }

    #[test]
    fn pre_request_failures_back_off_then_cap_and_reset_on_success() {
        let mut s = Schedule::new(DEVICE_INTERVAL_S);
        let mut now = 0;
        for expected in [300, 900, 1800, 3600, 3600, 3600] {
            let due = s.record(now, Outcome::Transport);
            assert_eq!(due - now, expected);
            now = due;
        }
        assert_eq!(s.record(now, Outcome::Success) - now, 8 * H);
        assert_eq!(s.record(now, Outcome::Transport) - now, 300);
    }

    #[test]
    fn anything_after_the_request_or_a_verification_failure_waits_the_slot() {
        let mut s = Schedule::new(DEVICE_INTERVAL_S);
        assert_eq!(s.record(10, Outcome::Transport), 10 + 300);
        assert_eq!(s.record(400, Outcome::Rejected), 400 + 8 * H);
        // The rejection also resets the backoff.
        assert_eq!(s.record(H, Outcome::Transport), H + 300);
    }

    #[test]
    fn one_handshake_failure_backs_off_but_a_repeat_waits_the_slot() {
        let mut s = Schedule::new(2 * H);
        assert_eq!(s.record(0, Outcome::Handshake), 300);
        assert_eq!(s.record(300, Outcome::Handshake), 300 + 2 * H);
        // A fresh pair after the slot: back off once more first.
        let t = 300 + 2 * H;
        assert_eq!(s.record(t, Outcome::Handshake), t + 300);
        // Interleaved transport failure breaks the pair.
        let mut s = Schedule::new(2 * H);
        s.record(0, Outcome::Handshake);
        s.record(300, Outcome::Transport);
        assert_eq!(s.record(1200, Outcome::Handshake), 1200 + 1800);
    }

    #[test]
    fn worst_case_requests_stay_bounded() {
        // Only pre-request failures retry early, and those never reach HTTP.
        // Requests that do reach HTTP are at most one per interval.
        let mut s = Schedule::new(MIN_INTERVAL_S);
        let mut now = 0;
        let mut http = 0;
        while now < 24 * H {
            let due = s.record(now, Outcome::Rejected);
            http += 1;
            now = due;
        }
        assert_eq!(http, 12);
    }

    #[test]
    fn a_young_stored_copy_defers_the_boot_fetch_to_its_interval() {
        let fetched = 1_793_000_000;
        let mut s = Schedule::new(8 * H);
        assert_eq!(
            s.resume_stored(20, fetched, fetched + 3 * H as i64),
            Some(5 * H)
        );
        assert!(!s.due(20 + 5 * H - 1));
        assert!(s.due(20 + 5 * H));
        // A just-fetched copy waits the whole interval.
        let mut s = Schedule::new(8 * H);
        assert_eq!(s.resume_stored(0, fetched, fetched), Some(8 * H));
    }

    #[test]
    fn an_old_or_future_stored_copy_leaves_the_fetch_due() {
        let fetched = 1_793_000_000;
        for now_unix in [
            fetched + 8 * H as i64,
            fetched + 30 * 24 * H as i64,
            fetched - 1,
        ] {
            let mut s = Schedule::new(8 * H);
            assert_eq!(s.resume_stored(20, fetched, now_unix), None, "{now_unix}");
            assert!(s.due(20));
        }
        // Extreme values do not overflow.
        let mut s = Schedule::new(8 * H);
        assert_eq!(s.resume_stored(20, i64::MIN, i64::MAX), None);
        assert_eq!(s.resume_stored(20, i64::MAX, i64::MIN), None);
    }

    #[test]
    fn build_time_interval_parsing() {
        assert_eq!(interval_from_minutes(None), Ok(DEVICE_INTERVAL_S));
        assert_eq!(interval_from_minutes(Some("120")), Ok(2 * H));
        assert_eq!(interval_from_minutes(Some(" 480 ")), Ok(8 * H));
        assert!(interval_from_minutes(Some("119")).is_err());
        assert!(interval_from_minutes(Some("2h")).is_err());
        assert!(interval_from_minutes(Some("")).is_err());
        assert!(interval_from_minutes(Some("99999999999999999999")).is_err());
    }
}
