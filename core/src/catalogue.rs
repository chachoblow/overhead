//! Checked, deterministic catalogue assembly (`no_std` + alloc).
//!
//! Callers own document parsing and the table mapping group indices to source
//! metadata. Supply only records deserialized through [`OmmElements`]. A failed
//! document must abort the caller's entire load; do not publish a partial build.
//! See docs/decisions/0011-catalogue-ingestion-and-provenance.md.

use alloc::{collections::BTreeMap, vec, vec::Vec};
use core::cmp::Ordering;

use crate::{IngestError, OmmElements, Satellite, sgp4};

/// Location in the caller's input table. Both indices are zero-based.
/// Each input record should have a unique origin within a build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RecordOrigin {
    pub group: usize,
    pub record: usize,
}

/// Diagnostics are data, not log messages. Parsing/document errors belong to
/// the caller; numeric/epoch validation and merge conflicts are reported here.
#[derive(Debug, Clone, PartialEq)]
pub enum CatalogueDiagnostic {
    InvalidRecord {
        origin: RecordOrigin,
        norad_id: u64,
        error: IngestError,
    },
    /// Disagreement among valid records at the newest valid epoch for this ID.
    /// Includes all records at that epoch, sorted by origin. No older fallback
    /// is selected during an initial build.
    EpochConflict {
        norad_id: u64,
        epoch: sgp4::chrono::NaiveDateTime,
        origins: Vec<RecordOrigin>,
    },
}

/// One accepted satellite and the provenance of its selected complete record.
pub struct CatalogueEntry {
    satellite: Satellite,
    elements: OmmElements,
    selected_from: RecordOrigin,
    groups: Vec<usize>,
}

impl CatalogueEntry {
    pub fn satellite(&self) -> &Satellite {
        &self.satellite
    }

    pub fn elements(&self) -> &sgp4::Elements {
        self.elements.elements()
    }

    /// For equivalent newest records, use the lowest (group, record) origin.
    /// This only breaks ties in non-orbital metadata, never orbital conflicts.
    pub fn selected_from(&self) -> RecordOrigin {
        self.selected_from
    }

    /// Sorted unique groups contributing valid records, including older ones.
    /// Invalid records do not establish membership.
    pub fn groups(&self) -> &[usize] {
        &self.groups
    }
}

/// A nonempty accepted catalogue. No clock, filesystem, network, or freshness
/// policy is implicit in construction. SGP4 initialization is not a guarantee
/// that propagation will succeed at every requested time.
pub struct Catalogue {
    entries: Vec<CatalogueEntry>,
    diagnostics: Vec<CatalogueDiagnostic>,
}

impl Catalogue {
    /// Sorted by NORAD ID, with at most one entry per ID.
    pub fn entries(&self) -> &[CatalogueEntry] {
        &self.entries
    }

    pub fn diagnostics(&self) -> &[CatalogueDiagnostic] {
        &self.diagnostics
    }
}

/// No usable satellite remained; diagnostics survive the failed build.
#[derive(Debug)]
pub struct EmptyCatalogue {
    pub diagnostics: Vec<CatalogueDiagnostic>,
}

impl core::fmt::Display for EmptyCatalogue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "no usable satellites remain in the catalogue")
    }
}

struct Candidate {
    entry: CatalogueEntry,
    newest_origins: Vec<RecordOrigin>,
    conflicted: bool,
}

/// Staging builder: finish before replacing any active catalogue. Allocated
/// capacity is not an embedded operating budget; target limits remain M2 work.
#[derive(Default)]
pub struct CatalogueBuilder {
    candidates: BTreeMap<u64, Candidate>,
    diagnostics: Vec<CatalogueDiagnostic>,
}

impl CatalogueBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate before selection, so even a newer invalid record cannot evict
    /// valid older elements. Records may arrive in any order.
    pub fn push(&mut self, origin: RecordOrigin, elements: OmmElements) {
        let raw = elements.elements();
        let satellite = match Satellite::from_elements(raw) {
            Ok(satellite) => satellite,
            Err(error) => {
                self.diagnostics.push(CatalogueDiagnostic::InvalidRecord {
                    origin,
                    norad_id: raw.norad_id,
                    error,
                });
                return;
            }
        };
        let candidate = match self.candidates.entry(raw.norad_id) {
            alloc::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert(Candidate {
                    entry: CatalogueEntry {
                        satellite,
                        elements,
                        selected_from: origin,
                        groups: vec![origin.group],
                    },
                    newest_origins: vec![origin],
                    conflicted: false,
                });
                return;
            }
            alloc::collections::btree_map::Entry::Occupied(slot) => slot.into_mut(),
        };
        let entry = &mut candidate.entry;
        if let Err(index) = entry.groups.binary_search(&origin.group) {
            entry.groups.insert(index, origin.group);
        }
        match satellite.epoch().cmp(&entry.satellite.epoch()) {
            Ordering::Less => {}
            Ordering::Greater => {
                entry.satellite = satellite;
                entry.elements = elements;
                entry.selected_from = origin;
                candidate.newest_origins.clear();
                candidate.newest_origins.push(origin);
                candidate.conflicted = false;
            }
            Ordering::Equal => {
                candidate.newest_origins.push(origin);
                candidate.conflicted |= !same_orbit(raw, entry.elements());
                if origin < entry.selected_from {
                    entry.satellite = satellite;
                    entry.elements = elements;
                    entry.selected_from = origin;
                }
            }
        }
    }

    pub fn finish(mut self) -> Result<Catalogue, EmptyCatalogue> {
        // Order invalid-record diagnostics by source rather than arrival.
        self.diagnostics.sort_by_key(|diagnostic| match diagnostic {
            CatalogueDiagnostic::InvalidRecord { origin, .. } => *origin,
            CatalogueDiagnostic::EpochConflict { .. } => unreachable!(),
        });
        let mut entries = Vec::new();
        for (norad_id, mut candidate) in self.candidates {
            if candidate.conflicted {
                candidate.newest_origins.sort_unstable();
                self.diagnostics.push(CatalogueDiagnostic::EpochConflict {
                    norad_id,
                    epoch: candidate.entry.satellite.epoch(),
                    origins: candidate.newest_origins,
                });
            } else {
                entries.push(candidate.entry);
            }
        }
        if entries.is_empty() {
            Err(EmptyCatalogue {
                diagnostics: self.diagnostics,
            })
        } else {
            Ok(Catalogue {
                entries,
                diagnostics: self.diagnostics,
            })
        }
    }
}

// Compare parsed numeric values exactly (not raw JSON bytes, not an accuracy
// tolerance). ID/epoch are already equal. Names, designators, classification,
// element/revolution counters, and unrelated metadata are not orbital values.
// Include all nine validated orbital floats and the ephemeris-type field.
fn same_orbit(a: &sgp4::Elements, b: &sgp4::Elements) -> bool {
    a.mean_motion == b.mean_motion
        && a.eccentricity == b.eccentricity
        && a.inclination == b.inclination
        && a.right_ascension == b.right_ascension
        && a.argument_of_perigee == b.argument_of_perigee
        && a.mean_anomaly == b.mean_anomaly
        && a.drag_term == b.drag_term
        && a.mean_motion_dot == b.mean_motion_dot
        && a.mean_motion_ddot == b.mean_motion_ddot
        && a.ephemeris_type == b.ephemeris_type
}
