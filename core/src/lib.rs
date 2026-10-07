//! Platform-agnostic satellite tracking engine: element ingestion, orbit
//! propagation, coordinate transforms, and application state.
//!
//! `no_std`; no allocation unless `omm` (OMM/JSON parsing) or `catalogue`
//! (checked catalogue assembly, implies `omm`) is enabled. Conventions (time,
//! frames, units, tolerances) are documented in
//! docs/decisions/0006-sgp4-crate-and-conventions.md.
#![no_std]

#[cfg(feature = "omm")]
extern crate alloc;

pub use sgp4;

#[cfg(feature = "catalogue")]
pub mod catalogue;
mod coordinates;
mod ingest;
mod observer;
#[cfg(feature = "omm")]
mod omm;
mod position;
mod propagate;
mod time;

pub use coordinates::{CoordinateError, GeodeticPosition, ecef_to_geodetic, teme_to_ecef};
pub use ingest::{IngestError, Satellite};
pub use observer::{LookAngles, ObservationError, ecef_to_look_angles};
#[cfg(feature = "omm")]
pub use omm::OmmElements;
pub use position::{EcefPosition, TemePosition};
pub use propagate::{PropagateError, TemeState};
pub use time::{TimeError, validate_utc_time};
