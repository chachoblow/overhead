//! Offline preflight for a future UTC-backed S3 loop; not wired to the replay.
//! No timestamp is invented on a cold boot. The caller must validate an SNTP
//! result before `sync` and check element age separately. All times are integer
//! microseconds: `uptime_us` is monotonic, `utc_unix_us` is UTC since Unix epoch.

pub const FRAME_INTERVAL_US: u64 = 50_000;
pub const IDLE_VCOM_INTERVAL_US: u64 = 500_000;
// Diagnostic interval from sharp-lines v1, not a production refresh policy.
pub const DIAGNOSTIC_FULL_INTERVAL_US: u64 = 8_000_000;

/// Explicit historical replay only: wrap a 300-second fixture excerpt without
/// modifying the frame deadline or presenting this sample as a live UTC clock.
#[cfg(feature = "sharp-natural")]
pub fn historical_offset_us(sample_unix_us: i64, fixture_epoch_unix_us: i64) -> Option<i64> {
    Some(
        sample_unix_us
            .checked_sub(fixture_epoch_unix_us)?
            .rem_euclid(300_000_000),
    )
}

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

/// Full-data refresh is independent of the number of rendered frames. A caller
/// must retain a confirmed framebuffer to resend when UTC is unavailable; an
/// idle VCOM command does not count as a data refresh. Call `note_full` only
/// after a completed full transfer, using its completion uptime. This reminder
/// cannot preempt a blocked loop or guarantee the panel's still-image limit.
#[derive(Default)]
pub struct FullRefresh {
    last_full: Option<u64>,
}

impl FullRefresh {
    pub const fn new() -> Self {
        Self { last_full: None }
    }

    pub fn due(&self, uptime_us: u64) -> bool {
        match self.last_full {
            None => true,
            Some(last) => uptime_us
                .checked_sub(last)
                .is_none_or(|elapsed| elapsed >= DIAGNOSTIC_FULL_INTERVAL_US),
        }
    }

    pub fn note_full(&mut self, completion_uptime_us: u64) {
        self.last_full = Some(completion_uptime_us);
    }

    /// Unknown panel state after a failed transaction requires a full resync.
    pub fn invalidate(&mut self) {
        self.last_full = None;
    }
}

/// Host-preflight action priority for a future fixture diagnostic. The caller
/// supplies whether it has confirmed pixels to resend; no clock is created
/// here. A live caller must validate UTC before calling `sync_utc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticAction {
    Frame { sample: DueFrame, full: bool },
    RefreshRetained,
    IdleVcom,
    Wait,
}

#[derive(Default)]
pub struct DiagnosticSchedule {
    frames: UtcFrames,
    vcom: VcomService,
    full: FullRefresh,
    panel_unknown: bool,
}

impl DiagnosticSchedule {
    pub const fn new() -> Self {
        Self {
            frames: UtcFrames::new(),
            vcom: VcomService::new(),
            full: FullRefresh::new(),
            panel_unknown: false,
        }
    }

    pub fn sync_utc(&mut self, uptime_us: u64, validated_utc_unix_us: i64) {
        self.frames.sync(uptime_us, validated_utc_unix_us);
    }

    pub fn invalidate_utc(&mut self) {
        self.frames.invalidate();
    }

    pub fn next_phase(&self) -> bool {
        self.vcom.next_phase()
    }

    /// Call only when ready to execute the returned action. A full-data
    /// reminder takes priority over idle VCOM; frames use actual start time.
    /// With no UTC and no retained pixels, only COM service can be requested.
    pub fn next(&mut self, uptime_us: u64, retained_pixels: bool) -> DiagnosticAction {
        if let Some(sample) = self.frames.due(uptime_us) {
            return DiagnosticAction::Frame {
                sample,
                full: self.full.due(uptime_us),
            };
        }
        if self.full.due(uptime_us) && retained_pixels {
            return DiagnosticAction::RefreshRetained;
        }
        if self.panel_unknown {
            return DiagnosticAction::Wait; // never send idle COM with unknown panel state
        }
        if self.vcom.idle_due(uptime_us) {
            return DiagnosticAction::IdleVcom;
        }
        DiagnosticAction::Wait
    }

