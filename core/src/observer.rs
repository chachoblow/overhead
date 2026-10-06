//! Geometric observer-relative measurements; no refraction or visibility policy.

use core::f64::consts::{FRAC_PI_2, TAU};

use crate::{CoordinateError, EcefPosition, GeodeticPosition};

/// Direct line-of-sight measurements from an explicit WGS-84 observer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LookAngles {
    /// Slant range in km, strictly positive.
    pub range_km: f64,
    /// Clockwise from true north, in [0, 2π). Undefined at zenith/nadir:
    /// `None` when horizontal range / slant range <= 1e-12.
    pub azimuth_rad: Option<f64>,
    /// Radians above the local tangent horizon, in [-π/2, π/2].
    /// Negative elevations are retained, not clipped.
    pub elevation_rad: f64,
}

/// Invalid observer/target geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationError {
    /// Invalid observer coordinates, non-finite target, or numeric overflow.
    Coordinate(CoordinateError),
    /// Observer and target have exactly the same ECEF position; no direction.
    CoincidentPositions,
}

impl From<CoordinateError> for ObservationError {
    fn from(error: CoordinateError) -> Self {
        Self::Coordinate(error)
    }
}

impl core::fmt::Display for ObservationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Coordinate(error) => error.fmt(f),
            Self::CoincidentPositions => f.write_str("observer and target positions coincide"),
        }
    }
}

/// Converts an ECEF target position (km) to geometric range/azimuth/elevation.
///
/// The observer's latitude/longitude are geodetic radians and altitude is km
/// above WGS-84, validated by [`GeodeticPosition::to_ecef`]. At exact poles,
/// the supplied longitude defines the limiting local north/east axes; it is
/// not discarded, even though all longitudes give the same observer position.
///
/// Subtracts the observer ECEF position and rotates into south/east/zenith
/// (SEZ). Azimuth is atan2(east, -south), elevation atan2(zenith, horizontal).
/// A relative 1e-12 horizontal tolerance suppresses numerical azimuth noise
/// on the vertical axis; it does not clip or alter elevation.
///
/// For a propagated TEME position, first call [`crate::TemeState::to_ecef`]
/// to rotate it using its bound propagation timestamp. This function has
/// no clock, propagation, atmospheric refraction, terrain, or occultation
/// model; even targets below the horizon return geometric measurements.
pub fn ecef_to_look_angles(
    target_km: EcefPosition,
    observer: GeodeticPosition,
) -> Result<LookAngles, ObservationError> {
    let site = observer.to_ecef()?.km();
    let target_km = target_km.km();
    if !target_km.iter().all(|value| value.is_finite()) {
        return Err(CoordinateError::NonFinite.into());
    }
    let delta: [f64; 3] = core::array::from_fn(|i| target_km[i] - site[i]);
    let range = libm::hypot(libm::hypot(delta[0], delta[1]), delta[2]);
    if !range.is_finite() {
        return Err(CoordinateError::NonFinite.into());
    }
    if range == 0.0 {
        return Err(ObservationError::CoincidentPositions);
    }
    // Normalize before rotation to avoid overflowing intermediate SEZ sums.
    let [x, y, z] = delta.map(|value| value / range);
    let (sin_lat, mut cos_lat) = libm::sincos(observer.latitude_rad);
    if observer.latitude_rad.abs() == FRAC_PI_2 {
        cos_lat = 0.0;
    }
    let (sin_lon, cos_lon) = libm::sincos(observer.longitude_rad);
    let radial = cos_lon * x + sin_lon * y;
    let south = sin_lat * radial - cos_lat * z;
    let east = -sin_lon * x + cos_lon * y;
    let zenith = cos_lat * radial + sin_lat * z;
    let horizontal = libm::hypot(south, east);
    let azimuth_rad = if horizontal <= 1e-12 {
        None
    } else {
        let angle = libm::atan2(east, -south);
        let angle = if angle < 0.0 { angle + TAU } else { angle };
        // Adding a tiny negative angle to TAU can round to TAU. Canonicalize
        // that boundary (and signed zero) so the public interval stays strict.
        Some(if angle >= TAU || angle == 0.0 {
            0.0
        } else {
            angle
        })
    };
    Ok(LookAngles {
        range_km: range,
        azimuth_rad,
        elevation_rad: libm::atan2(zenith, horizontal),
    })
}
