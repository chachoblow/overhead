//! Offline distinct-catalogue costs. Host requested heap is NOT target peak RAM.
use std::{env, hint::black_box, process::ExitCode, time::Instant};

use overhead_core::{
    GeodeticPosition, OmmElements,
    catalogue::{Catalogue, CatalogueBuilder, RecordOrigin},
    catalogue_passes::{CataloguePasses, search_catalogue},
    passes::{EvaluationBudget, SearchConfig, search_satellite},
    sgp4::{
        Elements,
        chrono::{NaiveDateTime, TimeDelta},
    },
};
use serde::Serialize;
use serde_json::value::RawValue;

#[path = "catalogue_benchmark/memory.rs"]
mod memory;

#[global_allocator]
static ALLOCATOR: memory::CountingAllocator = memory::CountingAllocator::new();

type Result<T> = std::result::Result<T, String>;
const USAGE: &str = "Usage: overhead-catalogue-benchmark [--samples N] [--label TEXT]
Offline 4/8 distinct-satellite catalogue initialization and pass-storage costs.
JSON output; no network, files, hardware, wall clock, or operating defaults.
Samples: 1..50 (default 5), one invocation/sample after one discarded warm-up.
Host requested heap only: excludes allocator overhead, stack, RSS, and S3 RAM.
Exit 0: completed experiment/help; 1: invalid input or failed experiment.
See tools/README.md for measurement boundaries.";
const START: &str = "2006-06-26T00:00:00Z";
const SATELLITE_LIMIT: u64 = 200_000;
const IDS: [u64; 8] = [6251, 8195, 28129, 24208, 28057, 9880, 14128, 28626];
const CLASSES: [&str; 8] = [
    "LEO",
    "resonant-HEO",
    "GNSS",
    "GEO",
    "LEO",
    "resonant-HEO",
    "GEO",
    "GEO",
];

#[derive(Serialize)]
struct Options {
    samples: usize,
    label: String,
}

fn options(args: &[String]) -> Result<Options> {
    let mut result = Options {
        samples: 5,
        label: String::new(),
    };
    let mut seen = Vec::new();
    for pair in args.chunks(2) {
        let [key, value] = pair else {
            return Err(format!("missing option value\n{USAGE}"));
        };
        if seen.contains(key) {
            return Err(format!("duplicate option: {key}"));
        }
        seen.push(key.clone());
        match key.as_str() {
            "--samples" => {
                result.samples = value.parse().map_err(|_| "invalid sample count")?;
                if !(1..=50).contains(&result.samples) {
                    return Err("samples must be in 1..50".into());
                }
            }
            "--label" => result.label = value.clone(),
            _ => return Err(format!("unexpected option: {key}\n{USAGE}")),
        }
    }
    Ok(result)
}

fn start() -> NaiveDateTime {
    NaiveDateTime::parse_from_str(START, "%Y-%m-%dT%H:%M:%SZ").unwrap()
}

fn observer() -> GeodeticPosition {
    GeodeticPosition {
        latitude_rad: 39.007_f64.to_radians(),
        longitude_rad: (-104.883_f64).to_radians(),
        altitude_km: 2.187,
    }
}

#[derive(Serialize)]
struct Input {
    norad_id: u64,
    name: String,
    orbit_class: &'static str,
    epoch_utc: String,
    age_at_start_s: f64,
}

fn fixtures() -> Result<Vec<Elements>> {
    let lines: Vec<_> = include_str!("../../fixtures/catalogue-costs.tle")
        .lines()
        .collect();
    let (records, remainder) = lines.as_chunks::<3>();
    if records.len() != IDS.len() || !remainder.is_empty() {
        return Err("expected eight named distinct TLE records".into());
    }
    records
        .iter()
        .zip(IDS)
        .map(|(lines, id)| {
            let elements = Elements::from_tle(
                Some(lines[0].into()),
                lines[1].as_bytes(),
                lines[2].as_bytes(),
            )
            .map_err(|e| e.to_string())?;
            if elements.norad_id != id {
                return Err("fixture identity/order changed".into());
            }
            Ok(elements)
        })
        .collect()
}

