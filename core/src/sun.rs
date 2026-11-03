//! Low-precision Sun direction and a cylindrical Earth-shadow test, for
//! naked-eye visibility classification (decision 0078). Not an ephemeris for
//! timing or pointing: about 0.01° in direction between 1950 and 2050.

use crate::{CoordinateError, EcefPosition, TemePosition, teme_to_ecef};
use sgp4::chrono::{NaiveDate, NaiveDateTime};

/// Astronomical unit, km (IAU 2012).
const AU_KM: f64 = 149_597_870.7;
/// WGS-84 equatorial radius, km: the shadow cylinder's radius.
const EARTH_RADIUS_KM: f64 = 6378.137;

/// Sun position in ECEF km at a UTC instant.
///
/// The Astronomical Almanac's low-precision formula (mean longitude, mean
/// anomaly, equation of centre and mean obliquity of date) gives the
/// equatorial direction and distance. That frame is treated as TEME and
/// rotated with the same GMST as satellites ([`teme_to_ecef`]); the frame
/// difference (nutation, arcseconds) is far below the formula's 0.01°.
/// UTC is used for TT (about a minute, ~0.0008° of solar motion).
pub fn sun_ecef(datetime: NaiveDateTime) -> Result<EcefPosition, CoordinateError> {
    teme_to_ecef(TemePosition::from_km(sun_equatorial_km(datetime)), datetime)
}

/// Mean-equator-of-date Sun vector, km. Separate for testing against
/// published right ascension/declination.
fn sun_equatorial_km(datetime: NaiveDateTime) -> [f64; 3] {
    let j2000 = NaiveDate::from_ymd_opt(2000, 1, 1)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let elapsed = datetime - j2000;
    let n = (elapsed.num_seconds() as f64 + elapsed.subsec_nanos() as f64 * 1e-9) / 86_400.0;
    let mean_longitude = (280.460 + 0.985_647_4 * n).to_radians();
    let g = (357.528 + 0.985_600_3 * n).to_radians();
    let longitude = mean_longitude
        + (1.915_f64.to_radians()) * libm::sin(g)
        + (0.020_f64.to_radians()) * libm::sin(2.0 * g);
    let obliquity = (23.439 - 0.000_000_4 * n).to_radians();
    let distance_km = (1.000_14 - 0.016_71 * libm::cos(g) - 0.000_14 * libm::cos(2.0 * g)) * AU_KM;
    let (sin_l, cos_l) = libm::sincos(longitude);
    let (sin_e, cos_e) = libm::sincos(obliquity);
    [
        distance_km * cos_l,
        distance_km * cos_e * sin_l,
        distance_km * sin_e * sin_l,
    ]
}