    /// No credit on failure. A failed/partial command leaves panel state
    /// unknown; the caller must recover with a full transfer, not idle COM.
    pub fn transfer_failed(&mut self) {
        self.full.invalidate();
        self.panel_unknown = true;
    }

    /// Record *completion* uptime and phase only after a successful command.
    pub fn transfer_succeeded(&mut self, uptime_us: u64, phase: bool, full: bool) {
        assert!(
            !self.panel_unknown || full,
            "unknown panel requires full resync"
        );
        self.vcom.note_transfer(uptime_us, phase);
        if full {
            self.full.note_full(uptime_us);
            self.panel_unknown = false;
        }
    }
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

    #[test]
    fn diagnostic_full_is_uptime_based_not_tick_based() {
        let mut refresh = FullRefresh::new();
        assert!(refresh.due(0));
        refresh.note_full(82_000);
        assert!(!refresh.due(8_081_999));
        assert!(refresh.due(8_082_000));
        // Missed deadlines do not cause a burst of full transactions.
        refresh.note_full(8_164_000);
        assert!(!refresh.due(8_164_001));
        assert!(refresh.due(16_164_000));
        refresh.invalidate();
        assert!(refresh.due(16_164_001));
        // A backwards/invalid monotonic sample cannot postpone a refresh.
        refresh.note_full(20_000_000);
        assert!(refresh.due(19_000_000));
    }

    #[test]
    fn natural_speed_deadlines_full_overrun_and_idle_without_utc() {
        let mut frames = UtcFrames::new();
        let mut vcom = VcomService::new();
        let mut refresh = FullRefresh::new();
        // A historical fixture anchor is explicit test data, NOT trusted SNTP.
        assert_eq!(frames.due(0), None);
        assert!(vcom.idle_due(0));
        vcom.note_transfer(0, vcom.next_phase()); // successful clear
        frames.sync(0, 1_000_000_000);
        let first = frames.due(0).unwrap();
        assert_eq!((first.utc_unix_us, first.skipped), (1_000_000_000, 0));
        assert!(refresh.due(0));
        // A measured ~82 ms full transaction overruns the 50 ms frame grid.
        refresh.note_full(82_000);
        vcom.note_transfer(82_000, vcom.next_phase());
        assert_eq!(frames.due(82_000).unwrap().utc_unix_us, 1_000_082_000);
        assert_eq!(frames.due(82_000), None); // no catch-up burst
        assert_eq!(frames.due(100_000).unwrap().utc_unix_us, 1_000_100_000);
        vcom.note_transfer(136_000, vcom.next_phase()); // partial frame completed
        frames.invalidate(); // time quality lost; no new orbit samples
        assert_eq!(frames.due(150_000), None);
        assert!(!vcom.idle_due(635_999));
        assert!(vcom.idle_due(636_000));
        let before_idle = vcom.next_phase();
        vcom.note_transfer(636_000, before_idle);
        assert_ne!(vcom.next_phase(), before_idle);
        // Continue servicing idle COM while UTC is invalid and no frames render.
        for uptime in (1_136_000..8_082_000).step_by(500_000) {
            assert!(vcom.idle_due(uptime));
            vcom.note_transfer(uptime, vcom.next_phase());
            assert_eq!(frames.due(uptime), None);
        }
        // Neither idle VCOM nor invalid UTC resets the data-refresh clock.
        assert!(!vcom.idle_due(8_082_000));
        assert!(refresh.due(8_082_000));
        refresh.note_full(8_164_000); // resend confirmed retained pixels, if available
        vcom.note_transfer(8_164_000, vcom.next_phase());
        assert!(!refresh.due(8_164_001));
        assert_eq!(frames.due(8_164_001), None);
        // Failed full/partial transfer: no completion credit; next send must resync.
        refresh.invalidate();
        assert!(refresh.due(8_164_002));
    }

    #[cfg(feature = "sharp-natural")]
    #[test]
    fn historical_replay_uses_actual_elapsed_time_and_wraps_only_fixture() {
        let epoch = 1_700_000_000_000_000_i64;
        assert_eq!(historical_offset_us(epoch, epoch), Some(0));
        assert_eq!(historical_offset_us(epoch + 50_000, epoch), Some(50_000));
        assert_eq!(
            historical_offset_us(epoch + 299_999_999, epoch),
            Some(299_999_999)
        );
        assert_eq!(historical_offset_us(epoch + 300_000_000, epoch), Some(0));
        assert_eq!(historical_offset_us(i64::MIN, i64::MAX), None);
    }

