//! Offscreen diagnostic controls for the separate Sharp profile image.
//! Each runs after a real frame was transferred and before the next redraw.
use embedded_graphics::{pixelcolor::BinaryColor, prelude::DrawTarget};
use measurement::{GlyphStage, Stage};
use overhead_app::RadarApp;
use overhead_render::{CoastLayer, measurement};

use crate::coast_measurement::Screen;

#[derive(Clone, Copy, Debug)]
pub enum Control {
    FullClear,
    CoastWithRamClear,
    FrameNoCoast,
}

impl Control {
    pub fn for_tick(tick: u32) -> Self {
        match tick % 3 {
            0 => Self::FullClear,
            1 => Self::CoastWithRamClear,
            _ => Self::FrameNoCoast,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::FullClear => "full_clear",
            Self::CoastWithRamClear => "coast_with_ram_clear",
            Self::FrameNoCoast => "frame_no_coast",
        }
    }

    pub fn run(self, screen: &mut Screen, app: &RadarApp, coast: &mut CoastLayer) {
        self.run_traced(screen, app, coast, |_| {});
    }

    /// The callback only fires for the coast-free frame control. Its elapsed
    /// timestamps are sampled during drawing; report them after it completes.
    pub fn run_traced(
        self,
        screen: &mut Screen,
        app: &RadarApp,
        coast: &mut CoastLayer,
        trace: impl FnMut(Stage),
    ) {
        self.run_traced_fields(screen, app, coast, trace, |_| {});
    }

    pub fn run_traced_fields(
        self,
        screen: &mut Screen,
        app: &RadarApp,
        coast: &mut CoastLayer,
        trace: impl FnMut(Stage),
        glyph_trace: impl FnMut(GlyphStage),
    ) {
        screen.reset_counts();
        match self {
            Self::FullClear => screen.clear(BinaryColor::Off).unwrap(),
            Self::CoastWithRamClear => {
                // Fast RAM clear is included in this control's reported time.
                screen.clear_for_coast();
                assert!(
                    measurement::prepared_coast_only(screen, app, coast).unwrap(),
                    "coast control failed"
                );
            }
            Self::FrameNoCoast => {
                measurement::frame_off_fields_traced(screen, app, false, trace, glyph_trace)
                    .unwrap()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use overhead_core::sgp4::chrono::TimeDelta;
    use overhead_render::Layout;

    #[test]
    fn offscreen_controls_leave_next_real_frame_identical() {
        let mut app = crate::coast_measurement::app("iss-ground-20", 0.0, 0.0, 5.0);
        let epoch = app.time().unwrap();
        let mut coast = CoastLayer::prepare(app.observer());
        let mut screen = Screen::new();
        for frame in 0..7 {
            app.update(epoch + TimeDelta::seconds(crate::sharp_test::replay_second(frame)))
                .unwrap();
            assert_eq!(app.selected_id(), Some(25544));
            overhead_render::draw(&mut screen, &app, Layout::wide(), false, &mut coast).unwrap();
            let original = screen.pixels;
            for tick in 0..3 {
                let mut stages = [None; 9];
                let mut glyph_stages = [None; 5];
                let mut index = 0;
                let mut glyph_index = 0;
                Control::for_tick(tick).run_traced_fields(
                    &mut screen,
                    &app,
                    &mut coast,
                    |stage| {
                        stages[index] = Some(stage);
                        index += 1;
                    },
                    |stage| {
                        glyph_stages[glyph_index] = Some(stage);
                        glyph_index += 1;
                    },
                );
                assert_eq!(index, if tick == 2 { 9 } else { 0 });
                assert_eq!(glyph_index, if tick == 2 { 5 } else { 0 });
                if tick == 2 {
                    assert_eq!(stages[0], Some(Stage::Clear));
                    assert_eq!(stages[8], Some(Stage::Status));
                    assert_eq!(glyph_stages[0], Some(GlyphStage::Separator));
                    assert_eq!(glyph_stages[4], Some(GlyphStage::Period));
                }
                overhead_render::draw(&mut screen, &app, Layout::wide(), false, &mut coast)
                    .unwrap();
                assert_eq!(screen.pixels, original, "frame {frame} control tick {tick}");
            }
        }
        assert!(coast.error().is_none());
    }
}
