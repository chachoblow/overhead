use overhead_app::projection::{DEFAULT_FOV_DEG, View, Viewport};
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
  --fov-deg N     Full width/height field of view, 0.1..100 degrees (default 20)
  --radar-size N   Radar allocation, 180..240 pixels (default 240)
  --viewport MODE  square (default, borderless), circle, borderless-circle
  --snapshot PNG  Render exactly the start UTC to a 400x240 PNG; no window
  --help, -h      Show this help

Controls: wheel/up = zoom in; down = zoom out; Tab = next marker; Esc = quit.
Actual 3D positions use a north-up perspective camera fixed 75000 km above the
observer along local up. Zoom changes the lens, not camera position. Earth is a
WGS-84 outline through the same camera. Only true elevation > 0 is eligible;
near-plane (1 km) and viewport clipping also apply. Readouts use true state;
ERR counts failed objects. The former --radius-km option is no longer supported.

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
    pub snapshot: Option<PathBuf>,
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
        let mut options = options.iter();
        while let Some(flag) = options.next() {
            let value = options
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag.as_str() {
                "--fov-deg" if fov.is_none() => fov = Some(number(value, "field of view")?),
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
        Ok(Self {
            manifest,
            start,
            observer,
            view: View::new(fov.unwrap_or(DEFAULT_FOV_DEG))?
                .with_viewport(viewport.unwrap_or_default()),
            layout: Layout::new(size.unwrap_or(240))?,
            snapshot,
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
