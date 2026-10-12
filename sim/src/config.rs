use overhead_app::projection::View;
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
  --radius-km N    Projected map radius, 50..20000 km (default 8000)
  --radar-size N   Radar allocation, 180..240 pixels (default 240)
  --snapshot PNG  Render exactly the start UTC to a 400x240 PNG; no window
  --help, -h      Show this help

Controls: wheel/up = zoom in; down = zoom out; Tab = next marker; Esc = quit.
Ground positions use a north-up spherical orthographic map. Map radius is not
surface or slant distance. Only true elevation > 0 is eligible, then map/view
clipping applies. Readouts use true physical state; ERR counts failed objects.

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
        let mut radius = None;
        let mut size = None;
        let mut snapshot = None;
        let mut options = options.iter();
        while let Some(flag) = options.next() {
            let value = options
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag.as_str() {
                "--radius-km" if radius.is_none() => radius = Some(number(value, "radius")?),
                "--radar-size" if size.is_none() => {
                    size = Some(value.parse::<u32>().map_err(|_| "invalid radar size")?)
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
            view: View::new(radius.unwrap_or(8000.0))?,
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
        let demo = parse("--demo --radius-km 500 --radar-size 200").unwrap();
        assert_eq!(demo.view.radius_km(), 500.0);
        let config = parse("x.json 2026-10-10T00:00:00.123Z -90 180 -1 --snapshot x.png").unwrap();
        assert_eq!(config.observer.latitude_rad.to_degrees(), -90.0);
        assert_eq!(config.snapshot, Some(PathBuf::from("x.png")));
    }
    #[test]
    fn reject_invalid_and_repeated_inputs() {
        for args in [
            "",
            "--demo --radius-km 0",
            "--demo --radius-km NaN",
            "--demo --radar-size 999",
            "--demo --radar-size 200 --radar-size 240",
            "--demo --radius-km",
            "--demo --wat 1",
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
