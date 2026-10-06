//! Validation of parsed orbital element sets into engine inputs.
//!
//! The `sgp4` crate handles the parsing (TLE always; OMM behind the `omm`
//! feature). This module owns the gate between "parsed data" and "data the
//! engine will propagate": structural sanity checks first, then SGP4
//! initialization. See docs/decisions/0006-sgp4-crate-and-conventions.md.

use crate::{TimeError, validate_utc_time};

/// Why an element set was rejected during ingestion.
#[derive(Debug, Clone, PartialEq)]
pub enum IngestError {
    /// A floating-point field is NaN or infinite.
    NonFinite,
    /// Mean motion (revolutions per day) must be positive.
    NonPositiveMeanMotion(f64),
    /// Eccentricity must be in [0, 1) for a closed orbit.
    EccentricityOutOfRange(f64),
    /// Inclination must be within [0°, 180°].
    InclinationOutOfRange(f64),
    /// Epoch year outside 1957..=2100 — almost certainly corrupt data.
    ImplausibleEpoch(i32),
    /// An explicit leap-second epoch is not supported.
    LeapSecondEpoch,
    /// SGP4 initialization rejected the elements.
    Sgp4(sgp4::ElementsError),
}

impl core::fmt::Display for IngestError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonFinite => write!(f, "element set contains a NaN or infinite value"),
            Self::NonPositiveMeanMotion(n) => write!(f, "mean motion {n} rev/day is not positive"),
            Self::EccentricityOutOfRange(e) => write!(f, "eccentricity {e} is outside [0, 1)"),
            Self::InclinationOutOfRange(i) => write!(f, "inclination {i}° is outside [0°, 180°]"),
            Self::ImplausibleEpoch(year) => write!(f, "epoch year {year} is implausible"),
            Self::LeapSecondEpoch => write!(f, "element epoch cannot be an explicit leap second"),
            Self::Sgp4(error) => write!(f, "SGP4 initialization failed: {error}"),
        }
    }
}

/// A validated satellite ready for propagation.
pub struct Satellite {
    norad_id: u64,
    epoch: sgp4::chrono::NaiveDateTime,
    constants: sgp4::Constants,
}

impl Satellite {
    /// Validates a parsed element set and initializes the SGP4 propagator.
    ///
    /// The epoch must satisfy [`validate_utc_time`]. This does not enforce
    /// freshness or change the upstream SGP4 epoch calendar approximation.
    pub fn from_elements(elements: &sgp4::Elements) -> Result<Self, IngestError> {
        let finite_fields = [
            elements.mean_motion,
            elements.eccentricity,
            elements.inclination,
            elements.right_ascension,
            elements.argument_of_perigee,
            elements.mean_anomaly,
            elements.drag_term,
            elements.mean_motion_dot,
            elements.mean_motion_ddot,
        ];
        if finite_fields.iter().any(|value| !value.is_finite()) {
            return Err(IngestError::NonFinite);
        }
        if elements.mean_motion <= 0.0 {
            return Err(IngestError::NonPositiveMeanMotion(elements.mean_motion));
        }
        if !(0.0..1.0).contains(&elements.eccentricity) {
            return Err(IngestError::EccentricityOutOfRange(elements.eccentricity));
        }
        if !(0.0..=180.0).contains(&elements.inclination) {
            return Err(IngestError::InclinationOutOfRange(elements.inclination));
        }
        validate_utc_time(elements.datetime).map_err(|error| match error {
            TimeError::UnsupportedYear(year) => IngestError::ImplausibleEpoch(year),
            TimeError::LeapSecond => IngestError::LeapSecondEpoch,
        })?;
        // AFSPC compatibility mode: the mode element sets are fitted in and
        // the mode the Vallado verification vectors were generated with.
        // See docs/decisions/0006-sgp4-crate-and-conventions.md.
        let constants = sgp4::Constants::from_elements_afspc_compatibility_mode(elements)
            .map_err(IngestError::Sgp4)?;
        Ok(Self {
            norad_id: elements.norad_id,
            epoch: elements.datetime,
            constants,
        })
    }

    /// The NORAD catalogue number identifying this object.
    pub fn norad_id(&self) -> u64 {
        self.norad_id
    }

    /// The UTC epoch of the underlying element set.
    pub fn epoch(&self) -> sgp4::chrono::NaiveDateTime {
        self.epoch
    }

    /// The initialized SGP4 propagator constants.
    pub fn constants(&self) -> &sgp4::Constants {
        &self.constants
    }
}
