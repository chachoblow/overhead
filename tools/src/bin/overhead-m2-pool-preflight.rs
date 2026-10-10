//! Offline source-suitability gate only; not the full M2 manifest or RAM preflight.
use std::{env, process::ExitCode};

use overhead_core::{
    GeodeticPosition, OmmElements, Satellite,
    catalogue::{Catalogue, CatalogueBuilder, RecordOrigin},
    catalogue_passes::search_catalogue,
    passes::{EvaluationBudget, SearchConfig, search_satellite},
    sgp4::chrono::{NaiveDateTime, TimeDelta},
};
use serde::Serialize;
use serde_json::{Value, value::RawValue};

type Result<T> = std::result::Result<T, String>;
const POOL: &str = include_str!("../../fixtures/m2-pool/pool.json");
const SELECTION: &str = include_str!("../../fixtures/m2-pool/selection.json");
const START: &str = "2026-10-10T00:00:00Z";
const LIMIT: u64 = 200_000;
const USAGE: &str = "Usage: overhead-m2-pool-preflight
Offline pinned 16-object source/path and complete-search suitability checks.
JSON includes four growth searches and all 30 density-site candidates.
Not a capture manifest, memory measurement, numeric oracle, or production policy.
No arguments except --help/-h. Exit 0: success/help; 1: failed gate/input.";

fn start() -> NaiveDateTime {
    NaiveDateTime::parse_from_str(START, "%Y-%m-%dT%H:%M:%SZ").unwrap()
}

// Inspect the *initialized* propagator, using sgp4 2.4's existing serde support.
// These private variant names are a version-bound diagnostic, not a core API.
// An upstream representation change must fail closed and trigger review.
fn initialized_path(satellite: &Satellite) -> Result<&'static str> {
    let constants = serde_json::to_value(satellite.constants()).map_err(|e| e.to_string())?;
    let method = &constants["method"];
    let path = if method.get("NearEarth").is_some() {
        "near-earth"
    } else if method.pointer("/DeepSpace/resonant/No").is_some() {
        "deep-space-nonresonant"
    } else if method
        .pointer("/DeepSpace/resonant/Yes/resonance/HalfDay")
        .is_some()
    {
        "deep-space-half-day-resonant"
    } else if method
        .pointer("/DeepSpace/resonant/Yes/resonance/OneDay")
        .is_some()
    {
        "deep-space-one-day-resonant"
    } else {
        return Err("unknown initialized sgp4 method; review pinned diagnostic".into());
    };
    if satellite.constants().initial_state().is_some()
        != matches!(
            path,
            "deep-space-half-day-resonant" | "deep-space-one-day-resonant"
        )
    {
        return Err("resonance-state/path disagreement".into());
    }
    Ok(path)
}

#[derive(Serialize)]
struct Input {
    norad_id: u64,
    epoch_utc: String,
    age_at_start_microseconds: i64,
    initialized_path: &'static str,
}

fn check_pool(raw: &[&RawValue], selection: &Value) -> Result<Vec<Input>> {
    if raw.len() != 16
        || selection["start_utc"] != START
        || selection["pool_bytes"] != POOL.len()
        || selection["records"].as_array().map(Vec::len) != Some(16)
    {
        return Err("pool/selection shape, UTC, or length mismatch".into());
    }
    let deep_classes = ["heo", "gnss", "geo", "heo", "gnss", "geo", "geo", "geo"];
    let mut inputs = Vec::new();
    let mut class_ids = std::collections::BTreeMap::<&str, Vec<u64>>::new();
    for (index, raw) in raw.iter().enumerate() {
        let checked: OmmElements = serde_json::from_str(raw.get()).map_err(|e| e.to_string())?;
        let elements = checked.elements();
        let satellite = Satellite::from_elements(elements).map_err(|e| e.to_string())?;
        let population = if index % 2 == 0 {
            "leo"
        } else {
            deep_classes[index / 2]
        };
        let path = initialized_path(&satellite)?;
        let expected = match population {
            "leo" => "near-earth",
            "heo" => "deep-space-half-day-resonant",
            "gnss" => "deep-space-nonresonant",
            "geo" => "deep-space-one-day-resonant",
            _ => unreachable!(),
        };
        let age = (start() - elements.datetime)
            .num_microseconds()
            .ok_or("epoch interval overflow")?;
        let selected = &selection["records"][index];
        let epoch_utc = selected["epoch_utc"]
            .as_str()
            .ok_or("missing epoch")?
            .to_owned();
        let selected_epoch = NaiveDateTime::parse_from_str(&epoch_utc, "%Y-%m-%dT%H:%M:%S%.fZ")
            .map_err(|e| e.to_string())?;
        if path != expected
            || age.unsigned_abs() > 48 * 3600 * 1_000_000
            || selected["norad_id"] != elements.norad_id
            || selected_epoch != elements.datetime
            || selected["population"] != population
            || selected["name"].as_str() != elements.object_name.as_deref()
            || inputs
                .iter()
                .any(|input: &Input| input.norad_id == elements.norad_id)
        {
            return Err(format!("unsuitable or mismatched pool record {index}"));
        }
        class_ids
            .entry(population)
            .or_default()
            .push(elements.norad_id);
        inputs.push(Input {
            norad_id: elements.norad_id,
            epoch_utc,
            age_at_start_microseconds: age,
            initialized_path: path,
        });
    }
    if class_ids
        .values()
        .any(|ids| ids.windows(2).any(|pair| pair[0] >= pair[1]))
    {
        return Err("population IDs not in ascending order".into());
    }
    Ok(inputs)
}

