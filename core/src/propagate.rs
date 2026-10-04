//! Propagation of a validated satellite to an explicit UTC timestamp.
//!
//! Output is the raw SGP4 state in the TEME frame (True Equator, Mean
//! Equinox of epoch) — an Earth-centered inertial frame that does not rotate
//! with the Earth. Earth-relative coordinates are a separate transform step.
//! See docs/decisions/0006-sgp4-crate-and-conventions.md.

use crate::Satellite;

/// Satellite position and velocity in the TEME frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TemeState {
    /// Position in km.
    pub position: [f64; 3],
    /// Velocity in km/s.
    pub velocity: [f64; 3],
    /// Minutes elapsed between the element epoch and the requested time
    /// (negative when the requested time precedes the epoch).
    pub minutes_since_epoch: f64,
}

/// Why propagation to a timestamp failed.
#[derive(Debug, Clone, PartialEq)]
pub enum PropagateError {
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
    /// Leap seconds are not modelled: the elapsed time is the naive
    /// difference between the UTC timestamps, matching the sgp4 crate's own
    /// epoch handling.
    pub fn state_at(
        &self,
        datetime: sgp4::chrono::NaiveDateTime,
    ) -> Result<TemeState, PropagateError> {
        let minutes_since_epoch = (datetime - self.epoch())
            .num_nanoseconds()
            .ok_or(PropagateError::TimeOutOfRange)? as f64
            / 60e9;
        let prediction = self
            .constants()
            .propagate_afspc_compatibility_mode(sgp4::MinutesSinceEpoch(minutes_since_epoch))
            .map_err(PropagateError::Sgp4)?;
        Ok(TemeState {
            position: prediction.position,
            velocity: prediction.velocity,
            minutes_since_epoch,
        })
    }
}
