//! Propagation of a validated satellite to an explicit UTC timestamp.
//!
//! Output is the raw SGP4 state in the TEME frame (True Equator, Mean
//! Equinox), interpreted at the propagation time, not as a frame frozen at
//! the element epoch. It does not rotate with the Earth. Earth-relative
//! positions are a separate transform step; see `crate::teme_to_ecef`.
//! See decisions 0006 and 0007 for conventions and the frame clarification.

use crate::{
    CoordinateError, EcefPosition, Satellite, TemePosition, TimeError, teme_to_ecef,
    validate_utc_time,
};
use sgp4::chrono::NaiveDateTime;

/// Read-only propagated TEME state bound to its absolute UTC timestamp.
///
/// Obtained from [`Satellite::state_at`]; use [`Self::to_ecef`] for rotation
/// without supplying a second, potentially inconsistent timestamp.
///
/// ```compile_fail
/// use overhead_core::{TemeState, sgp4::chrono::NaiveDateTime};
/// fn retime(state: &mut TemeState, wrong_time: NaiveDateTime) {
///     state.datetime = wrong_time; // Propagated state cannot be retimed.
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TemeState {
    position: TemePosition,
    velocity: [f64; 3],
    minutes_since_epoch: f64,
    datetime: NaiveDateTime,
}

impl TemeState {
    /// TEME position in km at [`Self::datetime`].
    pub fn position(&self) -> TemePosition {
        self.position
    }

    /// TEME velocity in km/s; not an Earth-fixed velocity.
    pub fn velocity(&self) -> [f64; 3] {
        self.velocity
    }

    /// Signed elapsed minutes from the element epoch (no leap seconds inserted).
    pub fn minutes_since_epoch(&self) -> f64 {
        self.minutes_since_epoch
    }

    /// Absolute propagation timestamp, interpreted as UTC.
    pub fn datetime(&self) -> NaiveDateTime {
        self.datetime
    }

    /// Rotates the position using this state's own propagation timestamp.
    /// Does not convert velocity.
    pub fn to_ecef(&self) -> Result<EcefPosition, CoordinateError> {
        teme_to_ecef(self.position, self.datetime)
    }
}

/// Why propagation to a timestamp failed.
#[derive(Debug, Clone, PartialEq)]
pub enum PropagateError {
    /// Requested timestamp violates the engine UTC contract.
    UnsupportedTime(TimeError),
    /// The interval between the element epoch and the requested time
    /// overflows datetime arithmetic.
    TimeOutOfRange,
    /// The orbital elements diverged during propagation (for example a
    /// decayed orbit or runaway perturbed eccentricity).
    Sgp4(sgp4::Error),
}

impl core::fmt::Display for PropagateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedTime(error) => error.fmt(f),
            Self::TimeOutOfRange => {
                write!(f, "requested time is out of range of the element epoch")
            }
            Self::Sgp4(error) => write!(f, "propagation diverged: {error}"),
        }
    }
}

impl Satellite {
    /// Computes the TEME state at an explicit UTC timestamp.
    ///
    /// Validates the requested time with [`validate_utc_time`]. Leap seconds
    /// are not modelled: elapsed time is the naive UTC timestamp difference.
    /// Supported years do not imply accurate propagation far from the epoch;
    /// no freshness policy is applied, and SGP4 divergence remains an error.
    pub fn state_at(&self, datetime: NaiveDateTime) -> Result<TemeState, PropagateError> {
        validate_utc_time(datetime).map_err(PropagateError::UnsupportedTime)?;
        let minutes_since_epoch = (datetime - self.epoch())
            .num_nanoseconds()
            .ok_or(PropagateError::TimeOutOfRange)? as f64
            / 60e9;
        let prediction = self
            .constants()
            .propagate_afspc_compatibility_mode(sgp4::MinutesSinceEpoch(minutes_since_epoch))
            .map_err(PropagateError::Sgp4)?;
        Ok(TemeState {
            position: TemePosition::from_km(prediction.position),
            velocity: prediction.velocity,
            minutes_since_epoch,
            datetime,
        })
    }
}
