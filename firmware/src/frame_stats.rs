//! Host-testable per-cause frame-cadence accounting for the device runtime's
//! diagnostic; no timers, UART or app ownership. The caller notes what
//! happened between frames and reports each finished frame with the 50 ms
//! deadlines skipped before it and its own work time.
//!
//! A skipped deadline at frame N is caused by whatever ran between frame N−1
//! starting and frame N starting: frame N−1's own work plus everything noted
//! in between. Each skip is therefore charged to the strongest cause of the
//! previous frame and of the interval before this one. Work time is charged
//! to the frame's own cause (the strongest noted since the previous frame,
//! including causes that make this frame heavier, such as a zoom step).

/// Ordered weakest to strongest: when several apply, the strongest is charged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Cause {
    /// Nothing noted.
    Idle,
    /// Only diagnostic UART output (blocking; absent from the device app).
    Log,
    /// The knob moved and the frame re-rasters the coastline (0060).
    Knob,
    /// The selected object changed (glyph transition, period-cache miss).
    Selection,
    /// A full-frame or retained full refresh was sent.
    Full,
    /// An SNTP observation was applied.
    Sntp,
    /// A fetched catalogue was loaded (forces a complete update).
    Load,
    /// An element fetch was in flight (TLS runs on the same core).
    Fetch,
    /// Fetched elements were being saved to flash (0079): each sector erase
    /// or program holds interrupts off on this core.
    Store,
}

pub const CAUSES: [Cause; 9] = [
    Cause::Idle,
    Cause::Log,
    Cause::Knob,
    Cause::Selection,
    Cause::Full,
    Cause::Sntp,
    Cause::Load,
    Cause::Fetch,
    Cause::Store,
];

impl Cause {
    pub const fn name(self) -> &'static str {
        match self {
            Cause::Idle => "idle",
            Cause::Log => "log",
            Cause::Knob => "knob",
            Cause::Selection => "select",
            Cause::Full => "full",
            Cause::Sntp => "sntp",
            Cause::Load => "load",
            Cause::Fetch => "fetch",
            Cause::Store => "store",
        }
    }
}

/// Work-time histogram in 1 ms buckets; the last bucket holds ≥ `BUCKETS-1` ms.
pub const BUCKETS: usize = 256;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Window {
    pub frames: u64,
    /// Skipped deadlines charged to this cause.
    pub skipped: u64,
    pub max_work_us: u64,
}

#[derive(Clone, Copy)]
pub struct Totals {
    pub window: Window,
    pub histogram: [u32; BUCKETS],
}

impl Totals {
    const EMPTY: Self = Self {
        window: Window {
            frames: 0,
            skipped: 0,
            max_work_us: 0,
        },
        histogram: [0; BUCKETS],
    };

    /// Smallest whole-millisecond bound below which at least `per_mille`/1000
    /// of this cause's frames' work fell; `None` without frames. Overflowing
    /// frames report `BUCKETS` (meaning "at least that").
    pub fn percentile_ms(&self, per_mille: u64) -> Option<u64> {
        let frames: u64 = self.histogram.iter().map(|&c| u64::from(c)).sum();
        if frames == 0 {
            return None;
        }
        let rank = (frames * per_mille).div_ceil(1000).max(1);
        let mut seen = 0;
        for (ms, &count) in self.histogram.iter().enumerate() {
            seen += u64::from(count);
            if seen >= rank {
                return Some(ms as u64 + 1);
            }
        }
        unreachable!("rank within total")
    }
}

pub struct FrameStats {
    /// Strongest cause noted since the previous frame was recorded.
    pending: Cause,
    /// The previous frame's own cause.
    carry: Cause,
    window: [Window; CAUSES.len()],
    boot: [Totals; CAUSES.len()],
}

impl Default for FrameStats {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameStats {
    pub const fn new() -> Self {
        Self {
            pending: Cause::Idle,
            carry: Cause::Idle,
            window: [Window {
                frames: 0,
                skipped: 0,
                max_work_us: 0,
            }; CAUSES.len()],
            boot: [Totals::EMPTY; CAUSES.len()],
        }
    }

    /// Something that happened since the previous frame (or that makes the
    /// next frame heavier). Several notes keep only the strongest.
    pub fn note(&mut self, cause: Cause) {
        self.pending = self.pending.max(cause);
    }

