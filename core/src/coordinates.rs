//! Position-only frame and WGS-84 ellipsoid conversions.
//!
//! TEME → PEF uses IAU-1982 GMST at the position's timestamp, with UT1≈UTC.
//! PEF is treated as ECEF (no polar motion). These are the GMST-only
//! simplifications in decision 0006, not a full ITRF reduction.
//! The rotation must NOT be used directly on velocity: that also requires
//! the Earth's angular-velocity cross product.

use core::f64::consts::{FRAC_PI_2, PI, TAU};
use sgp4::chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike};

const WGS84_A: f64 = 6378.137; // equatorial radius, km
const WGS84_F: f64 = 1.0 / 298.257_223_563;
const WGS84_E2: f64 = WGS84_F * (2.0 - WGS84_F);
const WGS84_B: f64 = WGS84_A * (1.0 - WGS84_F);

/// WGS-84 ellipsoidal coordinates (not geocentric latitude or MSL height).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeodeticPosition {
    /// Latitude in radians, in [-π/2, π/2].
    pub latitude_rad: f64,
    /// East-positive longitude in radians. Inverse conversion returns [-π, π).
    /// Longitude is conventionally zero on the exact polar axis.
    pub longitude_rad: f64,
    /// Height in km above the WGS-84 ellipsoid; may be negative.
    pub altitude_km: f64,
}

/// Invalid input or failed coordinate conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinateError {
    /// An input or computed result is NaN/infinite (including overflow).
    NonFinite,
    /// Latitude is outside [-π/2, π/2].
    InvalidLatitude,
    /// Longitude is outside [-π, π].
    InvalidLongitude,
    /// No unique latitude/longitude exists at the Earth's center.
    EarthCenter,
    /// The inverse iteration did not converge within its fixed work limit.
    NoConvergence,
    /// Outside 1957..=2100, or an explicit leap-second representation.
    UnsupportedTime,
}

impl core::fmt::Display for CoordinateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::NonFinite => "coordinate input or result is not finite",
            Self::InvalidLatitude => "latitude is outside [-pi/2, pi/2]",
            Self::InvalidLongitude => "longitude is outside [-pi, pi]",
            Self::EarthCenter => "geodetic position is undefined at Earth's center",
            Self::NoConvergence => "geodetic conversion did not converge",
            Self::UnsupportedTime => "time must be in 1957..=2100 and not a leap second",
        })
    }
}

fn finite(values: [f64; 3]) -> Result<[f64; 3], CoordinateError> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(values)
    } else {
        Err(CoordinateError::NonFinite)
    }
}

/// Converts a TEME **position** (km) to ECEF (km) at an explicit UTC time.
///
/// Pass the same timestamp used to propagate this position, not the orbital
/// element epoch. UTC years 1957..=2100 are supported, matching ingestion's
/// era. Explicit leap seconds are rejected; no leap-second/EOP tables are
/// required. UT1≈UTC introduces up to ~0.4 km of surface rotation error.
///
/// Uses the position matrix in Vallado's `teme2ecef` with GMST only and no
/// polar motion: `[cos(θ)x + sin(θ)y, -sin(θ)x + cos(θ)y, z]`.
pub fn teme_to_ecef(
    position_km: [f64; 3],
    datetime: NaiveDateTime,
) -> Result<[f64; 3], CoordinateError> {
    let [x, y, z] = finite(position_km)?;
    if !(1957..=2100).contains(&datetime.year()) || datetime.nanosecond() >= 1_000_000_000 {
        return Err(CoordinateError::UnsupportedTime);
    }
    let j2000 = NaiveDate::from_ymd_opt(2000, 1, 1)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    // Chrono handles Gregorian century boundaries correctly (notably 2100),
    // unlike sgp4 2.4's simplified calendar-to-Julian-years helper.
    let elapsed = datetime - j2000;
    let seconds = elapsed.num_seconds() as f64 + elapsed.subsec_nanos() as f64 * 1e-9;
    let years = seconds / (365.25 * 86400.0);
    let theta = sgp4::iau_epoch_to_sidereal_time(years);
    let (sin, cos) = libm::sincos(theta);
    finite([cos * x + sin * y, -sin * x + cos * y, z])
}

