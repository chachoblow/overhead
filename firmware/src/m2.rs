//! Frozen M2 pipeline and portable outcome checks. No production defaults.
//! Tables are generated offline, without changing pinned preparation sources.
use crate::heap_meter::Snapshot;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::{self, Write};
use overhead_core::{
    GeodeticPosition,
    catalogue::{Catalogue, CatalogueBuilder, CatalogueDiagnostic, RecordOrigin},
    catalogue_passes::{Arrival, CataloguePasses, search_catalogue},
    passes::{EvaluationBudget, SearchConfig},
    sgp4::chrono::{NaiveDateTime, TimeDelta},
};
use serde_json::value::RawValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lifetime {
    Flash,
    Dropped,
    Retained,
}

pub struct Group {
    pub name: &'static str,
    pub key: &'static str,
    pub sources: &'static str,
    pub text: &'static str,
}
pub struct Case {
    pub id: u8,
    pub start_utc: &'static str,
    pub site: [f64; 3],
    pub window_s: i64,
    pub allowance: u64,
    pub lifetime: Lifetime,
    pub groups: &'static [Group],
    pub expected: &'static str,
}
include!("m2_fixtures.rs");

impl Case {
    pub fn config(&self) -> SearchConfig {
        SearchConfig::new(
            NaiveDateTime::parse_from_str(self.start_utc, "%Y-%m-%dT%H:%M:%SZ").unwrap(),
            TimeDelta::seconds(self.window_s),
            10_f64.to_radians(),
            TimeDelta::seconds(60),
            TimeDelta::seconds(5),
        )
        .unwrap()
    }
    pub fn observer(&self) -> GeodeticPosition {
        GeodeticPosition {
            latitude_rad: self.site[0].to_radians(),
            longitude_rad: self.site[1].to_radians(),
            altitude_km: self.site[2],
        }
    }
    pub fn copy_inputs(&self) -> [Option<String>; 4] {
        let mut copies = [None, None, None, None];
        if self.lifetime != Lifetime::Flash {
            for (slot, group) in copies.iter_mut().zip(self.groups) {
                *slot = Some(group.text.into());
            }
        }
        copies
    }
    pub fn texts<'a>(&self, copies: &'a [Option<String>; 4]) -> [&'a str; 4] {
        let mut texts = [""; 4];
        for (i, group) in self.groups.iter().enumerate() {
            texts[i] = copies[i].as_deref().unwrap_or(group.text);
        }
        texts
    }
}

