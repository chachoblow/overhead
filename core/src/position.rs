//! Small frame labels, not a generic vector or units framework.

/// TEME position in kilometres, interpreted at a separate explicit timestamp.
///
/// Construction labels the frame; it does not validate the components. The
/// conversion APIs reject non-finite inputs. There is no implicit conversion
/// to ECEF: use a propagated state's `to_ecef()` or [`crate::teme_to_ecef`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TemePosition([f64; 3]);

impl TemePosition {
    /// Labels raw kilometre components as TEME (without numeric validation).
    pub const fn from_km(km: [f64; 3]) -> Self {
        Self(km)
    }

    /// Returns the TEME components in kilometres.
    pub const fn km(self) -> [f64; 3] {
        self.0
    }
}

/// Earth-fixed position in kilometres under the GMST-only model (0007).
///
/// Construction labels the frame without numeric validation; geometry APIs
/// validate inputs. TEME and ECEF cannot be passed interchangeably.
///
/// ```compile_fail
/// use overhead_core::{TemePosition, ecef_to_geodetic};
/// let teme = TemePosition::from_km([7000.0, 0.0, 0.0]);
/// ecef_to_geodetic(teme); // Requires EcefPosition, not TemePosition.
/// ```
///
/// ```compile_fail
/// use overhead_core::{TemePosition, GeodeticPosition, ecef_to_look_angles};
/// let observer = GeodeticPosition {
///     latitude_rad: 0.0, longitude_rad: 0.0, altitude_km: 0.0,
/// };
/// ecef_to_look_angles(TemePosition::from_km([7000.0, 0.0, 0.0]), observer);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EcefPosition([f64; 3]);

impl EcefPosition {
    /// Labels raw kilometre components as ECEF (without numeric validation).
    pub const fn from_km(km: [f64; 3]) -> Self {
        Self(km)
    }

    /// Returns the ECEF components in kilometres.
    pub const fn km(self) -> [f64; 3] {
        self.0
    }
}