/// Converts ECEF position in km to WGS-84 geodetic coordinates.
///
/// Iterative ellipsoid-normal latitude (Vallado Algorithm 13), bounded to
/// 16 iterations with a 1e-13 radian convergence threshold. Intended for
/// terrestrial and satellite positions, including shallow negative heights;
/// deep-interior normal-coordinate ambiguity is not resolved. The exact
/// geocenter is rejected. On the exact polar axis longitude is set to zero.
///
/// Height uses the projection onto the ellipsoid normal, avoiding division
/// by sin(latitude) or cos(latitude) at the equator and poles.
pub fn ecef_to_geodetic(position_km: [f64; 3]) -> Result<GeodeticPosition, CoordinateError> {
    let [x, y, z] = finite(position_km)?;
    let p = libm::hypot(x, y);
    let radius = libm::hypot(p, z);
    if !radius.is_finite() {
        return Err(CoordinateError::NonFinite);
    }
    if radius == 0.0 {
        return Err(CoordinateError::EarthCenter);
    }
    if p == 0.0 {
        return Ok(GeodeticPosition {
            latitude_rad: if z > 0.0 { FRAC_PI_2 } else { -FRAC_PI_2 },
            longitude_rad: 0.0,
            altitude_km: z.abs() - WGS84_B,
        });
    }
    let longitude = libm::atan2(y, x);
    let longitude = if longitude >= PI {
        longitude - TAU
    } else {
        longitude
    };
    let mut latitude = libm::atan2(z, p * (1.0 - WGS84_E2));
    for _ in 0..16 {
        let sin = libm::sin(latitude);
        let n = WGS84_A / libm::sqrt(1.0 - WGS84_E2 * sin * sin);
        let next = libm::atan2(z + WGS84_E2 * n * sin, p);
        let converged = (next - latitude).abs() < 1e-13;
        latitude = next;
        if converged {
            let (sin, cos) = libm::sincos(latitude);
            let height = p * cos + z * sin - WGS84_A * libm::sqrt(1.0 - WGS84_E2 * sin * sin);
            finite([latitude, longitude, height])?;
            return Ok(GeodeticPosition {
                latitude_rad: latitude,
                longitude_rad: longitude,
                altitude_km: height,
            });
        }
    }
    Err(CoordinateError::NoConvergence)
}

impl GeodeticPosition {
    /// Converts WGS-84 geodetic coordinates to ECEF position in km.
    ///
    /// Accepts longitude in [-π, π], latitude in [-π/2, π/2], and finite
    /// height (negative heights allowed). Exact poles produce x=y=0.
    pub fn to_ecef(self) -> Result<[f64; 3], CoordinateError> {
        let [latitude, longitude, height] =
            finite([self.latitude_rad, self.longitude_rad, self.altitude_km])?;
        if !(-FRAC_PI_2..=FRAC_PI_2).contains(&latitude) {
            return Err(CoordinateError::InvalidLatitude);
        }
        if !(-PI..=PI).contains(&longitude) {
            return Err(CoordinateError::InvalidLongitude);
        }
        let (sin_lat, mut cos_lat) = libm::sincos(latitude);
        if latitude.abs() == FRAC_PI_2 {
            cos_lat = 0.0;
        }
        let (sin_lon, cos_lon) = libm::sincos(longitude);
        let n = WGS84_A / libm::sqrt(1.0 - WGS84_E2 * sin_lat * sin_lat);
        finite([
            (n + height) * cos_lat * cos_lon,
            (n + height) * cos_lat * sin_lon,
            (n * (1.0 - WGS84_E2) + height) * sin_lat,
        ])
    }
}
