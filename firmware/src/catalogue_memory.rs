//! Bounded experiment inputs/work, shared by target and host expectations.
//! No device operating defaults or independent numerical oracle.
use alloc::vec::Vec;
use overhead_core::{
    OmmElements,
    catalogue::{Catalogue, CatalogueBuilder, RecordOrigin},
    catalogue_passes::{Arrival, CataloguePasses, search_catalogue},
    passes::{EvaluationBudget, SearchConfig},
    sgp4::chrono::{NaiveDate, TimeDelta},
};
use serde_json::value::RawValue;

pub const IDS: [u64; 8] = [6251, 8195, 28129, 24208, 28057, 9880, 14128, 28626];
pub const SATELLITE_LIMIT: u64 = 200_000;
pub const CASE_COUNT: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Case {
    pub size: usize,
    pub window_s: i64,
    pub allowance: u64,
}

impl Case {
    pub fn at(index: usize) -> Option<Self> {
        if index >= CASE_COUNT {
            return None;
        }
        let size = if index < 4 { 4 } else { 8 };
        Some(Self {
            size,
            window_s: if index.is_multiple_of(4) { 3600 } else { 86400 },
            allowance: match index % 4 {
                2 => 1000,
                3 => 0,
                _ => SATELLITE_LIMIT * size as u64,
            },
        })
    }

    /// Build-script conversion of original TLEs; these bytes are flash rodata.
    pub fn json(self) -> &'static str {
        match self.size {
            4 => include_str!(concat!(env!("OUT_DIR"), "/catalogue-4.json")),
            8 => include_str!(concat!(env!("OUT_DIR"), "/catalogue-8.json")),
            _ => panic!("unsupported catalogue size"),
        }
    }

    pub fn config(self) -> SearchConfig {
        SearchConfig::new(
            NaiveDate::from_ymd_opt(2006, 6, 26)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            TimeDelta::seconds(self.window_s),
            10_f64.to_radians(),
            TimeDelta::seconds(60),
            TimeDelta::seconds(5),
        )
        .unwrap()
    }
}

pub fn build(json: &str) -> Result<Catalogue, &'static str> {
    let records: Vec<&RawValue> = serde_json::from_str(json).map_err(|_| "invalid array")?;
    let mut builder = CatalogueBuilder::new();
    for (record, raw) in records.into_iter().enumerate() {
        let elements: OmmElements = serde_json::from_str(raw.get()).map_err(|_| "invalid OMM")?;
        builder.push(RecordOrigin { group: 0, record }, elements);
    }
    builder.finish().map_err(|_| "empty catalogue")
}

pub fn validate_catalogue(catalogue: &Catalogue, case: Case) {
    assert!(catalogue.diagnostics().is_empty());
    assert_eq!(catalogue.entries().len(), case.size);
    for entry in catalogue.entries() {
        assert!(IDS[..case.size].contains(&entry.satellite().norad_id()));
    }
}

pub fn aggregate(
    catalogue: &Catalogue,
    config: &SearchConfig,
    allowance: u64,
) -> (CataloguePasses, Vec<Arrival>) {
    let result = search_catalogue(
        catalogue,
        crate::observer(),
        config,
        SATELLITE_LIMIT,
        &mut EvaluationBudget::new(allowance),
    );
    let candidates = result.earliest_candidates();
    (result, candidates)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Work {
    pub evaluations: u64,
    pub passes: usize,
    pub candidates: usize,
    pub searched: usize,
    pub complete: bool,
}

pub fn work(result: &CataloguePasses, candidates: &[Arrival]) -> Work {
    Work {
        evaluations: result.evaluations(),
        passes: result.satellites().iter().map(|s| s.passes().len()).sum(),
        candidates: candidates.len(),
        searched: result
            .satellites()
            .iter()
            .filter(|s| s.report().evaluations > 0)
            .count(),
        complete: result.is_complete(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use overhead_core::passes::search_satellite;

    #[test]
    fn matrix_matches_host_evidence_and_streamed_reports() {
        let expected = [
            (252, 1, 4),
            (5812, 6, 4),
            (1000, 2, 1),
            (0, 0, 0),
            (500, 3, 8),
            (11616, 13, 8),
            (1000, 2, 1),
            (0, 0, 0),
        ];
        for (index, (evaluations, passes, searched)) in expected.into_iter().enumerate() {
            let case = Case::at(index).unwrap();
            let catalogue = build(case.json()).unwrap();
            validate_catalogue(&catalogue, case);
            let config = case.config();
            let (result, candidates) = aggregate(&catalogue, &config, case.allowance);
            assert_eq!(
                work(&result, &candidates),
                Work {
                    evaluations,
                    passes,
                    searched,
                    candidates: usize::from(evaluations > 0),
                    complete: index % 4 < 2,
                }
            );
            let mut budget = EvaluationBudget::new(case.allowance);
            for (entry, stored) in catalogue.entries().iter().zip(result.satellites()) {
                let mut passes = Vec::new();
                let report = search_satellite(
                    entry.satellite(),
                    crate::observer(),
                    &config,
                    SATELLITE_LIMIT,
                    &mut budget,
                    |pass| passes.push(pass),
                );
                assert_eq!(stored.report(), &report);
                assert_eq!(stored.passes(), passes);
            }
            assert_eq!(budget.used(), result.evaluations());
            assert_eq!(
                aggregate(&catalogue, &config, case.allowance),
                (result, candidates)
            );
        }
        assert_eq!(Case::at(CASE_COUNT), None);
    }

    #[test]
    fn invalid_documents_are_not_successful_experiments() {
        for json in ["[]", "{}", "[", "[null]"] {
            assert!(build(json).is_err());
        }
        let json = Case::at(0).unwrap().json();
        for prefix in [
            "\"CENTER_NAME\":\"MARS\",",
            "\"TIME_SYSTEM\":\"UTC\",\"TIME_SYSTEM\":\"UTC\",",
        ] {
            assert!(build(&json.replacen('{', &alloc::format!("{{{prefix}"), 1)).is_err());
        }
    }
}
