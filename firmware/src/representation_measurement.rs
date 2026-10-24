//! Measurement-only candidate adapter. Same site/app/RAM target as Stage A.
//! Nothing here is selected by the historical kernel or coast measurement images.
use crate::coast_measurement::{Mode, Screen};
use overhead_app::RadarApp;
use overhead_render::measurement;

pub const ASSET: &[u8; 32_211] = include_bytes!("../fixtures/coast-quadratic-trial.bin");

pub fn run(screen: &mut Screen, app: &RadarApp, mode: Mode) -> bool {
    match mode {
        Mode::Coast => {
            measurement::candidate_draw(screen, app.observer(), app.view(), ASSET).is_ok()
        }
        Mode::FrameOff => {
            crate::coast_measurement::run_off(screen, app);
            true
        }
        Mode::FrameOn => measurement::frame_candidate(screen, app, ASSET, false, false),
        Mode::Late | Mode::LateWarning => {
            measurement::frame_candidate(screen, app, ASSET, mode == Mode::LateWarning, true)
        }
    }
}

pub fn verify_late(screen: &Screen, reference: &mut Screen, app: &RadarApp, warning: bool) {
    measurement::candidate_failure_control(reference, app, warning).unwrap();
    assert_eq!(
        screen.pixels, reference.pixels,
        "candidate rollback lost foreground/notices"
    );
}

pub fn modes(name: &str, on: bool) -> &'static [Mode] {
    if !on {
        &[Mode::FrameOff]
    } else if name == "polar-20" || name == "iss-ground-20" {
        &[
            Mode::Coast,
            Mode::FrameOn,
            Mode::FrameOff,
            Mode::Late,
            Mode::LateWarning,
        ]
    } else {
        &[Mode::Coast, Mode::FrameOn, Mode::FrameOff]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coast_measurement::{CASES, app};

    #[test]
    fn candidate_repeats_and_preserves_rollback_controls() {
        for (name, lat, lon, fov) in CASES {
            let app = app(name, lat, lon, fov);
            let mut screen = Screen::new();
            for &mode in modes(name, true) {
                screen.clear_for_coast();
                assert!(run(&mut screen, &app, mode), "{name}/{mode:?}");
                let pixels = screen.pixels;
                let writes = (screen.writes, screen.on_writes);
                screen.clear_for_coast();
                assert!(run(&mut screen, &app, mode));
                assert_eq!(pixels, screen.pixels);
                assert_eq!(writes, (screen.writes, screen.on_writes));
                assert!(screen.on_writes > 0);
                if matches!(mode, Mode::Late | Mode::LateWarning) {
                    verify_late(&screen, &mut Screen::new(), &app, mode == Mode::LateWarning);
                }
            }
            let mut malformed = Screen::new();
            assert!(!measurement::frame_candidate(
                &mut malformed,
                &app,
                &[],
                false,
                false
            ));
            let mut expected = Screen::new();
            measurement::candidate_failure_control(&mut expected, &app, false).unwrap();
            assert_eq!(malformed.pixels, expected.pixels);
        }
    }
}
