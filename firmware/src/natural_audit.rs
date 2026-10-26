//! Opt-in command-side telemetry for the historical fixture, not a panel COM
//! measurement or a guarantee that a blocked loop can service the display.
use crate::device_timing::FRAME_INTERVAL_US;

#[derive(Default)]
pub struct NaturalAudit {
    pub commands: u64,
    pub full_commands: u64,
    pub frames: u64,
    pub over_budget_frames: u64,
    pub skipped_deadlines: u64,
    pub max_frame_us: u64,
    pub max_full_us: u64,
    /// Start-to-start gap between completed commands (not physical COM edges).
    pub max_command_start_gap_us: u64,
    /// Last completion to next command start (excludes transfer duration).
    pub max_no_command_us: u64,
    last_start: Option<u64>,
    last_completion: Option<u64>,
    last_phase: Option<bool>,
}

impl NaturalAudit {
    pub const fn new() -> Self {
        Self {
            commands: 0,
            full_commands: 0,
            frames: 0,
            over_budget_frames: 0,
            skipped_deadlines: 0,
            max_frame_us: 0,
            max_full_us: 0,
            max_command_start_gap_us: 0,
            max_no_command_us: 0,
            last_start: None,
            last_completion: None,
            last_phase: None,
        }
    }

    /// Credit only a successful command. Reject non-monotonic times and a
    /// repeated VCOM command bit. The caller halts instead of misreporting.
    pub fn completed_command(&mut self, start: u64, done: u64, phase: bool, full: bool) -> bool {
        let Some(duration) = done.checked_sub(start) else {
            return false;
        };
        if let Some(last_start) = self.last_start {
            let Some(start_gap) = start.checked_sub(last_start) else {
                return false;
            };
            let Some(no_command) = start.checked_sub(self.last_completion.unwrap()) else {
                return false;
            };
            if self.last_phase == Some(phase) {
                return false;
            }
            self.max_command_start_gap_us = self.max_command_start_gap_us.max(start_gap);
            self.max_no_command_us = self.max_no_command_us.max(no_command);
        }
        self.commands += 1;
        if full {
            self.full_commands += 1;
            self.max_full_us = self.max_full_us.max(duration);
        }
        self.last_start = Some(start);
        self.last_completion = Some(done);
        self.last_phase = Some(phase);
        true
    }

    /// End-to-end frame occupancy includes update, draw and completed send.
    /// Budget exceedance is not identical to skipped frame-grid deadlines.
    pub fn completed_frame(&mut self, start: u64, done: u64, skipped: u64) -> bool {
        let Some(duration) = done.checked_sub(start) else {
            return false;
        };
        self.frames += 1;
        self.skipped_deadlines += skipped;
        self.over_budget_frames += u64::from(duration > FRAME_INTERVAL_US);
        self.max_frame_us = self.max_frame_us.max(duration);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credits_completed_commands_and_separates_frame_overrun_from_skips() {
        let mut audit = NaturalAudit::new();
        assert!(audit.completed_command(0, 20, true, false)); // clear
        assert!(audit.completed_command(100, 52_100, false, true));
        assert!(audit.completed_frame(40, 52_100, 0));
        assert!(audit.completed_command(100_000, 100_100, true, false));
        assert!(audit.completed_frame(70_000, 100_100, 1));
        assert!(audit.completed_command(8_200_000, 8_254_000, false, true)); // retained full
        assert_eq!(
            (audit.commands, audit.full_commands, audit.frames),
            (4, 2, 2)
        );
        assert_eq!((audit.over_budget_frames, audit.skipped_deadlines), (1, 1));
        assert_eq!(audit.max_frame_us, 52_060);
        assert_eq!(audit.max_full_us, 54_000);
        assert_eq!(audit.max_command_start_gap_us, 8_100_000);
        assert_eq!(audit.max_no_command_us, 8_099_900);
    }

    #[test]
    fn rejects_repeated_phase_and_invalid_time_without_credit() {
        let mut audit = NaturalAudit::new();
        assert!(!audit.completed_command(20, 19, true, true));
        assert!(audit.completed_command(20, 30, true, false));
        assert!(!audit.completed_command(31, 32, true, true));
        assert!(!audit.completed_command(29, 35, false, true));
        assert!(!audit.completed_frame(100, 99, 7));
        assert_eq!(
            (audit.commands, audit.full_commands, audit.frames),
            (1, 0, 0)
        );
        assert_eq!(audit.skipped_deadlines, 0);
        assert!(audit.completed_command(40, 50, false, true));
    }
}
