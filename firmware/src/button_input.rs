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
        if completed && app.time().is_some() {
            app.select_next();
        }
        completed
    }
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
