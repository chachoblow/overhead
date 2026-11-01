//! 0075 build-time observer site for the live-fetch radar (interim under 0004):
//! decimal degrees and metres from the ignored `firmware/.site.env`. The
//! coordinates are personal data; images and logs built from them stay local.
use overhead_core::GeodeticPosition;

/// Parses latitude/longitude in decimal degrees (north/east positive) and an
/// optional height in metres above the WGS-84 ellipsoid (default 0).
pub fn parse(
    lat_deg: &str,
    lon_deg: &str,
    alt_m: Option<&str>,
) -> Result<GeodeticPosition, &'static str> {
    let lat: f64 = lat_deg
        .trim()
        .parse()
        .map_err(|_| "SITE_LAT_DEG is not a number")?;
    let lon: f64 = lon_deg
        .trim()
        .parse()
        .map_err(|_| "SITE_LON_DEG is not a number")?;
    let alt: f64 = match alt_m {
        None => 0.0,
        Some(a) if a.trim().is_empty() => 0.0,
        Some(a) => a.trim().parse().map_err(|_| "SITE_ALT_M is not a number")?,
    };
    if !lat.is_finite() || !(-90.0..=90.0).contains(&lat) {
        return Err("SITE_LAT_DEG outside -90..=90");
    }
    if !lon.is_finite() || !(-180.0..=180.0).contains(&lon) {
        return Err("SITE_LON_DEG outside -180..=180");
    }
    if !alt.is_finite() || !(-500.0..=9_000.0).contains(&alt) {
        return Err("SITE_ALT_M outside -500..=9000");
    }
    Ok(GeodeticPosition {
        latitude_rad: lat.to_radians(),
        longitude_rad: lon.to_radians(),
        altitude_km: alt / 1000.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_degrees_and_metres() {
        let p = parse("37.5", "-122.25", Some("120")).unwrap();
        assert!((p.latitude_rad - 37.5f64.to_radians()).abs() < 1e-15);
        assert!((p.longitude_rad + 122.25f64.to_radians()).abs() < 1e-15);
        assert!((p.altitude_km - 0.12).abs() < 1e-15);
        assert_eq!(parse(" 0 ", "0", None).unwrap().altitude_km, 0.0);
        assert_eq!(parse("0", "0", Some("")).unwrap().altitude_km, 0.0);
    }

    #[test]
    fn refuses_bad_or_out_of_range_values() {
        for (lat, lon, alt) in [
            ("91", "0", None),
            ("0", "181", None),
            ("NaN", "0", None),
            ("0", "inf", None),
            ("", "0", None),
            ("37N", "0", None),
            ("0", "0", Some("10000")),
            ("0", "0", Some("x")),
        ] {
            assert!(parse(lat, lon, alt).is_err(), "{lat} {lon} {alt:?}");
        }
    }
}