fn build(json: &str) -> Result<Catalogue> {
    // Like the local loader, validate the entire array before parsing individual
    // checked records, retaining duplicate-key detection. One group, no I/O or
    // formatted provenance report. TLE -> OMM conversion is outside measurement.
    let records: Vec<&RawValue> = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut builder = CatalogueBuilder::new();
    for (record, raw) in records.into_iter().enumerate() {
        let elements: OmmElements = serde_json::from_str(raw.get()).map_err(|e| e.to_string())?;
        builder.push(RecordOrigin { group: 0, record }, elements);
    }
    builder.finish().map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
struct Work {
    satellites: usize,
    evaluations: u64,
    passes: usize,
    earliest_candidates: usize,
    searched: usize,
    complete: bool,
}

#[derive(Serialize)]
struct Phase {
    elapsed_ns: u64,
    heap: memory::Heap,
}

#[derive(Serialize)]
struct Sample {
    initialization: Phase,
    aggregation: Phase,
    /// Maximum requested heap attributable to the two phases combined. The
    /// prepared input JSON is excluded; catalogue stays live during aggregation.
    pipeline_peak_extra_bytes: usize,
    combined_retained_bytes: usize,
    work: Work,
}

fn measure<T>(run: impl FnOnce() -> T) -> (T, Phase) {
    let before = ALLOCATOR.begin();
    let start = Instant::now();
    let value = black_box(run());
    let elapsed_ns = start
        .elapsed()
        .as_nanos()
        .try_into()
        .expect("duration fits u64");
    let heap = ALLOCATOR.finish(before);
    (value, Phase { elapsed_ns, heap })
}

fn verify_search(
    catalogue: &Catalogue,
    config: &SearchConfig,
    limit: u64,
    result: &CataloguePasses,
) -> Result<()> {
    // Untimed same-model cross-check against direct streaming searches. This
    // validates retained reports/records, not independent orbital accuracy.
    let mut budget = EvaluationBudget::new(limit);
    for (entry, stored) in catalogue.entries().iter().zip(result.satellites()) {
        let mut passes = Vec::new();
        let report = search_satellite(
            entry.satellite(),
            observer(),
            config,
            SATELLITE_LIMIT,
            &mut budget,
            |pass| passes.push(pass),
        );
        if stored.norad_id() != entry.elements().norad_id
            || stored.report() != &report
            || stored.passes() != passes
        {
            return Err("aggregation differs from direct streaming search".into());
        }
    }
    if result.satellites().len() != catalogue.entries().len()
        || result.evaluations() != budget.used()
    {
        return Err("aggregation accounting mismatch".into());
    }
    Ok(())
}

fn sample(
    json: &str,
    ids: &[u64],
    config: &SearchConfig,
    limit: u64,
    expected_complete: bool,
) -> Result<Sample> {
    let baseline = ALLOCATOR.live();
    let (catalogue, initialization) = measure(|| build(black_box(json)));
    let catalogue = catalogue?;
    if !catalogue.diagnostics().is_empty()
        || catalogue.entries().len() != ids.len()
        || catalogue
            .entries()
            .iter()
            .any(|entry| !ids.contains(&entry.elements().norad_id))
    {
        return Err("catalogue lost or rejected an input".into());
    }
    let mut budget = EvaluationBudget::new(limit);
    let ((result, candidates), aggregation) = measure(|| {
        let result = search_catalogue(
            black_box(&catalogue),
            black_box(observer()),
            black_box(config),
            SATELLITE_LIMIT,
            &mut budget,
        );
        let candidates = result.earliest_candidates();
        (result, candidates)
    });
    if result.is_complete() != expected_complete {
        return Err(format!("unexpected completion: {result:?}"));
    }
    verify_search(&catalogue, config, limit, &result)?;
    let work = Work {
        satellites: result.satellites().len(),
        evaluations: result.evaluations(),
        passes: result.satellites().iter().map(|s| s.passes().len()).sum(),
        earliest_candidates: candidates.len(),
        searched: result
            .satellites()
            .iter()
            .filter(|s| s.report().evaluations > 0)
            .count(),
        complete: result.is_complete(),
    };
    drop(candidates);
    drop(result);
    drop(catalogue);
    if ALLOCATOR.live() != baseline {
        return Err("workload did not release all requested heap bytes".into());
    }
    Ok(Sample {
        pipeline_peak_extra_bytes: initialization
            .heap
            .peak_extra_bytes
            .max(initialization.heap.retained_extra_bytes + aggregation.heap.peak_extra_bytes),
        combined_retained_bytes: initialization.heap.retained_extra_bytes
            + aggregation.heap.retained_extra_bytes,
        initialization,
        aggregation,
        work,
    })
}

#[derive(Serialize)]
struct Row {
    satellites: usize,
    norad_ids: Vec<u64>,
    input_json_bytes: usize,
    window_s: i64,
    total_evaluation_limit: u64,
    samples: Vec<Sample>,
}

#[derive(Serialize)]
struct Report {
    schema_version: u32,
    measurement: &'static str,
    host_os: &'static str,
    host_arch: &'static str,
    pointer_bits: u32,
    debug_assertions: bool,
    options: Options,
    start_utc: &'static str,
    observer_lat_lon_deg_height_km: [f64; 3],
    detection_s: i64,
    tolerance_s: i64,
    min_elevation_deg: f64,
    satellite_evaluation_limit: u64,
    inputs: Vec<Input>,
    rows: Vec<Row>,
}

fn run(options: Options) -> Result<Report> {
    let elements = fixtures()?;
    let inputs = elements
        .iter()
        .zip(CLASSES)
        .map(|(e, orbit_class)| Input {
            norad_id: e.norad_id,
            name: e.object_name.clone().unwrap(),
            orbit_class,
            epoch_utc: format!("{}T{}Z", e.datetime.date(), e.datetime.time()),
            age_at_start_s: (start() - e.datetime).num_microseconds().unwrap() as f64 / 1e6,
        })
        .collect();
    let mut rows = Vec::new();
    for size in [4, 8] {
        // Standard sgp4 serialization of unmodified parsed TLE values. Checked
        // OMM uses its documented omitted-metadata defaults (Earth/TEME/UTC/SGP4).
        let json = serde_json::to_string(&elements[..size]).map_err(|e| e.to_string())?;
        for (window_s, limit, complete) in [
            (3600, SATELLITE_LIMIT * size as u64, true),
            (86400, SATELLITE_LIMIT * size as u64, true),
            (86400, 1000, false),
            (86400, 0, false),
        ] {
            let config = SearchConfig::new(
                start(),
                TimeDelta::seconds(window_s),
                10_f64.to_radians(),
                TimeDelta::seconds(60),
                TimeDelta::seconds(5),
            )
            .map_err(|e| e.to_string())?;
            let warmup = sample(&json, &IDS[..size], &config, limit, complete)?;
            let mut samples = Vec::with_capacity(options.samples);
            for _ in 0..options.samples {
                let current = sample(&json, &IDS[..size], &config, limit, complete)?;
                if current.work != warmup.work
                    || current.initialization.heap != warmup.initialization.heap
                    || current.aggregation.heap != warmup.aggregation.heap
                {
                    return Err("work or allocation counts changed between invocations".into());
                }
                samples.push(current);
            }
            rows.push(Row {
                satellites: size,
                norad_ids: IDS[..size].to_vec(),
                input_json_bytes: json.len(),
                window_s,
                total_evaluation_limit: limit,
                samples,
            });
        }
    }
    Ok(Report {
        schema_version: 1,
        measurement: "single-threaded-host-requested-heap-and-instrumented-time",
        host_os: env::consts::OS,
        host_arch: env::consts::ARCH,
        pointer_bits: usize::BITS,
        debug_assertions: cfg!(debug_assertions),
        options,
        start_utc: START,
        observer_lat_lon_deg_height_km: [39.007, -104.883, 2.187],
        detection_s: 60,
        tolerance_s: 5,
        min_elevation_deg: 10.0,
        satellite_evaluation_limit: SATELLITE_LIMIT,
        inputs,
        rows,
    })
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() == 1 && matches!(args[0].as_str(), "--help" | "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    match options(&args)
        .and_then(run)
        .and_then(|report| serde_json::to_string_pretty(&report).map_err(|e| e.to_string()))
    {
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
    fn fixture_metadata_and_numeric_precision_survive_checked_omm_conversion() {
        let elements = fixtures().unwrap();
        let json = serde_json::to_string(&elements).unwrap();
        let catalogue = build(&json).unwrap();
        assert!(catalogue.diagnostics().is_empty());
        assert_eq!(catalogue.entries().len(), IDS.len());
        for entry in catalogue.entries() {
            let original = elements
                .iter()
                .find(|e| e.norad_id == entry.elements().norad_id)
                .unwrap();
            let parsed = entry.elements();
            // serde_json's existing fast parser is not a bitwise f64 round-trip
            // contract. Bound conversion rounding without changing workspace
            // features or silently treating these as independent numeric fixtures.
            let mut normalized = original.clone();
            for (value, expected) in [
                (&mut normalized.mean_motion, parsed.mean_motion),
                (&mut normalized.mean_motion_dot, parsed.mean_motion_dot),
                (&mut normalized.mean_motion_ddot, parsed.mean_motion_ddot),
                (&mut normalized.drag_term, parsed.drag_term),
                (&mut normalized.inclination, parsed.inclination),
                (&mut normalized.right_ascension, parsed.right_ascension),
                (&mut normalized.eccentricity, parsed.eccentricity),
                (
                    &mut normalized.argument_of_perigee,
                    parsed.argument_of_perigee,
                ),
                (&mut normalized.mean_anomaly, parsed.mean_anomaly),
            ] {
                assert!((*value - expected).abs() <= 2.0 * f64::EPSILON * value.abs());
                *value = expected;
            }
            assert_eq!(parsed, &normalized);
            assert!((start() - original.datetime).num_hours().abs() < 48);
            assert_eq!(entry.groups(), &[0]);
        }
    }

    #[test]
    fn checked_ingestion_rejects_bad_documents_metadata_and_empty_catalogues() {
        for json in ["[]", "{}", "[", "[null]"] {
            assert!(build(json).is_err());
        }
        let elements = fixtures().unwrap();
        let json = serde_json::to_string(&elements[..1]).unwrap();
        for prefix in [
            "\"CENTER_NAME\":\"MARS\",",
            "\"TIME_SYSTEM\":\"UTC\",\"TIME_SYSTEM\":\"UTC\",",
        ] {
            let bad = json.replacen('{', &format!("{{{prefix}"), 1);
            assert!(build(&bad).is_err());
        }
        let catalogue = build(&json).unwrap();
        assert_eq!(catalogue.entries().len(), 1);
        assert_eq!(catalogue.entries()[0].elements(), &elements[0]);
    }
}
