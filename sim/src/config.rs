use overhead_app::{
    identification::{DEFAULT_LABEL_HIDE_DEG, DEFAULT_LABEL_SHOW_DEG, LabelZoom},
    projection::{DEFAULT_FOV_DEG, View, Viewport},
};
use overhead_core::{
    GeodeticPosition,
    sgp4::chrono::{NaiveDateTime, TimeDelta},
    validate_utc_time,
};
use overhead_render::Layout;
use std::{path::PathBuf, time::Duration};

pub const USAGE: &str = "Usage: overhead-sim MANIFEST.json UTC LAT_DEG LON_DEG HEIGHT_KM [OPTIONS]
       overhead-sim --demo [OPTIONS]

Local OMM manifest uses the overhead-catalogue format; paths are relative to it.
UTC: YYYY-MM-DDTHH:MM:SS[.fraction]Z, 1957-2100, no leap seconds.
Observer: WGS-84 latitude [-90,90], east longitude [-180,180], ellipsoidal km.
Replay advances from explicit UTC at 1x using monotonic elapsed time, not now.
No network, freshness cutoff, pass prediction, or production scheduling.

Options (each at most once):
  --fov-deg N     Full vertical field of view, 0.1..100 degrees (default 20)
  --radar-size N   Square radar allocation, 180..240 pixels (default 240)
  --viewport MODE  square (default, borderless), circle, borderless-circle
  --wide          352x240 comparison with off-screen arrows; no --radar-size/--viewport
  --debug         Start wide replay in text-first debug; requires --wide, no --snapshot
  --label-hide-deg N  Hide wide-view ID at/above this vertical FOV (default 70)
  --label-show-deg N  Restore ID at/below this FOV (default 60); requires --wide
  --snapshot PNG  Render exactly the start UTC to a 400x240 PNG; no window
  --help, -h      Show this help

Controls: wheel/up = zoom in; down = zoom out; Tab = next marker; Esc = quit.
Wide interactive replay only: v = toggle radar/debug; zoom and Tab stay active.
Actual 3D positions use a north-up perspective camera fixed 75000 km above the
observer along local up. Zoom changes the lens, not camera position. Earth is a
WGS-84 outline through the same camera. Only true elevation > 0 is eligible;
near-plane (1 km) and viewport clipping also apply. Readouts use true state;
The default panel's ERR counts failed objects; --wide shows TRACK ERROR and
INGEST WARN notices. One arrow per eligible off-screen satellite; overlaps are
allowed, never merged or suppressed. No arrows for depth < 1 km. Wide horizontal
FOV follows drawable aspect, preserving pixel scale and Earth's proportions.
Both label options require --wide and 0.1 <= show < hide <= 100. Between thresholds,
keep the previous visibility; at launch show below hide. Only the ID/ISS text is
hidden, never the selected diamond. No fade or dithering.
The former --radius-km option is no longer supported.

--demo uses the pinned 16-object historical M2 pool at 2026-10-10T00:00:00Z,
observer 39.007 -104.883 2.187, from this build's checkout. Not live data or a
production catalogue/capacity choice. See tools/fixtures/m2-pool/README.md.

Examples (workspace root):
  cargo run -p overhead-sim -- --demo
  cargo run -p overhead-sim -- --demo --snapshot /tmp/overhead.png
  cargo run -p overhead-sim -- sim/examples/weather-gps.json \\
    2026-10-10T00:00:00Z 39.007 -104.883 2.187 --radar-size 200";

pub struct Config {
    pub manifest: PathBuf,
    pub start: NaiveDateTime,
    pub observer: GeodeticPosition,
    pub view: View,
    pub layout: Layout,
    pub label_zoom: LabelZoom,
    pub snapshot: Option<PathBuf>,
    pub debug: bool,
}

