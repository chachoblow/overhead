//! Host-checkable byte oracle for the separate period-cache target comparison.
//! The control redraw is offscreen and is never transferred to the panel.
use overhead_app::RadarApp;
use overhead_render::{CoastLayer, Layout};

use crate::coast_measurement::Screen;

pub fn reference_matches(
    reference: &mut Screen,
    cached: &Screen,
    app: &RadarApp,
    coast: &mut CoastLayer,
) -> bool {
    reference.reset_counts();
    overhead_render::draw(reference, app, Layout::wide(), false, coast).unwrap();
    reference.pixels == cached.pixels
}

#[cfg(test)]
mod tests {
    use super::*;
    use overhead_core::sgp4::chrono::TimeDelta;
    use overhead_render::PeriodCache;

    #[test]
    fn cached_replay_matches_uncached_bytes_across_wrap_and_first_miss() {
        let mut app = crate::coast_measurement::app("iss-ground-20", 0.0, 0.0, 5.0);
        let epoch = app.time().unwrap();
        let mut coast = CoastLayer::prepare(app.observer());
        let mut cached = Screen::new();
        let mut reference = Screen::new();
        let mut cache = PeriodCache::new();
        let mut checksums = [0_u32; 7];
        for tick in 0..21 {
            let index = tick % 7;
            app.update(epoch + TimeDelta::seconds(crate::sharp_test::replay_second(tick)))
                .unwrap();
            assert_eq!(app.selected_id(), Some(25544));
            cached.reset_counts();
            overhead_render::draw_cached(
                &mut cached,
                &app,
                Layout::wide(),
                false,
                &mut coast,
                &mut cache,
            )
            .unwrap();
            assert!(reference_matches(&mut reference, &cached, &app, &mut coast));
            if tick < 7 {
                checksums[index as usize] = cached.checksum();
            } else {
                assert_eq!(cached.checksum(), checksums[index as usize]);
            }
        }
        assert!(coast.error().is_none());
    }
}
