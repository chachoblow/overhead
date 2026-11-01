//! Host-testable BOOT input edge filter; no GPIO, app, or scheduling ownership.
//! A completed debounced press emits one selection request. The caller supplies
//! monotonic uptime and the sampled logical pressed level; no polling rate or
//! target latency is guaranteed by this module.

#[derive(Clone, Copy, Debug)]
pub struct CompletedPress {
    debounce_us: u64,
    stable: Option<bool>,
    candidate: bool,
    since_us: u64,
    last_us: Option<u64>,
    armed: bool,
}

impl CompletedPress {
    /// The caller chooses the interval after validating the device's input.
    /// No fixed threshold or GPIO electrical polarity is selected here.
    pub const fn new(debounce_us: u64) -> Option<Self> {
        if debounce_us == 0 {
            return None;
        }
        Some(Self {
            debounce_us,
            stable: None,
            candidate: false,
            since_us: 0,
            last_us: None,
            armed: false,
        })
    }

    /// Return true only after both a new press and its release remain stable
    /// for the configured interval. Starting while pressed does not trigger a
    /// request: release first, then make a new complete press. A clock regression
    /// discards the gesture and requires a new baseline.
    pub fn sample(&mut self, now_us: u64, pressed: bool) -> bool {
        if self.last_us.is_some_and(|last| now_us < last) {
            self.stable = None;
            self.armed = false;
        }
        self.last_us = Some(now_us);
        let Some(stable) = self.stable else {
            self.stable = Some(pressed);
            self.candidate = pressed;
            self.since_us = now_us;
            return false;
        };
        if pressed == stable {
            self.candidate = stable;
            self.since_us = now_us;
            return false;
        }
        if pressed != self.candidate {
            self.candidate = pressed;
            self.since_us = now_us;
            return false;
        }
        if now_us - self.since_us < self.debounce_us {
            return false;
        }
        self.stable = Some(pressed);
        if pressed {
            self.armed = true;
            false
        } else {
            core::mem::replace(&mut self.armed, false)
        }
    }

    /// Return true for a completed press (including one consumed with no valid
    /// frame), for optional caller diagnostics. Never queue it for recovery.
    /// This does not sample GPIO, drive a display, or choose a poll cadence.
    #[cfg(feature = "sharp-radar")]
    pub fn poll_selection(
        &mut self,
        now_us: u64,
        pressed: bool,
        app: &mut overhead_app::RadarApp,
    ) -> bool {
        let completed = self.sample(now_us, pressed);
        apply_presses(u16::from(completed), app);
        completed
    }
}

/// Interrupt-side half of [0070]: periodic samples feed [`CompletedPress`] and
/// completed presses accumulate until the display loop takes them. It owns no
/// GPIO, timer, app or panel; the caller shares it under a critical section.
///
/// [0070]: ../../docs/decisions/0070-boot-input-capture-independent-of-render.md
#[derive(Clone, Copy, Debug)]
pub struct PressLatch {
    filter: CompletedPress,
    pending: u16,
    stats: SampleStats,
    last_us: Option<u64>,
}

/// Sampling evidence since the last [`PressLatch::take_stats`]; diagnostics only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SampleStats {
    pub samples: u32,
    /// Largest spacing between consecutive samples (0 before two samples).
    pub max_gap_us: u64,
}

impl PressLatch {
    pub const fn new(debounce_us: u64) -> Option<Self> {
        match CompletedPress::new(debounce_us) {
            Some(filter) => Some(Self {
                filter,
                pending: 0,
                stats: SampleStats {
                    samples: 0,
                    max_gap_us: 0,
                },
                last_us: None,
            }),
            None => None,
        }
    }

    /// Feed one sample. Completed presses saturate rather than wrap; the
    /// drain runs every loop iteration, so saturation is not expected.
    pub fn sample(&mut self, now_us: u64, pressed: bool) {
        if let Some(last) = self.last_us {
            let gap = now_us.saturating_sub(last);
            self.stats.max_gap_us = self.stats.max_gap_us.max(gap);
        }
        self.last_us = Some(now_us);
        self.stats.samples = self.stats.samples.saturating_add(1);
        if self.filter.sample(now_us, pressed) {
            self.pending = self.pending.saturating_add(1);
        }
    }