#[derive(Debug, PartialEq)]
pub struct ParseError {
    pub origin: RecordOrigin,
    pub message: String,
}
pub struct DocumentFailure {
    pub group: usize,
    pub message: String,
}
pub struct Load {
    pub catalogue: Result<Catalogue, DocumentFailure>,
    pub parse_errors: Vec<ParseError>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecordObservation {
    pub group: usize,
    pub record: usize,
    pub requested_live: usize,
    pub occupied_live: usize,
    pub calls: usize,
    pub requested: usize,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Observations {
    pub raw_capacities: [usize; 4],
    pub records: [RecordObservation; 32],
    pub count: usize,
}

/// Validate each complete array before checked OMM records; retain owned error
/// messages. The snapshot callback must not allocate and is not a heap ledger.
#[inline(never)]
pub fn load(
    texts: &[&str],
    observed: &mut Observations,
    mut snapshot: impl FnMut() -> Snapshot,
) -> Load {
    assert!(texts.len() <= 4);
    let mut builder = CatalogueBuilder::new();
    let mut parse_errors = Vec::new();
    for (group, text) in texts.iter().enumerate() {
        let records: Vec<&RawValue> = match serde_json::from_str(text) {
            Ok(records) => records,
            Err(error) => {
                return Load {
                    catalogue: Err(DocumentFailure {
                        group,
                        message: error.to_string(),
                    }),
                    parse_errors,
                };
            }
        };
        observed.raw_capacities[group] = records.capacity();
        for (record, raw) in records.into_iter().enumerate() {
            let before = snapshot();
            let origin = RecordOrigin { group, record };
            match serde_json::from_str(raw.get()) {
                Ok(elements) => builder.push(origin, elements),
                Err(error) => parse_errors.push(ParseError {
                    origin,
                    message: error.to_string(),
                }),
            }
            let after = snapshot();
            observed.records[observed.count] = RecordObservation {
                group,
                record,
                requested_live: after.requested_live,
                occupied_live: after.occupied_live,
                calls: after.calls - before.calls,
                requested: after.requested - before.requested,
            };
            observed.count += 1;
        }
    }
    // Empty catalogue is never a permitted outcome in the frozen suite.
    Load {
        catalogue: Ok(builder.finish().expect("unexpected empty catalogue")),
        parse_errors,
    }
}

#[inline(never)]
pub fn aggregate(
    case: &Case,
    config: &SearchConfig,
    load: &Load,
) -> Option<(CataloguePasses, Vec<Arrival>)> {
    load.catalogue.as_ref().ok().map(|catalogue| {
        let result = search_catalogue(
            catalogue,
            case.observer(),
            config,
            200_000,
            &mut EvaluationBudget::new(case.allowance),
        );
        let candidates = result.earliest_candidates();
        (result, candidates)
    })
}

/// Canonical outcomes emitted from retained objects, never copied expectations.
/// Formatting/validation belongs outside measured phase timers.
pub fn outcomes(
    w: &mut impl Write,
    load: &Load,
    result: &Option<(CataloguePasses, Vec<Arrival>)>,
) -> fmt::Result {
    if let Ok(c) = &load.catalogue {
        for entry in c.entries() {
            let o = entry.selected_from();
            writeln!(
                w,
                "ACCEPT,{},{:?}Z,{:?},{},{}",
                entry.elements().norad_id,
                entry.elements().datetime,
                entry.groups(),
                o.group,
                o.record
            )?;
        }
    }
    for p in &load.parse_errors {
        writeln!(
            w,
            "PARSE,{},{},{}",
            p.origin.group, p.origin.record, p.message
        )?;
    }
    if let Ok(c) = &load.catalogue {
        for d in c.diagnostics() {
            match d {
                CatalogueDiagnostic::InvalidRecord {
                    norad_id,
                    origin,
                    error,
                } => writeln!(
                    w,
                    "INVALID,{norad_id},{},{},{error:?}",
                    origin.group, origin.record
                )?,
                CatalogueDiagnostic::EpochConflict {
                    norad_id,
                    epoch,
                    origins,
                } => {
                    writeln!(w, "CONFLICT,{norad_id},{epoch:?}Z")?;
                    for o in origins {
                        writeln!(w, "ORIGIN,{},{}", o.group, o.record)?;
                    }
                }
            }
        }
        writeln!(w, "FAIL,none")?;
    } else if let Err(f) = &load.catalogue {
        writeln!(w, "FAIL,{},{}", f.group, f.message)?;
    }
    if let Some((r, candidates)) = result {
        writeln!(
            w,
            "WORK,{},{},{},{}",
            u8::from(r.is_complete()),
            r.evaluations(),
            r.satellites()
                .iter()
                .map(|s| s.passes().len())
                .sum::<usize>(),
            candidates.len()
        )?;
        for s in r.satellites() {
            writeln!(
                w,
                "SAT,{},{},{},{},{:?}",
                s.norad_id(),
                s.report().evaluations,
                s.passes().len(),
                s.report().upcoming_passes,
                s.report().status
            )?;
        }
    }
    Ok(())
}

struct Compare<'a>(&'a str);
impl Write for Compare<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0 = self.0.strip_prefix(s).ok_or(fmt::Error)?;
        Ok(())
    }
}
pub fn verify(case: &Case, load: &Load, result: &Option<(CataloguePasses, Vec<Arrival>)>) {
    let mut expected = Compare(case.expected);
    outcomes(&mut expected, load, result).expect("outcomes differ from frozen manifest");
    assert!(expected.0.is_empty(), "missing frozen outcomes");
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot() -> Snapshot {
        Snapshot {
            requested_live: 0,
            occupied_live: 0,
            calls: 0,
            requested: 0,
            failures: 0,
        }
    }
    #[test]
    fn every_frozen_case_and_ram_lifetime_matches_portable_annex() {
        for case in &CASES {
            let mut copies = case.copy_inputs();
            let mut observations = Observations::default();
            let loaded = load(
                &case.texts(&copies)[..case.groups.len()],
                &mut observations,
                snapshot,
            );
            if case.lifetime == Lifetime::Dropped {
                copies.fill(None);
            }
            let result = aggregate(case, &case.config(), &loaded);
            verify(case, &loaded, &result);
            assert!(observations.count <= 32);
            if let Some((ref r, _)) = result {
                assert!(r.satellites().len() <= 16);
            }
            assert_eq!(case.id == 16, loaded.catalogue.is_err());
            assert_eq!(result, aggregate(case, &case.config(), &loaded));
            if case.lifetime != Lifetime::Flash {
                let control = &CASES[if case.id <= 18 { 1 } else { 6 }];
                let baseline = load(
                    &control.texts(&[None, None, None, None])[..control.groups.len()],
                    &mut Observations::default(),
                    snapshot,
                );
                assert_eq!(result, aggregate(control, &control.config(), &baseline));
            }
        }
    }
    #[test]
    fn whole_array_and_duplicate_key_checks_are_preserved() {
        let case = &CASES[0];
        let bad = alloc::format!("{} , null]", case.groups[0].text.trim_end_matches(']'));
        let loaded = load(&[&bad], &mut Observations::default(), snapshot);
        assert!(loaded.catalogue.is_ok()); // Invalid record, not failed document.
        assert_eq!(loaded.parse_errors.len(), 1);
        let bad = alloc::format!("{},", case.groups[0].text.trim_end_matches(']'));
        let mut obs = Observations::default();
        let loaded = load(&[&bad], &mut obs, snapshot);
        assert!(loaded.catalogue.is_err());
        assert_eq!(obs.count, 0); // No partial array accepted.
        let text = case.groups[0].text.replacen(
            '{',
            "{\"TIME_SYSTEM\":\"UTC\",\"TIME_SYSTEM\":\"UTC\",",
            1,
        );
        let loaded = load(&[&text], &mut Observations::default(), snapshot);
        assert_eq!(loaded.parse_errors.len(), 1);
    }
}