    /// A frame finished: `skipped` deadlines passed before it started and its
    /// update/render/send took `work_us`; `own` is a cause known only now
    /// (for example a full transfer). Returns the cause its skips were charged to.
    pub fn frame(&mut self, skipped: u64, work_us: u64, own: Cause) -> Cause {
        let own = self.pending.max(own);
        let charged = own.max(self.carry);
        self.window[charged as usize].skipped += skipped;
        let w = &mut self.window[own as usize];
        w.frames += 1;
        w.max_work_us = w.max_work_us.max(work_us);
        let b = &mut self.boot[own as usize];
        b.window.frames += 1;
        b.window.max_work_us = b.window.max_work_us.max(work_us);
        let bucket = usize::try_from(work_us / 1000)
            .unwrap_or(BUCKETS - 1)
            .min(BUCKETS - 1);
        b.histogram[bucket] = b.histogram[bucket].saturating_add(1);
        self.boot[charged as usize].window.skipped += skipped;
        self.carry = own;
        self.pending = Cause::Idle;
        charged
    }

    /// The current window per cause, then cleared.
    pub fn take_window(&mut self) -> [Window; CAUSES.len()] {
        core::mem::take(&mut self.window)
    }

    pub fn boot(&self, cause: Cause) -> &Totals {
        &self.boot[cause as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_frames_count_without_skips() {
        let mut s = FrameStats::new();
        for _ in 0..3 {
            assert_eq!(s.frame(0, 40_000, Cause::Idle), Cause::Idle);
        }
        let w = s.take_window();
        assert_eq!(
            w[Cause::Idle as usize],
            Window {
                frames: 3,
                skipped: 0,
                max_work_us: 40_000
            }
        );
        assert_eq!(s.take_window()[Cause::Idle as usize], Window::default());
        assert_eq!(s.boot(Cause::Idle).window.frames, 3);
    }

    #[test]
    fn skip_is_charged_to_the_heavy_previous_frame() {
        let mut s = FrameStats::new();
        s.note(Cause::Knob);
        assert_eq!(s.frame(0, 60_000, Cause::Idle), Cause::Knob);
        // The next frame starts late because the zoom frame overran.
        assert_eq!(s.frame(1, 40_000, Cause::Idle), Cause::Knob);
        // And the carry ends after one frame.
        assert_eq!(s.frame(1, 40_000, Cause::Idle), Cause::Idle);
        let w = s.take_window();
        assert_eq!(
            (
                w[Cause::Knob as usize].frames,
                w[Cause::Knob as usize].skipped
            ),
            (1, 1)
        );
        assert_eq!(
            (
                w[Cause::Idle as usize].frames,
                w[Cause::Idle as usize].skipped
            ),
            (2, 1)
        );
    }

    #[test]
    fn interval_events_charge_the_frame_they_delay() {
        let mut s = FrameStats::new();
        s.frame(0, 40_000, Cause::Idle);
        s.note(Cause::Log);
        s.note(Cause::Sntp);
        s.note(Cause::Log);
        assert_eq!(s.frame(2, 40_000, Cause::Idle), Cause::Sntp);
        let w = s.take_window();
        assert_eq!(w[Cause::Sntp as usize].skipped, 2);
        assert_eq!(w[Cause::Sntp as usize].frames, 1);
        assert_eq!(w[Cause::Log as usize], Window::default());
    }

    #[test]
    fn own_cause_reported_with_the_frame_counts() {
        let mut s = FrameStats::new();
        s.note(Cause::Log);
        assert_eq!(s.frame(0, 55_000, Cause::Full), Cause::Full);
        assert_eq!(s.frame(1, 40_000, Cause::Idle), Cause::Full);
        assert_eq!(s.boot(Cause::Full).window.skipped, 1);
        assert_eq!(s.boot(Cause::Full).window.frames, 1);
    }

    #[test]
    fn strongest_cause_wins() {
        let mut s = FrameStats::new();
        s.note(Cause::Fetch);
        s.note(Cause::Knob);
        assert_eq!(s.frame(3, 40_000, Cause::Full), Cause::Fetch);
    }

    #[test]
    fn percentiles_from_millisecond_buckets() {
        let mut s = FrameStats::new();
        assert_eq!(s.boot(Cause::Idle).percentile_ms(500), None);
        for us in [10_500, 20_100, 20_900, 30_000, 999_000] {
            s.frame(0, us, Cause::Idle);
        }
        let t = s.boot(Cause::Idle);
        assert_eq!(t.percentile_ms(1), Some(11));
        assert_eq!(t.percentile_ms(500), Some(21));
        assert_eq!(t.percentile_ms(800), Some(31));
        assert_eq!(t.percentile_ms(1000), Some(BUCKETS as u64));
        assert_eq!(t.window.max_work_us, 999_000);
    }
}