    /// Completed presses since the last take; resets the count.
    pub fn take_presses(&mut self) -> u16 {
        core::mem::take(&mut self.pending)
    }

    /// Sample count and largest gap since the last take; resets both. The gap
    /// spanning the take is still measured in the next window.
    pub fn take_stats(&mut self) -> SampleStats {
        core::mem::take(&mut self.stats)
    }
}

/// Display-side half of 0070, applying [0068] to drained presses: each one
/// advances the selection once when a valid frame exists; otherwise all are
/// consumed, not queued. Validity is judged at drain time. Returns the number
/// of selections applied.
///
/// [0068]: ../../docs/decisions/0068-boot-selection-gesture.md
#[cfg(feature = "sharp-radar")]
pub fn apply_presses(presses: u16, app: &mut overhead_app::RadarApp) -> u16 {
    if app.time().is_none() {
        return 0;
    }
    for _ in 0..presses {
        app.select_next();
    }
    presses
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_interval_and_startup_hold() {
        assert!(CompletedPress::new(0).is_none());
        let mut input = CompletedPress::new(10).unwrap();
        assert!(!input.sample(0, true));
        assert!(!input.sample(100, true));
        assert!(!input.sample(101, false));
        assert!(!input.sample(111, false));
        assert!(!input.sample(120, false));
        assert!(!input.sample(121, true));
        assert!(!input.sample(131, true));
        assert!(!input.sample(132, false));
        assert!(input.sample(142, false));
        assert!(!input.sample(152, false));
    }

    #[test]
    fn bounce_hold_and_repeated_polls_emit_one_per_completed_press() {
        let mut input = CompletedPress::new(10).unwrap();
        assert!(!input.sample(0, false));
        assert!(!input.sample(1, true));
        assert!(!input.sample(5, false));
        assert!(!input.sample(6, true));
        assert!(!input.sample(15, true));
        assert!(!input.sample(16, true));
        assert!(!input.sample(100, true));
        assert!(!input.sample(101, false));
        assert!(!input.sample(105, true));
        assert!(!input.sample(106, false));
        assert!(!input.sample(115, false));
        assert!(input.sample(116, false));
        assert!(!input.sample(200, false));
        assert!(!input.sample(201, true));
        assert!(!input.sample(211, true));
        assert!(!input.sample(212, false));
        assert!(input.sample(222, false));
    }

    #[test]
    fn regression_aborts_partial_press_and_never_emits_from_old_history() {
        let mut input = CompletedPress::new(10).unwrap();
        assert!(!input.sample(100, false));
        assert!(!input.sample(101, true));
        assert!(!input.sample(111, true));
        assert!(!input.sample(5, true)); // New baseline while held.
        assert!(!input.sample(15, false));
        assert!(!input.sample(25, false));
        assert!(!input.sample(26, true));
        assert!(!input.sample(36, true));
        assert!(!input.sample(37, false));
        assert!(input.sample(47, false));
    }

    #[cfg(feature = "sharp-radar")]
    #[test]
    fn valid_frame_selection_and_invalid_frame_press_is_not_queued() {
        let mut app = crate::coast_measurement::app("iss-ground-20", 0.0, 0.0, 5.0);
        let mut button = CompletedPress::new(10).unwrap();
        let original_time = app.time().unwrap();
        let id = app.selected_id();
        assert!(id.is_some());
        button.poll_selection(0, false, &mut app);
        button.poll_selection(1, true, &mut app);
        button.poll_selection(11, true, &mut app);
        button.poll_selection(12, false, &mut app);
        assert!(button.poll_selection(22, false, &mut app));
        assert_eq!(app.selected_id(), id); // One in-view marker.
        app.invalidate_frame();
        assert_eq!(app.selected_id(), None);
        button.poll_selection(23, true, &mut app);
        button.poll_selection(33, true, &mut app);
        button.poll_selection(34, false, &mut app);
        assert!(button.poll_selection(44, false, &mut app)); // Consumed without UTC.
        assert_eq!(app.selected_id(), None);
        assert_eq!(app.time(), None);
        app.update(original_time).unwrap();
        assert_eq!(app.selected_id(), id); // Normal acquisition, not queued input.
        button.poll_selection(45, false, &mut app);
        assert_eq!(app.selected_id(), id);

        // A gesture begun on a valid frame but completed after time loss is
        // consumed at completion, not carried into the recovered frame.
        button.poll_selection(46, true, &mut app);
        button.poll_selection(56, true, &mut app);
        app.invalidate_frame();
        button.poll_selection(57, false, &mut app);
        assert!(button.poll_selection(67, false, &mut app));
        app.update(original_time).unwrap();
        assert_eq!(app.selected_id(), id);
        assert!(!button.poll_selection(68, false, &mut app));
    }

    #[cfg(feature = "sharp-radar")]
    #[test]
    fn completed_presses_cycle_multiple_real_markers_and_preserve_zero_view() {
        use overhead_app::{RadarApp, projection::View};
        use overhead_core::{
            GeodeticPosition, OmmElements,
            catalogue::{CatalogueBuilder, RecordOrigin},
            sgp4::chrono::NaiveDate,
        };

        let elements: alloc::vec::Vec<OmmElements> =
            serde_json::from_str(include_str!("../../tools/fixtures/m2-pool/pool.json")).unwrap();
        let mut builder = CatalogueBuilder::new();
        for (record, elements) in elements.into_iter().enumerate() {
            builder.push(RecordOrigin { group: 0, record }, elements);
        }
        let mut app = RadarApp::new(
            builder.finish().unwrap(),
            GeodeticPosition {
                latitude_rad: 39.007_f64.to_radians(),
                longitude_rad: (-104.883_f64).to_radians(),
                altitude_km: 2.187,
            },
            View::rectangular(100.0, 352.0 / 240.0).unwrap(),
        )
        .unwrap();
        let time = NaiveDate::from_ymd_opt(2026, 10, 10)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        app.update(time).unwrap();
        let ids: alloc::vec::Vec<_> = app
            .markers()
            .map(|(entry, _, _)| entry.elements().norad_id)
            .collect();
        assert!(ids.len() > 1, "fixture scene must exercise cycling");
        assert_eq!(app.selected_id(), Some(ids[0]));
        let mut button = CompletedPress::new(10).unwrap();
        button.poll_selection(0, false, &mut app);
        for (i, expected) in ids[1..].iter().chain(&ids[..1]).enumerate() {
            let t = 30 * (i as u64 + 1);
            button.poll_selection(t, true, &mut app);
            button.poll_selection(t + 10, true, &mut app);
            if i == 0 {
                app.update(time).unwrap(); // Frame refresh while BOOT is held.
            }
            button.poll_selection(t + 11, false, &mut app);
            assert!(button.poll_selection(t + 21, false, &mut app));
            assert_eq!(app.selected_id(), Some(*expected));
        }
        let t = 30 * (ids.len() as u64 + 1);
        app.zoom(64);
        assert_eq!(app.counts().visible, 0);
        let retained = app.selected_id();
        button.poll_selection(t, true, &mut app);
        button.poll_selection(t + 10, true, &mut app);
        button.poll_selection(t + 11, false, &mut app);
        button.poll_selection(t + 21, false, &mut app);
        assert_eq!(app.selected_id(), retained);
    }

    #[cfg(feature = "sharp-boot-cycle")]
    #[test]
    fn historical_target_scene_cycles_between_distinct_visible_ids() {
        let mut app = crate::coast_measurement::boot_cycle_app();
        let first = app.selected_id().unwrap();
        let mut screen = crate::coast_measurement::Screen::new();
        let mut coast = overhead_render::CoastLayer::new();
        coast.prepare_for(app.observer());
        overhead_render::draw(
            &mut screen,
            &app,
            overhead_render::Layout::wide(),
            false,
            &mut coast,
        )
        .unwrap();
        let first_pixels = screen.pixels;
        let mut button = CompletedPress::new(20_000).unwrap();
        button.poll_selection(0, false, &mut app);
        button.poll_selection(1, true, &mut app);
        button.poll_selection(20_001, true, &mut app);
        button.poll_selection(20_002, false, &mut app);
        assert!(button.poll_selection(40_002, false, &mut app));
        assert_ne!(app.selected_id(), Some(first));
        overhead_render::draw(
            &mut screen,
            &app,
            overhead_render::Layout::wide(),
            false,
            &mut coast,
        )
        .unwrap();
        assert_ne!(
            screen.pixels, first_pixels,
            "selection must change panel pixels"
        );
        assert!(
            app.markers()
                .any(|(entry, _, _)| Some(entry.elements().norad_id) == app.selected_id())
        );
    }

    // Sampling-gap model: the firmware loops sample BOOT only between blocking
    // render/SPI work, so the filter sees sparse samples. These tests pin what
    // the provisional 20 ms filter guarantees for a clean press of duration P
    // under modelled sample schedules; they do not measure target timing.
    const D: u64 = 20_000;

    fn events(samples: impl Iterator<Item = u64>, pressed: impl Fn(u64) -> bool) -> usize {
        let mut input = CompletedPress::new(D).unwrap();
        samples.filter(|&t| input.sample(t, pressed(t))).count()
    }

    fn regular(step: u64, end: u64) -> impl Iterator<Item = u64> {
        (0..).map(move |k| k * step).take_while(move |&t| t < end)
    }

    /// 5 ms samples, except one blocking gap after the sample at `gap_at`.
    fn with_gap(gap_at: u64, gap: u64, end: u64) -> impl Iterator<Item = u64> {
        core::iter::successors(Some(0), move |&t| {
            let next = t + if t == gap_at { gap } else { 5_000 };
            (next < end).then_some(next)
        })
    }

    /// Repeating cycle of sample offsets (e.g. one 50 ms frame-grid period).
    fn cyclic(period: u64, offsets: &'static [u64], end: u64) -> impl Iterator<Item = u64> {
        (0..)
            .flat_map(move |k| offsets.iter().map(move |o| k * period + o))
            .take_while(move |&t| t < end)
    }

    #[test]
    fn regular_sampling_guarantee_is_ceil_d_over_s_plus_one_samples() {
        // Spacings: idle 5 ms; ~36.6 ms partial frame + 5 ms wait; ~56.7 ms
        // retained full transfer + wait; ~87 ms full-data frame + wait.
        for step in [5_000, 36_600, 56_700, 87_000] {
            let guaranteed = (D.div_ceil(step) + 1) * step;
            for phase in (0..step).step_by(250) {
                let a = 1_000_000 + phase;
                let end = a + 3 * guaranteed + 4 * step;
                for p in [guaranteed, guaranteed + step / 2] {
                    let n = events(regular(step, end), |t| (a..a + p).contains(&t));
                    assert_eq!(n, 1, "step={step} phase={phase} p={p}");
                }
                // Two presses separated by an equally long release stay distinct.
                let g = guaranteed;
                let two = events(regular(step, end + 2 * g), |t| {
                    (a..a + g).contains(&t) || (a + 2 * g..a + 3 * g).contains(&t)
                });
                assert_eq!(two, 2, "step={step} phase={phase}");
                // At or below the interval itself no press can ever qualify.
                assert_eq!(events(regular(step, end), |t| (a..a + D).contains(&t)), 0);
            }
        }
    }

    #[test]
    fn regular_sampling_below_guarantee_can_miss_but_never_overcounts() {
        for step in [36_600, 56_700, 87_000] {
            let guaranteed = (D.div_ceil(step) + 1) * step;
            let p = guaranteed - 1;
            for phase in (0..step).step_by(250) {
                let a = 1_000_000 + phase;
                let n = events(regular(step, a + 4 * guaranteed), |t| {
                    (a..a + p).contains(&t)
                });
                assert!(n <= 1);
            }
            // Worst alignment: the press begins just after a sample.
            let a = (1_000_000 / step + 1) * step + 1;
            let n = events(regular(step, a + 4 * guaranteed), |t| {
                (a..a + p).contains(&t)
            });
            assert_eq!(n, 0, "step={step}: p={p} must be missable");
        }
    }

    #[test]
    fn press_inside_a_blocking_gap_is_missed_and_straddling_needs_gap_plus_d() {
        // Boot-cycle VCOM refresh: ~52.4 ms full transfer between samples.
        let (gap_at, gap) = (1_000_000, 52_400);
        let end = 2_000_000;
        // Entirely hidden: never sampled pressed.
        let hidden = events(with_gap(gap_at, gap, end), |t| {
            (gap_at + 1..gap_at + gap).contains(&t)
        });
        assert_eq!(hidden, 0);
        // Begins just after the last pre-gap sample: first pressed sample is
        // at gap end, so P must exceed gap + D (+ one 5 ms step) to qualify.
        let a = gap_at + 1;
        let short = events(with_gap(gap_at, gap, end), |t| {
            (a..a + gap + D - 1).contains(&t)
        });
        assert_eq!(short, 0);
        let long = events(with_gap(gap_at, gap, end), |t| {
            (a..a + gap + D + 5_000).contains(&t)
        });
        assert_eq!(long, 1);
    }

    #[test]
    fn release_hidden_in_a_gap_merges_two_taps_into_one_event() {
        let (gap_at, gap) = (1_000_000, 52_400);
        // Both taps are sampled pressed across the gap; the release between
        // them is never seen, so the filter reports one completed press.
        let n = events(with_gap(gap_at, gap, 2_000_000), |t| {
            (gap_at - 30_000..gap_at + 10_000).contains(&t)
                || (gap_at + 20_000..gap_at + gap + 40_000).contains(&t)
        });
        assert_eq!(n, 1);
    }

    #[test]
    fn bounced_press_never_overcounts_under_sparse_sampling() {
        // 3 ms of 1 ms contact chatter at both edges.
        for step in [5_000, 36_600, 87_000] {
            let guaranteed = (D.div_ceil(step) + 1) * step + 6_000;
            for phase in (0..step).step_by(250) {
                let a = 1_000_000 + phase;
                let b = a + guaranteed;
                let n = events(regular(step, b + 3 * guaranteed), |t| {
                    let chatter =
                        |e: u64| t >= e && t < e + 3_000 && ((t - e) / 1_000).is_multiple_of(2);
                    (a + 3_000..b - 3_000).contains(&t) || chatter(a) || chatter(b - 3_000)
                });
                assert_eq!(n, 1, "step={step} phase={phase}");
            }
        }
    }

    #[test]
    fn live_utc_partial_frame_cycle_needs_about_two_frames_of_press() {
        // Modelled live-UTC partial-frame loop on the 50 ms grid: a sample
        // before ~31.6 ms of blocking update/render/send, then 5 ms waits.
        // Offsets are an estimate from one bounded capture, not a target bound.
        const OFFSETS: &[u64] = &[0, 36_600, 41_600, 46_600];
        let mut missed_60 = false;
        for phase in (0..50_000).step_by(250) {
            let a = 1_000_000 + phase;
            let end = a + 400_000;
            let p90 = events(cyclic(50_000, OFFSETS, end), |t| {
                (a..a + 90_000).contains(&t)
            });
            assert_eq!(p90, 1, "phase={phase}");
            let p60 = events(cyclic(50_000, OFFSETS, end), |t| {
                (a..a + 60_000).contains(&t)
            });
            assert!(p60 <= 1);
            missed_60 |= p60 == 0;
        }
        assert!(missed_60, "a 60 ms press must be missable in this model");
    }

    // 0070 capture: a 5 ms periodic sampler feeds PressLatch regardless of
    // render/SPI; the display loop drains it at its own (sparse) boundaries.
    fn latch_counts(drain_at: &[u64], end: u64, pressed: impl Fn(u64) -> bool) -> u16 {
        let mut latch = PressLatch::new(D).unwrap();
        let mut drains = drain_at.iter().copied().peekable();
        let mut total = 0;
        for t in regular(5_000, end) {
            while drains.next_if(|&d| d <= t).is_some() {
                total += latch.take_presses();
            }
            latch.sample(t, pressed(t));
        }
        total + latch.take_presses()
    }

    #[test]
    fn latch_keeps_taps_the_polling_loop_misses_or_merges() {
        // Same press shapes as the polling-gap tests, but sampled every 5 ms
        // and drained only around a 52.4 ms blocking transfer.
        let (gap_at, gap) = (1_000_000, 52_400);
        let drains = [gap_at, gap_at + gap, 2_000_000];
        let a = gap_at + 1;
        let hidden = |t: u64| (a..a + 30_000).contains(&t);
        assert_eq!(events(with_gap(gap_at, gap, 2_000_000), hidden), 0);
        assert_eq!(latch_counts(&drains, 2_000_000, hidden), 1);
        let taps = |t: u64| {
            (gap_at - 30_000..gap_at + 10_000).contains(&t)
                || (gap_at + 40_000..gap_at + gap + 40_000).contains(&t)
        };
        assert_eq!(events(with_gap(gap_at, gap, 2_000_000), taps), 1);
        assert_eq!(latch_counts(&drains, 2_000_000, taps), 2);
    }

    #[test]
    fn latch_25_ms_guarantee_holds_with_sparse_or_absent_drains() {
        // Drains every 190 ms (uncached redraw + transfer) or only at the end.
        let sparse: [u64; 20] = core::array::from_fn(|k| k as u64 * 190_000);
        for drains in [&sparse[..], &[]] {
            for phase in (0..5_000).step_by(250) {
                let a = 1_000_000 + phase;
                let end = a + 400_000;
                let press = |t: u64| (a..a + 25_000).contains(&t);
                assert_eq!(latch_counts(drains, end, press), 1, "phase={phase}");
                // Three 25 ms taps, 25 ms apart, all land in one drain window.
                let three =
                    |t: u64| (0..3).any(|k| (a + k * 50_000..a + k * 50_000 + 25_000).contains(&t));
                assert_eq!(latch_counts(drains, end, three), 3, "phase={phase}");
            }
        }
        // Worst alignment for a 24.999 ms press stays missable, never doubled.
        let a = 1_000_001;
        assert_eq!(
            latch_counts(&[], 1_400_000, |t| (a..a + 24_999).contains(&t)),
            0
        );
    }

    #[test]
    fn latch_take_resets_and_saturates_without_wrapping() {
        let mut latch = PressLatch::new(10).unwrap();
        assert!(PressLatch::new(0).is_none());
        assert_eq!(latch.take_stats(), SampleStats::default());
        latch.sample(0, false);
        latch.sample(5, true);
        latch.sample(15, true);
        latch.sample(40, false);
        latch.sample(50, false);
        assert_eq!(latch.take_presses(), 1);
        assert_eq!(latch.take_presses(), 0);
        assert_eq!(
            latch.take_stats(),
            SampleStats {
                samples: 5,
                max_gap_us: 25
            }
        );
        latch.sample(53, false);
        assert_eq!(latch.take_stats().max_gap_us, 3, "gap spanning a take");
        latch.pending = u16::MAX;
        latch.sample(54, true);
        latch.sample(64, true);
        latch.sample(65, false);
        latch.sample(75, false);
        assert_eq!(latch.take_presses(), u16::MAX);
    }

    #[cfg(feature = "sharp-boot-cycle")]
    #[test]
    fn drained_presses_each_advance_and_invalid_frame_consumes_all() {
        let mut app = crate::coast_measurement::boot_cycle_app();
        let ids: alloc::vec::Vec<_> = app
            .markers()
            .map(|(entry, _, _)| entry.elements().norad_id)
            .collect();
        assert!(ids.len() > 2, "historical scene must exercise cycling");
        assert_eq!(app.selected_id(), Some(ids[0]));
        assert_eq!(apply_presses(0, &mut app), 0);
        assert_eq!(app.selected_id(), Some(ids[0]));
        assert_eq!(apply_presses(2, &mut app), 2);
        assert_eq!(app.selected_id(), Some(ids[2]));
        let time = app.time().unwrap();
        app.invalidate_frame();
        assert_eq!(apply_presses(3, &mut app), 0);
        app.update(time).unwrap();
        // Not queued: recovery matches a twin that never saw the 3 presses.
        let mut twin = crate::coast_measurement::boot_cycle_app();
        apply_presses(2, &mut twin);
        twin.invalidate_frame();
        twin.update(time).unwrap();
        assert_eq!(app.selected_id(), twin.selected_id());
        assert!(app.selected_id().is_some());
    }

    #[test]
    fn large_uptime_does_not_overflow() {
        let mut input = CompletedPress::new(10).unwrap();
        assert!(!input.sample(u64::MAX - 25, false));
        assert!(!input.sample(u64::MAX - 24, true));
        assert!(!input.sample(u64::MAX - 14, true));
        assert!(!input.sample(u64::MAX - 11, false));
        assert!(input.sample(u64::MAX - 1, false));
    }
}