fn population(raw: &[&RawValue], indices: &[usize]) -> Result<(Catalogue, usize)> {
    // Exact subset representation: raw source objects with the pool's LF separators.
    let json = format!(
        "[\n{}\n]\n",
        indices
            .iter()
            .map(|&i| raw[i].get())
            .collect::<Vec<_>>()
            .join(",\n")
    );
    if indices.len() > 16 || json.len() > 32768 {
        return Err("population exceeds contract ceilings".into());
    }
    let records: Vec<&RawValue> = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let mut builder = CatalogueBuilder::new();
    for (record, raw) in records.into_iter().enumerate() {
        let checked = serde_json::from_str(raw.get()).map_err(|e| e.to_string())?;
        builder.push(RecordOrigin { group: 0, record }, checked);
    }
    let catalogue = builder.finish().map_err(|e| e.to_string())?;
    if !catalogue.diagnostics().is_empty() || catalogue.entries().len() != indices.len() {
        return Err("population lost/rejected records".into());
    }
    Ok((catalogue, json.len()))
}

#[derive(Clone, Copy, Serialize)]
struct Site {
    latitude_deg: f64,
    longitude_deg: f64,
    altitude_km: f64,
}

impl Site {
    fn observer(self) -> GeodeticPosition {
        GeodeticPosition {
            latitude_rad: self.latitude_deg.to_radians(),
            longitude_rad: self.longitude_deg.to_radians(),
            altitude_km: self.altitude_km,
        }
    }
}

#[derive(Serialize)]
struct SatelliteWork {
    norad_id: u64,
    evaluations: u64,
    passes: usize,
}

#[derive(Serialize)]
struct Search {
    population: String,
    site: Site,
    input_records: usize,
    input_json_bytes: usize,
    evaluations: u64,
    stored_passes: usize,
    earliest_candidates: usize,
    complete: bool,
    satellites: Vec<SatelliteWork>,
}

fn search(raw: &[&RawValue], indices: &[usize], name: &str, site: Site) -> Result<Search> {
    let (catalogue, bytes) = population(raw, indices)?;
    let config = SearchConfig::new(
        start(),
        TimeDelta::seconds(86400),
        10_f64.to_radians(),
        TimeDelta::seconds(60),
        TimeDelta::seconds(5),
    )
    .map_err(|e| e.to_string())?;
    let mut budget = EvaluationBudget::new(LIMIT * indices.len() as u64);
    let result = search_catalogue(&catalogue, site.observer(), &config, LIMIT, &mut budget);
    if !result.is_complete() || result.evaluations() != budget.used() {
        return Err(format!(
            "{name} failed/incomplete at {},{}; review contract",
            site.latitude_deg, site.longitude_deg
        ));
    }
    // Cross-check aggregation against direct streaming searches, including every
    // pass/report, not just counts. Same-model consistency, not numerical truth.
    let mut direct_budget = EvaluationBudget::new(LIMIT * indices.len() as u64);
    let mut satellites = Vec::new();
    for (entry, stored) in catalogue.entries().iter().zip(result.satellites()) {
        let before = direct_budget.used();
        let mut passes = Vec::new();
        let report = search_satellite(
            entry.satellite(),
            site.observer(),
            &config,
            LIMIT,
            &mut direct_budget,
            |pass| passes.push(pass),
        );
        if &report != stored.report()
            || passes != stored.passes()
            || stored.norad_id() != entry.elements().norad_id
        {
            return Err("streaming/retained search disagreement".into());
        }
        satellites.push(SatelliteWork {
            norad_id: stored.norad_id(),
            evaluations: direct_budget.used() - before,
            passes: passes.len(),
        });
    }
    if direct_budget.used() != budget.used() || satellites.len() != indices.len() {
        return Err("streaming/retained accounting disagreement".into());
    }
    Ok(Search {
        population: name.into(),
        site,
        input_records: indices.len(),
        input_json_bytes: bytes,
        evaluations: result.evaluations(),
        stored_passes: satellites.iter().map(|s| s.passes).sum(),
        earliest_candidates: result.earliest_candidates().len(),
        complete: true,
        satellites,
    })
}

// Candidates are already ordered by ascending (latitude, longitude). Include the
// index in each key to make the tie-break explicit, even if iteration changes.
fn density_extrema(rows: &[Search]) -> (usize, usize) {
    let high = (0..rows.len())
        .min_by_key(|&i| (std::cmp::Reverse(rows[i].stored_passes), i))
        .unwrap();
    let low = (0..rows.len())
        .filter(|&i| i != high)
        .min_by_key(|&i| (rows[i].stored_passes, i))
        .unwrap();
    (high, low)
}

