//! Platform-agnostic satellite tracking engine: element ingestion, orbit
//! propagation, coordinate transforms, and application state.
//!
//! `no_std`; no allocation unless the `omm` feature (OMM/JSON parsing) is
//! enabled. Conventions (time, frames, units, tolerances) are documented in
//! docs/decisions/0006-sgp4-crate-and-conventions.md.
#![no_std]

pub use sgp4;

mod ingest;
mod propagate;

pub use ingest::{IngestError, Satellite};
pub use propagate::{PropagateError, TemeState};