impl Config {
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let (manifest, utc, lat, lon, height, options) =
            if args.first().is_some_and(|a| a == "--demo") {
                (
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/catalogue.json"),
                    "2026-10-10T00:00:00Z",
                    "39.007",
                    "-104.883",
                    "2.187",
                    &args[1..],
                )
            } else {
                if args.len() < 5 {
                    return Err(format!(
                        "expected a manifest, UTC and observer, or --demo\n\n{USAGE}"
                    ));
                }
                (
                    PathBuf::from(&args[0]),
                    args[1].as_str(),
                    args[2].as_str(),
                    args[3].as_str(),
                    args[4].as_str(),
                    &args[5..],
                )
            };
        let utc = utc.strip_suffix('Z').ok_or("UTC must end in Z")?;
        let start = NaiveDateTime::parse_from_str(utc, "%Y-%m-%dT%H:%M:%S%.f")
            .map_err(|e| format!("invalid UTC: {e}"))?;
        validate_utc_time(start).map_err(|e| e.to_string())?;
        let latitude = number(lat, "latitude")?;
        let longitude = number(lon, "longitude")?;
        if !(-90.0..=90.0).contains(&latitude) || !(-180.0..=180.0).contains(&longitude) {
            return Err("latitude/longitude outside [-90,90]/[-180,180] degrees".into());
        }
        let observer = GeodeticPosition {
            latitude_rad: latitude.to_radians(),
            longitude_rad: longitude.to_radians(),
            altitude_km: number(height, "height")?,
        };
        observer.to_ecef().map_err(|e| e.to_string())?;
        let mut fov = None;
        let mut size = None;
        let mut viewport = None;
        let mut snapshot = None;
        let mut label_hide = None;
        let mut label_show = None;
        let mut wide = false;
        let mut debug = false;
        let mut options = options.iter();
        while let Some(flag) = options.next() {
            if flag == "--wide" || flag == "--debug" {
                let enabled = if flag == "--wide" {
                    &mut wide
                } else {
                    &mut debug
                };
                if *enabled {
                    return Err(format!("repeated option: {flag}"));
                }
                *enabled = true;
                continue;
            }
            let value = options
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag.as_str() {
                "--fov-deg" if fov.is_none() => fov = Some(number(value, "field of view")?),
                "--label-hide-deg" if label_hide.is_none() => {
                    label_hide = Some(number(value, "label hide threshold")?);
                }
                "--label-show-deg" if label_show.is_none() => {
                    label_show = Some(number(value, "label show threshold")?);
                }
                "--radius-km" => {
                    return Err(
                        "--radius-km was removed; use --fov-deg (lens angle, not map distance)"
                            .into(),
                    );
                }
                "--radar-size" if size.is_none() => {
                    size = Some(value.parse::<u32>().map_err(|_| "invalid radar size")?)
                }
                "--viewport" if viewport.is_none() => {
                    viewport = Some(match value.as_str() {
                        "circle" => Viewport::Circle,
                        "borderless-circle" => Viewport::BorderlessCircle,
                        "square" => Viewport::Square,
                        _ => {
                            return Err(
                                "viewport must be circle, borderless-circle or square".into()
                            );
                        }
                    });
                }
                "--snapshot" if snapshot.is_none() && !value.is_empty() => {
                    snapshot = Some(PathBuf::from(value))
                }
                _ => return Err(format!("unknown, repeated or invalid option: {flag}")),
            }
        }
        if debug && !wide {
            return Err("--debug requires --wide".into());
        }
        if debug && snapshot.is_some() {
            return Err("--debug cannot be combined with --snapshot".into());
        }
        if wide && (size.is_some() || viewport.is_some()) {
            return Err("--wide cannot be combined with --radar-size or --viewport".into());
        }
        if !wide && (label_hide.is_some() || label_show.is_some()) {
            return Err("--label-hide-deg and --label-show-deg require --wide".into());
        }
        let label_zoom = LabelZoom::new(
            label_show.unwrap_or(DEFAULT_LABEL_SHOW_DEG),
            label_hide.unwrap_or(DEFAULT_LABEL_HIDE_DEG),
        )?;
        let layout = if wide {
            Layout::wide()
        } else {
            Layout::new(size.unwrap_or(240))?
        };
        let fov = fov.unwrap_or(DEFAULT_FOV_DEG);
        let view = if wide {
            View::rectangular(fov, layout.aspect_ratio())?
        } else {
            View::new(fov)?.with_viewport(viewport.unwrap_or_default())
        };
        Ok(Self {
            manifest,
            start,
            observer,
            view,
            layout,
            label_zoom,
            snapshot,
            debug,
        })
    }
}