#[derive(Serialize)]
struct SelectedCase {
    case_id: u8,
    search_index: usize,
}

#[derive(Serialize)]
struct Report {
    schema_version: u8,
    status: &'static str,
    pool_sha256: String,
    start_utc: &'static str,
    lookahead_s: u64,
    threshold_deg: u8,
    detection_s: u8,
    crossing_tolerance_s: u8,
    per_satellite_allowance: u64,
    inputs: Vec<Input>,
    searches: Vec<Search>,
    selected_cases: Vec<SelectedCase>,
    densest_case_id: u8,
}

fn run() -> Result<Report> {
    let raw: Vec<&RawValue> = serde_json::from_str(POOL).map_err(|e| e.to_string())?;
    let selection: Value = serde_json::from_str(SELECTION).map_err(|e| e.to_string())?;
    let inputs = check_pool(&raw, &selection)?;
    let mut searches = Vec::new();
    let mut selected_cases = Vec::new();
    for (index, size) in [5, 9, 12, 16].into_iter().enumerate() {
        searches.push(search(
            &raw,
            &(0..size).collect::<Vec<_>>(),
            &format!("mixed-{size}"),
            Site {
                latitude_deg: 39.007,
                longitude_deg: -104.883,
                altitude_km: 2.187,
            },
        )?);
        selected_cases.push(SelectedCase {
            case_id: 3 + index as u8,
            search_index: index,
        });
    }
    for (offset, name) in [(0, "leo-heavy"), (1, "deep-space-heavy")] {
        let indices: Vec<_> = (offset..16)
            .step_by(2)
            .chain((1 - offset..4).step_by(2))
            .collect();
        let first = searches.len();
        for latitude_deg in [-60., -30., 0., 30., 60.] {
            for longitude_deg in [-120., 0., 120.] {
                searches.push(search(
                    &raw,
                    &indices,
                    name,
                    Site {
                        latitude_deg,
                        longitude_deg,
                        altitude_km: 0.,
                    },
                )?);
            }
        }
        let (high, low) = density_extrema(&searches[first..]);
        for (delta, index) in [(0, high), (1, low)] {
            selected_cases.push(SelectedCase {
                case_id: 7 + 2 * offset as u8 + delta,
                search_index: first + index,
            });
        }
    }
    let densest_case_id = selected_cases
        .iter()
        .min_by_key(|case| {
            (
                std::cmp::Reverse(searches[case.search_index].stored_passes),
                case.case_id,
            )
        })
        .unwrap()
        .case_id;
    Ok(Report {
        schema_version: 1,
        status: "source-suitability-only-not-frozen-manifest",
        pool_sha256: selection["pool_sha256"]
            .as_str()
            .ok_or("missing pool hash")?
            .into(),
        start_utc: START,
        lookahead_s: 86400,
        threshold_deg: 10,
        detection_s: 60,
        crossing_tolerance_s: 5,
        per_satellite_allowance: LIMIT,
        inputs,
        searches,
        selected_cases,
        densest_case_id,
    })
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args().skip(1).collect();
    if args == ["--help"] || args == ["-h"] {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let result = if args.is_empty() {
        run()
    } else {
        Err(USAGE.into())
    }
    .and_then(|report| serde_json::to_string_pretty(&report).map_err(|e| e.to_string()));
    match result {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_pool_and_search_counts() {
        let report = run().unwrap();
        assert_eq!(report.searches.len(), 34);
        assert_eq!(report.selected_cases.len(), 8);
        let expected: Value = serde_json::from_str(include_str!(
            "../../../docs/evaluations/m2-pool-suitability.json"
        ))
        .unwrap();
        assert_eq!(serde_json::to_value(report).unwrap(), expected);
    }

    #[test]
    fn pool_gate_rejects_changed_identity_epoch_class_and_order() {
        let raw: Vec<&RawValue> = serde_json::from_str(POOL).unwrap();
        let selection: Value = serde_json::from_str(SELECTION).unwrap();
        for (field, value) in [
            ("norad_id", Value::from(1)),
            ("epoch_utc", Value::from("2006-06-26T00:00:00Z")),
            ("population", Value::from("geo")),
        ] {
            let mut changed = selection.clone();
            changed["records"][0][field] = value;
            assert!(check_pool(&raw, &changed).is_err());
        }
        let mut reordered = raw.clone();
        reordered.swap(0, 2);
        assert!(check_pool(&reordered, &selection).is_err());
        assert!(check_pool(&raw[..15], &selection).is_err());
    }

    #[test]
    fn extrema_ties_choose_first_then_first_remaining() {
        let raw: Vec<&RawValue> = serde_json::from_str(POOL).unwrap();
        let row = || {
            search(
                &raw,
                &[0],
                "test",
                Site {
                    latitude_deg: 0.,
                    longitude_deg: 0.,
                    altitude_km: 0.,
                },
            )
            .unwrap()
        };
        let mut rows = vec![row(), row(), row()];
        assert_eq!(density_extrema(&rows), (0, 1));
        rows[1].stored_passes += 1;
        assert_eq!(density_extrema(&rows), (1, 0));
    }
}
