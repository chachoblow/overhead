mod config;

use config::{Config, USAGE, replay_time};
use embedded_graphics::{pixelcolor::BinaryColor, prelude::DrawTarget};
use embedded_graphics_simulator::{
    BinaryColorTheme, OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
    sdl2::{Keycode, MouseWheelDirection},
};
use overhead_app::{RadarApp, projection::Viewport};
use overhead_render::{Layout, presentation::Presentation};
use std::{
    env,
    process::ExitCode,
    thread,
    time::{Duration, Instant},
};

fn run(config: Config) -> Result<(), String> {
    let loaded = overhead_tools::catalogue::load(&config.manifest)?;
    eprint!("{}{}", loaded.report, loaded.diagnostics);
    let ingestion_warning = !loaded.diagnostics.is_empty();
    let mut app = RadarApp::new(loaded.catalogue, config.observer, config.view)
        .map_err(|e| e.to_string())?
        .with_group_categories(loaded.group_categories)
        .with_label_zoom(config.label_zoom);
    app.update(config.start).map_err(|e| e.to_string())?;
    let mut previous_failures = Vec::new();
    report_failures(&app, &mut previous_failures);
    eprintln!(
        "REPLAY start={}Z observer={:.6},{:.6},{:.3}km; local data only; no freshness guarantee",
        config.start,
        config.observer.latitude_rad.to_degrees(),
        config.observer.longitude_rad.to_degrees(),
        config.observer.altitude_km
    );
    let mut display = SimulatorDisplay::<BinaryColor>::new(overhead_render::DISPLAY_SIZE);
    let mut presentation = InteractivePresentation::new(config.view.viewport(), config.debug);
    if config.snapshot.is_some() {
        overhead_render::draw(&mut display, &app, config.layout, ingestion_warning)
            .map_err(|e| e.to_string())?;
    } else {
        // Normal startup primes the glyph timeline before input, so even a Tab
        // on the first cycle has an endpoint. Debug starts without glyph history.
        presentation
            .draw(
                &mut display,
                &app,
                config.layout,
                ingestion_warning,
                Duration::ZERO,
            )
            .map_err(|e| e.to_string())?;
    }
    if let Some(path) = config.snapshot {
        display
            .to_rgb_output_image(
                &OutputSettingsBuilder::new()
                    .scale(1)
                    .pixel_spacing(0)
                    .theme(BinaryColorTheme::LcdWhite)
                    .build(),
            )
            .save_png(&path)
            .map_err(|e| format!("cannot save {}: {e}", path.display()))?;
        let counts = app.counts();
        println!(
            "UTC={}Z catalogue={} sky={} visible={} failed={} indicators={} snapshot={}",
            config.start,
            counts.catalogue,
            counts.above_horizon,
            counts.visible,
            counts.failed,
            if app.view().viewport() == overhead_app::projection::Viewport::Rectangle {
                app.indicators().count()
            } else {
                0
            },
            path.display()
        );
        return Ok(());
    }
    let settings = OutputSettingsBuilder::new()
        .theme(BinaryColorTheme::LcdWhite)
        .scale(2)
        .build();
    let mut window = Window::new("Overhead - local replay", &settings);
    let started = Instant::now();
    // This is a host visual-loop target, not a production device cadence or
    // bounded scheduler. Missed frames are not queued or caught up in a burst.
    let frame_period = Duration::from_millis(50);
    loop {
        let frame_started = Instant::now();
        // First frame is exactly the supplied start, including snapshot checks.
        window.update(&display);
        for event in window.events() {
            match event {
                SimulatorEvent::Quit => return Ok(()),
                SimulatorEvent::KeyDown {
                    keycode, repeat, ..
                } => {
                    if !presentation.key(&mut app, keycode, repeat) {
                        return Ok(());
                    }
                }
                SimulatorEvent::MouseWheel {
                    scroll_delta,
                    direction,
                } => app.zoom(wheel_steps(scroll_delta.y, direction)),
                _ => {}
            }
        }
        if let Some(remaining) = frame_period.checked_sub(frame_started.elapsed()) {
            thread::sleep(remaining);
        }
        let elapsed = started.elapsed();
        app.update(replay_time(config.start, elapsed)?)
            .map_err(|e| e.to_string())?;
        report_failures(&app, &mut previous_failures);
        presentation
            .draw(
                &mut display,
                &app,
                config.layout,
                ingestion_warning,
                elapsed,
            )
            .map_err(|e| e.to_string())?;
    }
}

/// Host-only presentation choice. Toggling never mutates app geometry or truth;
/// discard hidden glyph history so returning normal starts at current endpoints.
struct InteractivePresentation {
    wide: bool,
    debug: bool,
    glyphs: Presentation,
}

impl InteractivePresentation {
    fn new(viewport: Viewport, debug: bool) -> Self {
        let wide = viewport == Viewport::Rectangle;
        Self {
            wide,
            debug: wide && debug,
            glyphs: Presentation::new(),
        }
    }

    /// False requests exit. Repeat policy preserves existing Tab/zoom behavior;
    /// holding V must not cycle between pages on keyboard auto-repeat.
    fn key(&mut self, app: &mut RadarApp, key: Keycode, repeat: bool) -> bool {
        match key {
            Keycode::Escape => return false,
            Keycode::Up => app.zoom(1),
            Keycode::Down => app.zoom(-1),
            Keycode::Tab if !repeat => app.select_next(),
            Keycode::V if self.wide && !repeat => {
                self.debug = !self.debug;
                self.glyphs = Presentation::new();
            }
            _ => {}
        }
        true
    }

    fn draw<D: DrawTarget<Color = BinaryColor>>(
        &mut self,
        display: &mut D,
        app: &RadarApp,
        layout: Layout,
        ingestion_warning: bool,
        elapsed: Duration,
    ) -> Result<(), D::Error> {
        if self.debug {
            overhead_render::debug::draw(display, app, ingestion_warning)
        } else {
            overhead_render::draw_at(
                display,
                app,
                layout,
                ingestion_warning,
                &mut self.glyphs,
                elapsed,
            )
        }
    }
}

fn wheel_steps(y: i32, direction: MouseWheelDirection) -> i32 {
    let steps = y.clamp(-64, 64);
    if direction == MouseWheelDirection::Flipped {
        -steps
    } else {
        steps
    }
}

fn report_failures(app: &RadarApp, previous: &mut Vec<(u64, String)>) {
    let current: Vec<_> = app
        .objects()
        .filter_map(|(entry, result)| {
            result
                .as_ref()
                .err()
                .map(|e| (entry.elements().norad_id, e.to_string()))
        })
        .collect();
    if current != *previous {
        if current.is_empty() {
            eprintln!("tracking recovered at {:?}", app.time());
        }
        for (id, error) in &current {
            eprintln!("tracking failed at {:?}, NORAD {id}: {error}", app.time());
        }
        *previous = current;
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() == 1 && matches!(args[0].as_str(), "--help" | "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    match Config::parse(&args).and_then(run) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
#[path = "interactive_tests.rs"]
mod tests;