fn number(s: &str, label: &str) -> Result<f64, String> {
    let n: f64 = s.parse().map_err(|_| format!("invalid {label}: {s}"))?;
    if !n.is_finite() {
        return Err(format!("{label} must be finite"));
    }
    Ok(n)
}

/// No wall clock and no accumulated per-frame rounding/drift.
pub fn replay_time(start: NaiveDateTime, elapsed: Duration) -> Result<NaiveDateTime, String> {
    let delta = TimeDelta::from_std(elapsed).map_err(|_| "replay duration overflow")?;
    let time = start
        .checked_add_signed(delta)
        .ok_or("replay UTC overflow")?;
    validate_utc_time(time).map_err(|e| e.to_string())?;
    Ok(time)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(text: &str) -> Result<Config, String> {
        Config::parse(
            &text
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        )
    }
    #[test]
    fn explicit_configuration_and_demo() {
        let demo = parse("--demo --fov-deg 60 --radar-size 200").unwrap();
        assert!((demo.view.fov_deg() - 60.0).abs() < 1e-12);
        assert!((parse("--demo").unwrap().view.fov_deg() - DEFAULT_FOV_DEG).abs() < 1e-12);
        let config = parse("x.json 2026-10-10T00:00:00.123Z -90 180 -1 --snapshot x.png").unwrap();
        assert_eq!(config.observer.latitude_rad.to_degrees(), -90.0);
        assert_eq!(config.snapshot, Some(PathBuf::from("x.png")));
    }
    #[test]
    fn comparison_viewports_are_explicit_and_default_is_square() {
        assert_eq!(parse("--demo").unwrap().view.viewport(), Viewport::Square);
        for (name, expected) in [
            ("circle", Viewport::Circle),
            ("borderless-circle", Viewport::BorderlessCircle),
            ("square", Viewport::Square),
        ] {
            let c = parse(&format!("--demo --viewport {name}")).unwrap();
            assert_eq!(c.view.viewport(), expected);
            assert!((c.view.fov_deg() - 20.0).abs() < 1e-12);
        }
    }
    #[test]
    fn wide_comparison_preserves_vertical_fov_and_has_explicit_aspect() {
        for args in ["--demo --wide", "--demo --fov-deg 20 --wide"] {
            let config = parse(args).unwrap();
            assert_eq!(config.view.viewport(), Viewport::Rectangle);
            assert!((config.view.fov_deg() - 20.0).abs() < 1e-12);
            assert_eq!(config.layout.aspect_ratio(), 173.0 / 117.0);
            assert_eq!(config.view, View::rectangular(20.0, 173.0 / 117.0).unwrap());
        }
        let config = parse("--demo --wide --snapshot wide.png --fov-deg 100").unwrap();
        assert!((config.view.fov_deg() - 100.0).abs() < 1e-12);
        assert_eq!(config.snapshot, Some(PathBuf::from("wide.png")));
    }

    #[test]
    fn debug_start_is_explicit_wide_only_interactive_and_order_independent() {
        assert!(!parse("--demo").unwrap().debug);
        assert!(!parse("--demo --wide").unwrap().debug);
        assert!(!parse("--demo --wide --snapshot normal.png").unwrap().debug);
        for args in [
            "--demo --wide --debug",
            "--demo --debug --fov-deg 60 --wide",
            "x.json 2026-10-10T00:00:00Z 0 0 0 --debug --wide",
        ] {
            let config = parse(args).unwrap();
            assert!(config.debug);
            assert_eq!(config.view.viewport(), Viewport::Rectangle);
            assert_eq!(
                config.view,
                View::rectangular(config.view.fov_deg(), config.layout.aspect_ratio()).unwrap()
            );
            assert!(config.snapshot.is_none());
        }
        for args in [
            "--demo --debug",
            "--demo --viewport square --debug",
            "--demo --viewport circle --debug",
            "--demo --wide --debug --debug",
            "--demo --debug --wide --debug",
            "--demo --wide --debug --snapshot debug.png",
            "--demo --snapshot debug.png --debug --wide",
            "--demo --wide --debug --radar-size 240",
            "--demo --wide --debug --viewport square",
        ] {
            assert!(parse(args).is_err(), "{args}");
        }
    }

    #[test]
    fn label_thresholds_are_configurable_and_wide_only() {
        assert_eq!(
            parse("--demo --wide").unwrap().label_zoom,
            LabelZoom::default()
        );
        for (args, show, hide) in [
            (
                "--demo --wide --label-show-deg 50 --label-hide-deg 80",
                50.0,
                80.0,
            ),
            ("--demo --label-hide-deg 80 --wide", 60.0, 80.0),
            ("--demo --wide --label-show-deg 50", 50.0, 70.0),
        ] {
            assert_eq!(
                parse(args).unwrap().label_zoom,
                LabelZoom::new(show, hide).unwrap()
            );
        }
        for args in [
            "--demo --label-hide-deg 80",
            "--demo --label-show-deg 50",
            "--demo --wide --label-hide-deg",
            "--demo --wide --label-show-deg",
            "--demo --wide --label-hide-deg NaN",
            "--demo --wide --label-show-deg inf",
            "--demo --wide --label-show-deg 0",
            "--demo --wide --label-hide-deg 100.1",
            "--demo --wide --label-hide-deg 60",
            "--demo --wide --label-show-deg 80",
            "--demo --wide --label-show-deg 50 --label-show-deg 55",
            "--demo --wide --label-hide-deg 80 --label-hide-deg 90",
        ] {
            assert!(parse(args).is_err(), "{args}");
        }
    }

    #[test]
    fn reject_invalid_and_repeated_inputs() {
        for args in [
            "",
            "--demo --fov-deg 0",
            "--demo --fov-deg 0.099",
            "--demo --fov-deg 100.01",
            "--demo --fov-deg NaN",
            "--demo --fov-deg inf",
            "--demo --fov-deg 20 --fov-deg 60",
            "--demo --radius-km 8000",
            "--demo --radar-size 999",
            "--demo --radar-size 200 --radar-size 240",
            "--demo --fov-deg",
            "--demo --wat 1",
            "--demo --viewport",
            "--demo --viewport triangle",
            "--demo --viewport square --viewport circle",
            "--demo --snapshot a --snapshot b",
            "--demo --wide --wide",
            "--demo --wide --viewport square",
            "--demo --radar-size 240 --wide",
            "--demo --wide 352",
            "--demo --wide --fov-deg",
            "x 2026-10-10T00:00:00 0 0 0",
            "x 2101-01-01T00:00:00Z 0 0 0",
            "x 2026-10-10T00:00:60Z 0 0 0",
            "x 2026-10-10T00:00:00Z 91 0 0",
            "x 2026-10-10T00:00:00Z 0 181 0",
            "x 2026-10-10T00:00:00Z 0 0 inf",
        ] {
            assert!(parse(args).is_err(), "{args}");
        }
    }
    #[test]
    fn replay_is_exact_and_stops_at_time_boundary() {
        let c = parse("--demo").unwrap();
        assert_eq!(replay_time(c.start, Duration::ZERO).unwrap(), c.start);
        assert_eq!(
            replay_time(c.start, Duration::from_millis(1234)).unwrap() - c.start,
            TimeDelta::milliseconds(1234)
        );
        let end = parse("x 2100-12-31T23:59:59Z 0 0 0").unwrap();
        assert!(replay_time(end.start, Duration::from_secs(1)).is_err());
    }
}
