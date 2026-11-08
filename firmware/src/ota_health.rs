//! Boot-time update state and the health check (0086). Host-tested; the
//! target maps the bootloader's `otadata` state into [`SlotState`] and acts
//! on [`Boot`] and [`Verdict`].
//!
//! Before activating an install, the running image records the new build as
//! `pending_build` (0084 settings). At the next boot:
//! - the new image (own build == pending) in `PendingVerify` runs the health
//!   check; passing marks it `Valid` and clears `pending_build`, failing
//!   marks it `Invalid` and resets into the old slot;
//! - any other image that finds a different `pending_build` knows the new one
//!   never confirmed (rolled back after a panic, hang, failed check or power
//!   cut) and records it as `failed_build`, so it is not installed again.
use crate::settings_store::Settings;

/// 0086: a new image must pass every check within 10 minutes of boot.
pub const HEALTH_WINDOW_US: u64 = 10 * 60 * 1_000_000;

/// The running slot's `otadata` state, as the bootloader left it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotState {
    New,
    PendingVerify,
    Valid,
    Invalid,
    Aborted,
    /// Erased `otadata` (a USB flash) or no entry for this slot.
    Undefined,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boot {
    /// Ordinary boot: nothing to confirm.
    Normal,
    /// This image is on probation: run the health check.
    Verify,
    /// The build that was pending never confirmed; it is now `failed_build`.
    RolledBack { failed: u32 },
}

/// Decides the boot and updates `s`; returns whether `s` changed (save it).
pub fn on_boot(own_build: u32, state: SlotState, s: &mut Settings) -> (Boot, bool) {
    let probation = matches!(state, SlotState::PendingVerify | SlotState::New);
    match s.pending_build {
        Some(p) if p != own_build => {
            s.failed_build = Some(p);
            s.pending_build = None;
            (Boot::RolledBack { failed: p }, true)
        }
        Some(_) if probation => (Boot::Verify, false),
        Some(_) => {
            // Already confirmed (the clearing save was cut) or USB-flashed.
            s.pending_build = None;
            (Boot::Normal, true)
        }
        None if probation => (Boot::Verify, false),
        None => (Boot::Normal, false),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Check {
    /// A radar or `NO FRAME` frame reached the panel.
    Frame = 1,
    WifiJoined = 2,
    TrustedUtc = 4,
    CatalogueLoaded = 8,
    /// The channel manifest was fetched and its signature verified.
    ManifestVerified = 16,
}

const ALL: u8 = 31;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Waiting,
    Healthy,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Health {
    started_us: u64,
    met: u8,
}

impl Health {
    pub const fn new(started_us: u64) -> Self {
        Self { started_us, met: 0 }
    }

    pub fn note(&mut self, c: Check) {
        self.met |= c as u8;
    }

    pub fn met(&self, c: Check) -> bool {
        self.met & c as u8 != 0
    }

    /// The checks still missing, as a bit mask (for the log).
    pub fn missing(&self) -> u8 {
        ALL & !self.met
    }

    pub fn verdict(&self, now_us: u64) -> Verdict {
        if self.met == ALL {
            Verdict::Healthy
        } else if now_us.saturating_sub(self.started_us) >= HEALTH_WINDOW_US {
            Verdict::Failed
        } else {
            Verdict::Waiting
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use overhead_update::Channel;

    fn settings(pending: Option<u32>, failed: Option<u32>) -> Settings {
        Settings {
            failed_build: failed,
            pending_build: pending,
            ..Settings::defaults(Channel::Beta)
        }
    }

    #[test]
    fn new_image_verifies() {
        let mut s = settings(Some(7), None);
        assert_eq!(on_boot(7, SlotState::PendingVerify, &mut s), (Boot::Verify, false));
        assert_eq!(s, settings(Some(7), None));
        assert_eq!(on_boot(7, SlotState::New, &mut s), (Boot::Verify, false));
    }

    #[test]
    fn old_image_records_the_rollback() {
        for state in [SlotState::Valid, SlotState::Undefined, SlotState::PendingVerify] {
            let mut s = settings(Some(7), Some(3));
            assert_eq!(on_boot(6, state, &mut s), (Boot::RolledBack { failed: 7 }, true));
            assert_eq!(s, settings(None, Some(7)));
        }
    }

    #[test]
    fn confirmed_or_usb_flashed_clears_pending() {
        let mut s = settings(Some(7), None);
        assert_eq!(on_boot(7, SlotState::Valid, &mut s), (Boot::Normal, true));
        assert_eq!(s, settings(None, None));
        let mut s = settings(Some(7), None);
        assert_eq!(on_boot(7, SlotState::Undefined, &mut s), (Boot::Normal, true));
    }

    #[test]
    fn ordinary_boots() {
        let mut s = settings(None, Some(4));
        assert_eq!(on_boot(5, SlotState::Valid, &mut s), (Boot::Normal, false));
        assert_eq!(on_boot(5, SlotState::Undefined, &mut s), (Boot::Normal, false));
        assert_eq!(on_boot(5, SlotState::PendingVerify, &mut s), (Boot::Verify, false));
        assert_eq!(s, settings(None, Some(4)));
    }

    #[test]
    fn health_needs_every_check_in_time() {
        let mut h = Health::new(1_000);
        assert_eq!(h.verdict(1_000), Verdict::Waiting);
        for c in [Check::Frame, Check::WifiJoined, Check::TrustedUtc, Check::CatalogueLoaded] {
            h.note(c);
            assert_eq!(h.verdict(2_000), Verdict::Waiting);
        }
        assert_eq!(h.missing(), Check::ManifestVerified as u8);
        assert_eq!(h.verdict(1_000 + HEALTH_WINDOW_US), Verdict::Failed);
        h.note(Check::ManifestVerified);
        assert_eq!(h.verdict(1_000 + HEALTH_WINDOW_US - 1), Verdict::Healthy);
        // Once healthy, lateness does not matter.
        assert_eq!(h.verdict(u64::MAX), Verdict::Healthy);
    }
}
