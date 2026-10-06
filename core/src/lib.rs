//! Platform-agnostic satellite tracking engine: element ingestion, orbit
//! propagation, coordinate transforms, and application state.
//!
//! `no_std`; no allocation unless the `omm` feature (OMM/JSON parsing) is
//! enabled. Conventions (time, frames, units, tolerances) are documented in
//! docs/decisions/0006-sgp4-crate-and-conventions.md.
#![no_std]

pub use sgp4;

mod coordinates;
mod ingest;
mod observer;
mod position;
mod propagate;
mod time;

pub use coordinates::{CoordinateError, GeodeticPosition, ecef_to_geodetic, teme_to_ecef};
pub use ingest::{IngestError, Satellite};
pub use observer::{LookAngles, ObservationError, ecef_to_look_angles};
pub use position::{EcefPosition, TemePosition};
pub use propagate::{PropagateError, TemeState};
pub use time::{TimeError, validate_utc_time};
