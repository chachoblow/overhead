//! Measurement-only adapter for the prepared, cached wide coastline layer
//! (decisions 0058/0059). Same scene fixtures and 1-bit RAM target as the
//! earlier representation image; not the device app or a Sharp driver.
use crate::coast_measurement::{self, Screen};
use crate::representation_measurement::ASSET;
use overhead_app::RadarApp;
use overhead_render::{CoastLayer, Layout, measurement};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// `CoastLayer::prepare` into caller-owned storage.
    Prepare,
    /// Coast only, cache discarded first: the cost of one zoom step.
    CoastRaster,
    /// Coast only from a valid cache: the steady-state cost.
    CoastCached,
    /// Complete frame after discarding the cache.
    FrameRaster,
    /// Complete frame from a valid cache.
    FrameCached,
    /// Complete frame without coastline (same control as earlier images).
    FrameOff,
}

pub const MODES: [Mode; 6] = [
    Mode::Prepare,
    Mode::CoastRaster,
    Mode::CoastCached,
    Mode::FrameRaster,
    Mode::FrameCached,
    Mode::FrameOff,
];

/// Run one invocation. `Prepare` replaces `layer`; others reuse it. Coast-only
/// modes expect the caller to clear the target first.
pub fn run(screen: &mut Screen, app: &RadarApp, layer: &mut Option<CoastLayer>, mode: Mode) {
    match mode {
        Mode::Prepare => {
            *layer = Some(CoastLayer::prepare(app.observer()));
            assert!(
                layer.as_ref().unwrap().error().is_none(),
                "preparation failed"
            );
        }
        Mode::CoastRaster | Mode::CoastCached | Mode::FrameRaster | Mode::FrameCached => {
            let layer = layer.as_mut().expect("prepared layer");
            if matches!(mode, Mode::CoastRaster | Mode::FrameRaster) {
                measurement::invalidate_coast_cache(layer);
            }
            if matches!(mode, Mode::CoastRaster | Mode::CoastCached) {
                assert!(measurement::prepared_coast_only(screen, app, layer).unwrap());
            } else {
                overhead_render::draw(screen, app, Layout::wide(), false, layer).unwrap();
            }
        }
        Mode::FrameOff => coast_measurement::run_off(screen, app),
    }
}

/// Exactness against the per-call candidate already measured on the S3:
/// coast-only output must equal `candidate_draw`, frames must equal the shared
/// candidate frame. Untimed; `reference` is overwritten.
pub fn verify(screen: &Screen, reference: &mut Screen, app: &RadarApp, mode: Mode) {
    reference.clear_for_coast();
    match mode {
        Mode::CoastRaster | Mode::CoastCached => {
            measurement::candidate_draw(reference, app.observer(), app.view(), ASSET).unwrap();
        }
        Mode::FrameRaster | Mode::FrameCached => {
            assert!(measurement::frame_candidate(
                reference, app, ASSET, false, false
            ));
        }
        Mode::Prepare | Mode::FrameOff => return,
    }
    assert_eq!(
        screen.pixels, reference.pixels,
        "{mode:?} differs from candidate"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coast_measurement::{CASES, app};

    #[test]
    fn prepared_modes_repeat_and_match_candidate() {
        for (name, lat, lon, fov) in CASES {
            let app = app(name, lat, lon, fov);
            let mut layer = None;
            let mut screen = Screen::new();
            let mut reference = Screen::new();
            for mode in MODES {
                let mut previous = None;
                for _ in 0..2 {
                    screen.clear_for_coast();
                    run(&mut screen, &app, &mut layer, mode);
                    verify(&screen, &mut reference, &app, mode);
                    let now = (screen.pixels, screen.writes, screen.on_writes);
                    if let Some(previous) = previous {
                        assert_eq!(previous, now, "{name}/{mode:?} not repeatable");
                    }
                    previous = Some(now);
                }
            }
        }
    }
}