    #[test]
    fn combined_policy_prioritizes_frame_full_retained_refresh_and_idle() {
        let mut schedule = DiagnosticSchedule::new();
        assert_eq!(schedule.next(0, false), DiagnosticAction::IdleVcom);
        let phase = schedule.next_phase();
        schedule.transfer_succeeded(0, phase, false); // clear, no full data
        assert_eq!(schedule.next(1, false), DiagnosticAction::Wait);
        schedule.sync_utc(0, 1_000_000_000); // explicit host fixture anchor
        assert_eq!(
            schedule.next(0, false),
            DiagnosticAction::Frame {
                sample: DueFrame {
                    utc_unix_us: 1_000_000_000,
                    skipped: 0
                },
                full: true
            }
        );
        schedule.transfer_succeeded(82_000, schedule.next_phase(), true);
        assert_eq!(
            schedule.next(82_000, true),
            DiagnosticAction::Frame {
                sample: DueFrame {
                    utc_unix_us: 1_000_082_000,
                    skipped: 0
                },
                full: false
            }
        );
        schedule.transfer_succeeded(118_000, schedule.next_phase(), false);
        schedule.invalidate_utc();
        assert_eq!(schedule.next(617_999, true), DiagnosticAction::Wait);
        assert_eq!(schedule.next(618_000, true), DiagnosticAction::IdleVcom);
        schedule.transfer_succeeded(618_085, schedule.next_phase(), false);
        assert_eq!(
            schedule.next(8_082_000, true),
            DiagnosticAction::RefreshRetained
        );
        schedule.transfer_succeeded(8_164_000, schedule.next_phase(), true);
        assert_eq!(schedule.next(8_164_001, true), DiagnosticAction::Wait);
        schedule.transfer_failed();
        assert_eq!(schedule.next(8_164_002, false), DiagnosticAction::Wait);
        assert_eq!(
            schedule.next(8_164_002, true),
            DiagnosticAction::RefreshRetained
        );
        // Failure did not advance phase; successful full resync does.
        let recovery_phase = schedule.next_phase();
        schedule.transfer_succeeded(8_246_000, recovery_phase, true);
        assert_eq!(schedule.next_phase(), !recovery_phase);
        assert_eq!(schedule.next(8_246_001, true), DiagnosticAction::Wait);
    }

    #[cfg(feature = "sharp-lines")]
    #[test]
    fn failed_partial_requires_sender_resync_without_vcom_or_refresh_credit() {
        use crate::{
            sharp_lines::{Kind, LineSender},
            sharp_test::FRAME_BYTES,
        };
        let mut sender = LineSender::new();
        let mut refresh = FullRefresh::new();
        let mut vcom = VcomService::new();
        let mut pixels = [0; FRAME_BYTES];
        let phase = vcom.next_phase();
        assert_eq!(
            sender
                .send(phase, &pixels, refresh.due(0), |_| Ok::<_, ()>(()))
                .unwrap()
                .kind,
            Kind::Full
        );
        refresh.note_full(82_000);
        vcom.note_transfer(82_000, phase);
        pixels[0] = 1;
        let mut writes = 0;
        assert_eq!(
            sender.send(vcom.next_phase(), &pixels, refresh.due(100_000), |_| {
                writes += 1;
                if writes == 2 { Err(()) } else { Ok(()) }
            }),
            Err(())
        );
        // A failed partial leaves panel pixels unknown, despite a recent full.
        refresh.invalidate();
        assert_eq!(vcom.next_phase(), !phase); // failed transaction did not earn a toggle
        assert!(refresh.due(100_001));
        let recovered_phase = vcom.next_phase();
        let recovered = sender
            .send(recovered_phase, &pixels, refresh.due(100_001), |_| {
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(recovered.kind, Kind::Full);
        refresh.note_full(182_001);
        vcom.note_transfer(182_001, recovered_phase);
        assert!(!refresh.due(182_002));
        assert!(!vcom.idle_due(182_002));
    }
}
