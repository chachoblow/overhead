//! Reproducible, host-only entry point for the M1 calculation pipeline.
use std::{env, fmt::Write, fs, process::ExitCode};

use overhead_core::{
    GeodeticPosition, LookAngles, Satellite, ecef_to_geodetic, ecef_to_look_angles,
    sgp4::{
        Elements,
        chrono::{Datelike, NaiveDateTime, Timelike},
    },
    teme_to_ecef,
};

const USAGE: &str = "Usage: overhead-track OMM.json UTC LAT_DEG LON_DEG HEIGHT_KM

OMM.json must contain a JSON array with exactly one satellite.
UTC: YYYY-MM-DDTHH:MM:SS[.fraction]Z (1957–2100; no leap seconds).
Observer: WGS-84 latitude [-90,90], east longitude [-180,180] in degrees,
          ellipsoidal height in km (not mean sea level; may be negative).
All inputs are explicit; no network, local-time conversion, or wall clock.

Example (from workspace root):
  cargo run -p overhead-tools --bin overhead-track -- \\
    core/tests/fixtures/iss-25544.json 2026-10-04T12:43:41.833056Z \\
    39.007 -104.883 2.187";

struct Inputs {
    path: String,
    time: NaiveDateTime,
    observer: GeodeticPosition,
}

impl Inputs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let [path, utc, lat, lon, height] = args else {
            return Err(format!("expected five arguments\n\n{USAGE}"));
        };
        let utc = utc.strip_suffix('Z').ok_or("UTC must end in Z")?;
        let time = NaiveDateTime::parse_from_str(utc, "%Y-%m-%dT%H:%M:%S%.f")
            .map_err(|error| format!("invalid UTC timestamp: {error}"))?;
        if !(1957..=2100).contains(&time.year()) || time.nanosecond() >= 1_000_000_000 {
            return Err("UTC must be in 1957..=2100 and not a leap second".into());
        }
        let number = |text: &str, label: &str| -> Result<f64, String> {
            let value: f64 = text
                .parse()
                .map_err(|_| format!("invalid {label}: {text}"))?;
            if !value.is_finite() {
                return Err(format!("{label} must be finite"));
            }
            Ok(value)
        };
        let latitude = number(lat, "latitude (deg)")?;
        let longitude = number(lon, "longitude (deg)")?;
        if !(-90.0..=90.0).contains(&latitude) {
            return Err("latitude must be in [-90, 90] deg".into());
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err("longitude must be in [-180, 180] deg".into());
        }
        let observer = GeodeticPosition {
            latitude_rad: latitude.to_radians(),
            longitude_rad: longitude.to_radians(),
            altitude_km: number(height, "height (km)")?,
        };
        observer
            .to_ecef()
            .map_err(|error| format!("invalid observer: {error}"))?;
        Ok(Self {
            path: path.clone(),
            time,
            observer,
        })
    }
}

fn report_look_angles(output: &mut String, angles: LookAngles) {
    writeln!(output, "Slant range (km): {:.9}", angles.range_km).unwrap();
    match angles.azimuth_rad {
        Some(azimuth) => writeln!(
            output,
            "Azimuth (deg clockwise from true north): {:.9}",
            azimuth.to_degrees()
        )
        .unwrap(),
        None => writeln!(
            output,
            "Azimuth (deg clockwise from true north): undefined (zenith/nadir)"
        )
        .unwrap(),
    }
    writeln!(
        output,
        "Elevation (deg): {:.9}",
        angles.elevation_rad.to_degrees()
    )
    .unwrap();
}

fn run(inputs: Inputs) -> Result<String, String> {
    let data = fs::read_to_string(&inputs.path)
        .map_err(|error| format!("cannot read {}: {error}", inputs.path))?;
    let elements: Vec<Elements> = serde_json::from_str(&data)
        .map_err(|error| format!("invalid OMM JSON in {}: {error}", inputs.path))?;
    let [elements] = elements.as_slice() else {
        return Err(format!(
            "expected exactly one satellite in OMM array; found {}",
            elements.len()
        ));
    };
    let satellite =
        Satellite::from_elements(elements).map_err(|error| format!("invalid elements: {error}"))?;
    let state = satellite
        .state_at(inputs.time)
        .map_err(|error| format!("propagation failed: {error}"))?;
    let ecef = teme_to_ecef(state.position, inputs.time)
        .map_err(|error| format!("TEME→ECEF failed: {error}"))?;
    let geodetic =
        ecef_to_geodetic(ecef).map_err(|error| format!("geodetic conversion failed: {error}"))?;
    let angles = ecef_to_look_angles(ecef, inputs.observer)
        .map_err(|error| format!("observer measurements failed: {error}"))?;

    // Buffer the complete report so failures never leave a partial success on stdout.
    let mut output = String::new();
    writeln!(
        output,
        "Satellite: {}",
        elements.object_name.as_deref().unwrap_or("(unnamed)")
    )
    .unwrap();
    writeln!(output, "NORAD ID: {}", satellite.norad_id()).unwrap();
    writeln!(
        output,
        "Element epoch (UTC): {}T{}Z",
        satellite.epoch().date(),
        satellite.epoch().time()
    )
    .unwrap();
    writeln!(
        output,
        "Requested time (UTC): {}T{}Z",
        inputs.time.date(),
        inputs.time.time()
    )
    .unwrap();
    writeln!(
        output,
        "Minutes since element epoch (min): {:.9}",
        state.minutes_since_epoch
    )
    .unwrap();
    writeln!(
        output,
        "Observer WGS-84 (lat deg, lon deg, ellipsoid km): {:.9} {:.9} {:.9}",
        inputs.observer.latitude_rad.to_degrees(),
        inputs.observer.longitude_rad.to_degrees(),
        inputs.observer.altitude_km
    )
    .unwrap();
    for (label, vector) in [
        ("TEME position (km)", state.position),
        ("TEME velocity (km/s)", state.velocity),
        ("ECEF position (km; GMST-only)", ecef),
    ] {
        writeln!(
            output,
            "{label}: {:.9} {:.9} {:.9}",
            vector[0], vector[1], vector[2]
        )
        .unwrap();
    }
    writeln!(
        output,
        "Satellite WGS-84 latitude (deg): {:.9}",
        geodetic.latitude_rad.to_degrees()
    )
    .unwrap();
    writeln!(
        output,
        "Satellite WGS-84 longitude (deg east): {:.9}",
        geodetic.longitude_rad.to_degrees()
    )
    .unwrap();
    writeln!(
        output,
        "Satellite WGS-84 ellipsoidal altitude (km): {:.9}",
        geodetic.altitude_km
    )
    .unwrap();
    report_look_angles(&mut output, angles);
    writeln!(
        output,
        "Model: AFSPC SGP4; WGS-84 geometry; UT1≈UTC; no polar motion/refraction."
    )
    .unwrap();
    writeln!(output, "Negative elevation is valid geometry, not a visibility decision; printed precision is not orbit accuracy.").unwrap();
    Ok(output)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() == 1 && matches!(args[0].as_str(), "--help" | "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    match Inputs::parse(&args).and_then(run) {
        Ok(report) => {
            print!("{report}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undefined_azimuth_is_not_reported_as_north() {
        let mut output = String::new();
        report_look_angles(
            &mut output,
            LookAngles {
                range_km: 400.0,
                azimuth_rad: None,
                elevation_rad: std::f64::consts::FRAC_PI_2,
            },
        );
        assert!(output.contains("undefined (zenith/nadir)"));
        assert!(output.contains("Elevation (deg): 90.000000000"));
    }
}
