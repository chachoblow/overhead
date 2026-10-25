//! Offline preflight for a future UTC-backed S3 loop; not wired to the replay.
//! No timestamp is invented on a cold boot. The caller must validate an SNTP
//! result before `sync` and check element age separately. All times are integer
//! microseconds: `uptime_us` is monotonic, `utc_unix_us` is UTC since Unix epoch.

pub const FRAME_INTERVAL_US: u64 = 50_000;
pub const IDLE_VCOM_INTERVAL_US: u64 = 500_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DueFrame {
    /// Sample at *actual* time, never at a missed/stale frame deadline.
    pub utc_unix_us: i64,
    /// Scheduled frames passed over since the last presented frame.
    pub skipped: u64,
}

#[derive(Default)]
pub struct UtcFrames {
    anchor: Option<(u64, i64)>,
    next_due: Option<u64>,
}

impl UtcFrames {
    pub const fn new() -> Self {
        Self {
            anchor: None,
            next_due: None,
        }
    }

    /// Only call after obtaining and validating a trusted UTC sample. A later
    /// sync replaces the anchor; expired/invalid time must call `invalidate`.
    pub fn sync(&mut self, uptime_us: u64, utc_unix_us: i64) {
        self.anchor = Some((uptime_us, utc_unix_us));
        self.next_due = Some(uptime_us);
    }

    pub fn invalidate(&mut self) {
        self.anchor = None;
        self.next_due = None;
    }

    /// Skip overrun deadlines without a burst of stale render attempts. The
    /// next deadline stays on the original 50 ms grid; UTC uses elapsed uptime.
    /// Invalid backwards clocks or overflow fail closed until a new sync.
    pub fn due(&mut self, uptime_us: u64) -> Option<DueFrame> {
        let (anchor_uptime, anchor_utc) = self.anchor?;
        let next_due = self.next_due?;
        let elapsed = uptime_us.checked_sub(anchor_uptime)?;
        if uptime_us < next_due {
            return None;
        }
        let utc = anchor_utc.checked_add(i64::try_from(elapsed).ok()?)?;
        let steps = (uptime_us - next_due) / FRAME_INTERVAL_US + 1;
        let next = next_due.checked_add(steps.checked_mul(FRAME_INTERVAL_US)?)?;
        self.next_due = Some(next);
        Some(DueFrame {
            utc_unix_us: utc,
            skipped: steps - 1,
        })
    }
}

/// Serial VCOM is a separate obligation even if no render is due. Invoke
/// `note_transfer` only after a successful clear, full frame or VCOM-only
/// command; a failed SPI operation must be handled by the caller. A blocked
/// main loop can still violate the panel's VCOM limit: this is not a watchdog.
#[derive(Default)]
pub struct VcomService {
    last_transfer: Option<u64>,
    phase: bool,
}

impl VcomService {
    pub const fn new() -> Self {
        Self {
            last_transfer: None,
            phase: false,
        }
    }

    pub fn next_phase(&self) -> bool {
        !self.phase
    }

    pub fn idle_due(&self, uptime_us: u64) -> bool {
        match self.last_transfer {
            None => true,
            Some(last) => uptime_us.saturating_sub(last) >= IDLE_VCOM_INTERVAL_US,
        }
    }

    pub fn note_transfer(&mut self, uptime_us: u64, phase: bool) {
        self.last_transfer = Some(uptime_us);
        self.phase = phase;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_utc_before_sync_or_after_invalidation() {
        let mut frames = UtcFrames::new();
        assert_eq!(frames.due(100), None);
        frames.sync(100, 1_000_000);
        assert_eq!(frames.due(100).unwrap().utc_unix_us, 1_000_000);
        frames.invalidate();
        assert_eq!(frames.due(50_100), None);
    }

    #[test]
    fn slow_draw_skips_without_utc_drift_or_catch_up_burst() {
        let mut frames = UtcFrames::new();
        frames.sync(1_000, 10_000_000);
        assert_eq!(frames.due(1_000).unwrap().skipped, 0);
        assert_eq!(frames.due(50_999), None);
        assert_eq!(frames.due(51_000).unwrap().utc_unix_us, 10_050_000);
        assert_eq!(
            frames.due(201_500),
            Some(DueFrame {
                utc_unix_us: 10_200_500,
                skipped: 2,
            })
        );
        assert_eq!(frames.due(201_500), None);
        assert_eq!(frames.due(251_000).unwrap().skipped, 0);
    }

    #[test]
    fn resync_and_invalid_monotonic_or_utc_arithmetic_fail_closed() {
        let mut frames = UtcFrames::new();
        frames.sync(10, 1_000);
        assert_eq!(frames.due(9), None);
        frames.sync(100, 5_000);
        assert_eq!(frames.due(100).unwrap().utc_unix_us, 5_000);
        frames.sync(0, i64::MAX - 1);
        assert_eq!(frames.due(10), None);
        frames.invalidate();
        assert_eq!(frames.due(10), None);
    }

    #[test]
    fn idle_vcom_is_independent_of_frame_and_alternates_after_success() {
        let mut vcom = VcomService::new();
        assert!(vcom.idle_due(0));
        let first = vcom.next_phase();
        vcom.note_transfer(0, first); // initial clear
        assert!(!vcom.next_phase());
        assert!(!vcom.idle_due(499_999));
        assert!(vcom.idle_due(500_000));
        let second = vcom.next_phase();
        vcom.note_transfer(500_000, second); // VCOM-only command
        assert_ne!(first, second);
        assert!(!vcom.idle_due(550_000));
        vcom.note_transfer(550_000, vcom.next_phase()); // successful frame
        assert!(!vcom.idle_due(1_049_999));
        assert!(vcom.idle_due(1_050_000));
    }
}