/// True when `satellite` lies inside Earth's shadow cylinder: behind Earth
/// as seen from the Sun and within one equatorial radius of the Earth–Sun
/// axis. No penumbra, flattening or atmospheric refraction; at the cylinder
/// edge the answer can differ from reality by a few seconds of a pass.
pub fn in_earth_shadow(satellite: EcefPosition, sun: EcefPosition) -> bool {
    let r = satellite.km();
    let s = sun.km();
    let norm = libm::sqrt(s[0] * s[0] + s[1] * s[1] + s[2] * s[2]);
    if !(norm.is_finite() && norm > 0.0) {
        return false;
    }
    let u = s.map(|v| v / norm);
    let along = r[0] * u[0] + r[1] * u[1] + r[2] * u[2];
    if along >= 0.0 {
        return false;
    }
    let off: [f64; 3] = core::array::from_fn(|i| r[i] - along * u[i]);
    off[0] * off[0] + off[1] * off[1] + off[2] * off[2] < EARTH_RADIUS_KM * EARTH_RADIUS_KM
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GeodeticPosition, ecef_to_geodetic, ecef_to_look_angles};

    fn utc(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }

    fn subsolar(datetime: NaiveDateTime) -> (f64, f64) {
        let g = ecef_to_geodetic(sun_ecef(datetime).unwrap()).unwrap();
        (g.latitude_rad.to_degrees(), g.longitude_rad.to_degrees())
    }

    #[test]
    fn matches_meeus_example_25a_right_ascension_and_declination() {
        // Meeus, Astronomical Algorithms, Example 25.a: 1992-10-13 0h TD,
        // apparent RA 13h13m31.4s = 198.38083°, declination −7°47'06" = −7.78507°.
        let [x, y, z] = sun_equatorial_km(utc(1992, 10, 13, 0, 0));
        let ra = libm::atan2(y, x).to_degrees().rem_euclid(360.0);
        let dec = libm::atan2(z, libm::hypot(x, y)).to_degrees();
        assert!((ra - 198.38083).abs() < 0.01, "ra {ra}");
        assert!((dec - -7.78507).abs() < 0.01, "dec {dec}");
        let au = libm::sqrt(x * x + y * y + z * z) / AU_KM;
        // Meeus: R = 0.99766 AU.
        assert!((au - 0.99766).abs() < 0.0002, "distance {au}");
    }

    #[test]
    fn subsolar_point_follows_season_and_equation_of_time() {
        // 2024 March equinox 03:06 UTC: subsolar latitude ~0.
        let (lat, _) = subsolar(utc(2024, 3, 20, 3, 6));
        assert!(lat.abs() < 0.02, "equinox latitude {lat}");
        // 2024 June solstice 20:51 UTC: latitude = obliquity.
        let (lat, _) = subsolar(utc(2024, 6, 20, 20, 51));
        assert!((lat - 23.44).abs() < 0.02, "solstice latitude {lat}");
        // Early November the Sun runs ~16.4 min fast: at 12:00 UTC it is
        // ~4.1° west of Greenwich.
        let (lat, lon) = subsolar(utc(2026, 11, 3, 12, 0));
        assert!((lon - -4.1).abs() < 0.1, "november longitude {lon}");
        assert!((lat - -15.2).abs() < 0.3, "november declination {lat}");
    }

    #[test]
    fn local_noon_and_midnight_elevations() {
        // Equator at the March equinox subsolar longitude: Sun near zenith;
        // the antipode sees it near nadir.
        let time = utc(2024, 3, 20, 3, 6);
        let (_, lon) = subsolar(time);
        let sun = sun_ecef(time).unwrap();
        let site = |lon_deg: f64| GeodeticPosition {
            latitude_rad: 0.0,
            longitude_rad: lon_deg.to_radians(),
            altitude_km: 0.0,
        };
        let noon = ecef_to_look_angles(sun, site(lon)).unwrap();
        assert!(noon.elevation_rad.to_degrees() > 89.9);
        let antipode = (lon + 360.0).rem_euclid(360.0) - 180.0;
        let midnight = ecef_to_look_angles(sun, site(antipode)).unwrap();
        assert!(midnight.elevation_rad.to_degrees() < -89.9);
    }

    #[test]
    fn shadow_cylinder_geometry() {
        let sun = EcefPosition::from_km([AU_KM, 0.0, 0.0]);
        let at = |x: f64, y: f64, z: f64| in_earth_shadow(EcefPosition::from_km([x, y, z]), sun);
        // Behind Earth, on the axis and just inside the radius: shadow.
        assert!(at(-7000.0, 0.0, 0.0));
        assert!(at(-7000.0, 6378.0, 0.0));
        assert!(at(-42_164.0, 0.0, 6000.0));
        // Just outside the radius, on the day side, or beside Earth: lit.
        assert!(!at(-7000.0, 6379.0, 0.0));
        assert!(!at(7000.0, 0.0, 0.0));
        assert!(!at(0.0, 7000.0, 0.0));
        // Degenerate Sun vector never claims shadow.
        assert!(!in_earth_shadow(
            EcefPosition::from_km([-7000.0, 0.0, 0.0]),
            EcefPosition::from_km([0.0, 0.0, 0.0])
        ));
    }
}
