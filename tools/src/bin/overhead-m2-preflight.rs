//! Bounded host preparation. Not a frozen manifest, S3 fit, or numeric oracle.
use overhead_core::{
    GeodeticPosition,
    catalogue::{Catalogue, CatalogueBuilder, CatalogueDiagnostic, RecordOrigin},
    catalogue_passes::{CataloguePasses, search_catalogue},
    passes::{EvaluationBudget, SearchConfig, SearchStatus, StopReason, search_satellite},
    sgp4::chrono::{NaiveDateTime, TimeDelta},
};
use serde::Serialize;
use serde_json::{Value, json, value::RawValue};
use std::{env, process::ExitCode};
#[path = "m2_preflight/inputs.rs"]
mod inputs;
#[path = "m2_preflight/memory.rs"]
mod memory;
#[global_allocator]
static METER: memory::Meter = memory::Meter::new();

type Result<T> = std::result::Result<T, String>;
const LIMIT: u64 = 200_000;
const USAGE: &str = "Usage: overhead-m2-preflight [--inputs]\nOffline candidate cases 01–20: ingestion/work/collection/requested-memory gates.\n--inputs emits deterministic source documents for prepare_m2_cases.py.\nNot a frozen manifest, backend occupancy, timing, target fit, or numeric oracle.\nExit 0: success/help; 1: invalid arguments or failed gate.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Lifetime {
    FlashControl,
    RamDropped,
    RamRetained,
}
#[derive(Clone, Serialize)]
struct Case {
    id: u8,
    groups: Vec<&'static str>,
    start_utc: &'static str,
    site_deg_km: [f64; 3],
    window_s: i64,
    shared_allowance: u64,
    lifetime: Lifetime,
    control: Option<u8>,
}
impl Case {
    fn config(&self) -> SearchConfig {
        SearchConfig::new(
            NaiveDateTime::parse_from_str(self.start_utc, "%Y-%m-%dT%H:%M:%SZ").unwrap(),
            TimeDelta::seconds(self.window_s),
            10_f64.to_radians(),
            TimeDelta::seconds(60),
            TimeDelta::seconds(5),
        )
        .unwrap()
    }
    fn observer(&self) -> GeodeticPosition {
        GeodeticPosition {
            latitude_rad: self.site_deg_km[0].to_radians(),
            longitude_rad: self.site_deg_km[1].to_radians(),
            altitude_km: self.site_deg_km[2],
        }
    }
    fn texts<'a>(&self, docs: &'a [inputs::Document]) -> Vec<&'a str> {
        self.groups
            .iter()
            .map(|key| docs.iter().find(|d| &d.key == key).unwrap().text.as_str())
            .collect()
    }
}
fn cases() -> Result<Vec<Case>> {
    let suitability: Value = serde_json::from_str(include_str!(
        "../../../docs/evaluations/m2-pool-suitability.json"
    ))
    .map_err(|e| e.to_string())?;
    let mut cases = Vec::new();
    for id in 1..=16 {
        let mut case = Case {
            id,
            groups: vec!["historical"],
            start_utc: "2006-06-26T00:00:00Z",
            site_deg_km: [39.007, -104.883, 2.187],
            window_s: if id == 1 { 3600 } else { 86400 },
            shared_allowance: 8 * LIMIT,
            lifetime: Lifetime::FlashControl,
            control: None,
        };
        if (3..=10).contains(&id) {
            let selected = &suitability["selected_cases"][usize::from(id - 3)];
            if selected["case_id"] != id {
                return Err("source suitability case order changed".into());
            }
            let search = &suitability["searches"][selected["search_index"]
                .as_u64()
                .ok_or("missing search index")?
                as usize];
            case.groups = vec![match id {
                3 => "mixed-5",
                4 => "mixed-9",
                5 => "mixed-12",
                6 => "mixed-16",
                7 | 8 => "leo-heavy",
                _ => "deep-space-heavy",
            }];
            case.start_utc = "2026-10-10T00:00:00Z";
            let site = &search["site"];
            case.site_deg_km = [
                site["latitude_deg"].as_f64().unwrap(),
                site["longitude_deg"].as_f64().unwrap(),
                site["altitude_km"].as_f64().unwrap(),
            ];
            case.shared_allowance = LIMIT * search["input_records"].as_u64().unwrap();
        }
        if id == 11 || id == 12 {
            let densest = suitability["densest_case_id"]
                .as_u64()
                .ok_or("missing densest ID")? as u8;
            case = cases
                .iter()
                .find(|c: &&Case| c.id == densest)
                .ok_or("densest case missing")?
                .clone();
            case.id = id;
            case.shared_allowance = if id == 11 { 1000 } else { 0 };
            case.control = Some(densest);
        }
        match id {
            13 => case.groups = vec!["historical"; 4],
            14 => {
                case.groups.push("conflict");
                case.shared_allowance = 7 * LIMIT;
            }
            15 => case.groups.push("invalid"),
            16 => {
                case.groups.push("malformed");
                case.shared_allowance = 0;
            }
            _ => {}
        }
        cases.push(case);
    }
    let densest = suitability["densest_case_id"].as_u64().unwrap() as u8;
    for (id, control, lifetime) in [
        (17, 2, Lifetime::RamDropped),
        (18, 2, Lifetime::RamRetained),
        (19, densest, Lifetime::RamDropped),
        (20, densest, Lifetime::RamRetained),
    ] {
        let mut case = cases[usize::from(control - 1)].clone();
        case.id = id;
        case.control = Some(control);
        case.lifetime = lifetime;
        cases.push(case);
    }
    Ok(cases)
}

#[derive(Debug, PartialEq)]
struct ParseError {
    origin: RecordOrigin,
    message: String,
}
enum Failure {
    Document {
        group: usize,
        message: String,
    },
    Empty {
        diagnostics: Vec<CatalogueDiagnostic>,
    },
}
struct Load {
    catalogue: std::result::Result<Catalogue, Failure>,
    parse_errors: Vec<ParseError>,
}
#[derive(Clone, Copy, Default, Serialize, Debug, PartialEq, Eq)]
struct RecordObservation {
    group: usize,
    record: usize,
    live: usize,
    allocation_calls: usize,
    requested_bytes: usize,
}
#[derive(Default)]
struct Observations {
    record_count: usize,
    records: [RecordObservation; 32],
    raw_capacities: [usize; 4],
}
fn load(texts: &[&str], mut observed: Option<&mut Observations>) -> Load {
    let mut builder = CatalogueBuilder::new();
    let mut parse_errors = Vec::new();
    let baseline = METER.snapshot().live;
    for (group, text) in texts.iter().enumerate() {
        let records: Vec<&RawValue> = match serde_json::from_str(text) {
            Ok(records) => records,
            Err(error) => {
                return Load {
                    catalogue: Err(Failure::Document {
                        group,
                        message: error.to_string(),
                    }),
                    parse_errors,
                };
            }
        };
        if let Some(o) = &mut observed {
            o.raw_capacities[group] = records.capacity();
        }
        for (record, raw) in records.into_iter().enumerate() {
            let before = METER.snapshot();
            let origin = RecordOrigin { group, record };
            match serde_json::from_str(raw.get()) {
                Ok(checked) => builder.push(origin, checked),
                Err(error) => parse_errors.push(ParseError {
                    origin,
                    message: error.to_string(),
                }),
            }
            if let Some(o) = &mut observed {
                let after = METER.snapshot();
                o.records[o.record_count] = RecordObservation {
                    group,
                    record,
                    live: after.live - baseline,
                    allocation_calls: after.calls - before.calls,
                    requested_bytes: after.requested - before.requested,
                };
                o.record_count += 1;
            }
        }
    }
    Load {
        catalogue: builder.finish().map_err(|e| Failure::Empty {
            diagnostics: e.diagnostics,
        }),
        parse_errors,
    }
}
fn origin(origin: RecordOrigin) -> Value {
    json!({"group":origin.group,"record":origin.record})
}
fn diagnostics(load: &Load) -> Value {
    let parse: Vec<_> = load
        .parse_errors
        .iter()
        .map(|e| json!({"origin":origin(e.origin),"message":e.message}))
        .collect();
    let core = match &load.catalogue {
        Ok(c) => c.diagnostics(),
        Err(Failure::Empty { diagnostics }) => diagnostics,
        _ => &[],
    };
    let core: Vec<_> = core.iter().map(|d| match d {
        CatalogueDiagnostic::InvalidRecord { origin: o, norad_id, error } => json!({"kind":"invalid-record","origin":origin(*o),"norad_id":norad_id,"error":format!("{error:?}")}),
        CatalogueDiagnostic::EpochConflict {norad_id, epoch, origins} => json!({"kind":"epoch-conflict","norad_id":norad_id,"epoch_utc":format!("{epoch:?}Z"),"origins":origins.iter().map(|o|origin(*o)).collect::<Vec<_>>()}),
    }).collect();
    let failure = match &load.catalogue {
        Ok(_) => Value::Null,
        Err(Failure::Document { group, message }) => {
            json!({"phase":"document","group":group,"message":message})
        }
        Err(Failure::Empty { .. }) => json!({"phase":"empty-catalogue"}),
    };
    json!({"parse":parse,"core":core,"failure":failure})
}
fn equivalent(a: &Load, b: &Load) -> bool {
    if a.parse_errors != b.parse_errors {
        return false;
    }
    match (&a.catalogue, &b.catalogue) {
        (Ok(a), Ok(b)) => {
            a.diagnostics() == b.diagnostics()
                && a.entries().len() == b.entries().len()
                && a.entries().iter().zip(b.entries()).all(|(a, b)| {
                    a.elements() == b.elements()
                        && a.groups() == b.groups()
                        && a.selected_from() == b.selected_from()
                })
        }
        (
            Err(Failure::Document {
                group: a,
                message: am,
            }),
            Err(Failure::Document {
                group: b,
                message: bm,
            }),
        ) => a == b && am == bm,
        _ => false,
    }
}
fn aggregate(case: &Case, load: &Load) -> Option<CataloguePasses> {
    load.catalogue.as_ref().ok().map(|c| {
        search_catalogue(
            c,
            case.observer(),
            &case.config(),
            LIMIT,
            &mut EvaluationBudget::new(case.shared_allowance),
        )
    })
}
fn verify(case: &Case, load: &Load, result: &Option<CataloguePasses>) -> Result<()> {
    let d = diagnostics(load);
    if case.id == 16 {
        if !matches!(load.catalogue, Err(Failure::Document { group: 1, .. }))
            || result.is_some()
            || !load.parse_errors.is_empty()
        {
            return Err("malformed document did not abort entire load".into());
        }
        return Ok(());
    }
    let catalogue = load
        .catalogue
        .as_ref()
        .map_err(|_| "unexpected failed load")?;
    let result = result.as_ref().ok_or("missing search")?;
    let expected_count = match case.id {
        3 => 5,
        4 => 9,
        5 => 12,
        6 => 16,
        7..=12 | 19 | 20 => 10,
        14 => 7,
        _ => 8,
    };
    if catalogue.entries().len() != expected_count {
        return Err("unexpected catalogue length".into());
    }
    match case.id {
        14 => {
            if !load.parse_errors.is_empty()
                || catalogue.diagnostics().len() != 1
                || catalogue
                    .entries()
                    .iter()
                    .any(|e| e.elements().norad_id == 6251)
            {
                return Err("conflict survival/diagnostic mismatch".into());
            }
            match &catalogue.diagnostics()[0] {
                CatalogueDiagnostic::EpochConflict {
                    norad_id: 6251,
                    origins,
                    ..
                } if origins
                    == &[
                        RecordOrigin {
                            group: 0,
                            record: 0,
                        },
                        RecordOrigin {
                            group: 1,
                            record: 1,
                        },
                    ] => {}
                _ => return Err("conflict origins mismatch".into()),
            }
        }
        15 => {
            if load.parse_errors.len() != 1
                || load.parse_errors[0].origin
                    != (RecordOrigin {
                        group: 1,
                        record: 0,
                    })
                || catalogue.diagnostics().len() != 1
            {
                return Err("rejection stages/counts mismatch".into());
            }
            match &catalogue.diagnostics()[0] {
                CatalogueDiagnostic::InvalidRecord {
                    origin:
                        RecordOrigin {
                            group: 1,
                            record: 1,
                        },
                    norad_id: 6251,
                    error: overhead_core::IngestError::EccentricityOutOfRange(1.0),
                } => {}
                _ => return Err("core rejection mismatch".into()),
            }
        }
        _ => {
            if !load.parse_errors.is_empty() || !catalogue.diagnostics().is_empty() {
                return Err(format!("unexpected diagnostics: {d}"));
            }
        }
    }
    for entry in catalogue.entries() {
        if entry.groups()
            != if case.id == 13 {
                &[0, 1, 2, 3][..]
            } else {
                &[0][..]
            }
            || entry.selected_from().group != 0
        {
            return Err("membership/provenance mismatch".into());
        }
    }
    let partial = case.id == 11 || case.id == 12;
    if result.is_complete() == partial || result.satellites().len() != expected_count {
        return Err("completion/report count mismatch".into());
    }
    let mut budget = EvaluationBudget::new(case.shared_allowance);
    for (entry, stored) in catalogue.entries().iter().zip(result.satellites()) {
        let mut passes = Vec::new();
        let report = search_satellite(
            entry.satellite(),
            case.observer(),
            &case.config(),
            LIMIT,
            &mut budget,
            |p| passes.push(p),
        );
        if stored.report() != &report
            || stored.passes() != passes
            || stored.norad_id() != entry.elements().norad_id
        {
            return Err("direct/retained search mismatch".into());
        }
        match &report.status {
            SearchStatus::Complete => {}
            SearchStatus::Unsearched(stop) | SearchStatus::Incomplete(stop)
                if partial && matches!(stop.reason, StopReason::TotalLimit) => {}
            _ => return Err("unexpected search failure".into()),
        }
    }
    if result.evaluations() != budget.used() || (partial && budget.used() != case.shared_allowance)
    {
        return Err("budget accounting mismatch".into());
    }
    Ok(())
}
fn search_report(result: &Option<CataloguePasses>) -> Value {
    match result {
        None => Value::Null,
        Some(result) => {
            json!({"complete":result.is_complete(),"evaluations":result.evaluations(),"stored_passes":result.satellites().iter().map(|s|s.passes().len()).sum::<usize>(),"earliest_candidates":result.earliest_candidates().len(),"satellites":result.satellites().iter().map(|s|json!({"norad_id":s.norad_id(),"evaluations":s.report().evaluations,"passes":s.passes().len(),"upcoming_passes":s.report().upcoming_passes,"status":format!("{:?}",s.report().status)})).collect::<Vec<_>>()})
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
struct Memory {
    input_copy: memory::Phase,
    initialization: memory::Phase,
    input_release: memory::Phase,
    aggregation: Option<memory::Phase>,
    destruction: memory::Phase,
    pipeline_peak: usize,
    raw_array_capacities: Vec<usize>,
    record_observations: Vec<RecordObservation>,
    entries_capacity: usize,
    core_diagnostics_capacity: usize,
    parse_diagnostics_capacity: usize,
    conflict_origins_capacity: usize,
    memberships_capacities: Vec<usize>,
    satellite_reports_capacity: usize,
    pass_capacities: Vec<usize>,
    arrival_candidates_capacity: usize,
}
fn measure(
    case: &Case,
    texts: &[&str],
    reference: &Load,
    expected: &Option<CataloguePasses>,
) -> Result<Memory> {
    // Fixed group table, no RAM allocation for the table itself. Source documents
    // are prepared outside measurement; "flash" is a borrowed-input host control.
    let mut copies: [Option<String>; 4] = [None, None, None, None];
    let mut observed = Observations::default();
    let mut memberships = [0; 16];
    let mut pass_capacities = [0; 16];
    let baseline = METER.start_pipeline();
    let phase = METER.begin();
    if case.lifetime != Lifetime::FlashControl {
        for (slot, text) in copies.iter_mut().zip(texts) {
            *slot = Some((*text).to_owned());
        }
    }
    let input_copy = METER.finish(phase);
    let phase = METER.begin();
    let loaded = {
        let mut borrowed = [""; 4];
        for (i, text) in texts.iter().enumerate() {
            borrowed[i] = copies[i].as_deref().unwrap_or(text);
        }
        load(&borrowed[..texts.len()], Some(&mut observed))
    };
    let initialization = METER.finish(phase);
    let phase = METER.begin();
    if case.lifetime == Lifetime::RamDropped {
        copies.fill(None);
    }
    let input_release = METER.finish(phase);
    let phase = METER.begin();
    let result = aggregate(case, &loaded);
    let candidates = result.as_ref().map(|r| r.earliest_candidates());
    let aggregation = result.as_ref().map(|_| METER.finish(phase));
    // Only nonallocating comparisons/observations inside the measured lifetime.
    let same = equivalent(&loaded, reference) && &result == expected;
    let mut capacities_valid = true;
    let mut capacity = |v: Option<usize>| {
        if v.is_none() {
            capacities_valid = false;
        }
        v.unwrap_or(0)
    };
    let parse_diagnostics_capacity = capacity(METER.capacity(&loaded.parse_errors));
    let (entries_capacity, core_diagnostics_capacity, conflict_origins_capacity, entries_len) =
        match &loaded.catalogue {
            Ok(c) => {
                for (i, e) in c.entries().iter().enumerate() {
                    memberships[i] = capacity(METER.capacity(e.groups()));
                }
                let conflict_capacity = c
                    .diagnostics()
                    .iter()
                    .filter_map(|d| match d {
                        CatalogueDiagnostic::EpochConflict { origins, .. } => {
                            Some(capacity(METER.capacity(origins)))
                        }
                        _ => None,
                    })
                    .sum();
                (
                    capacity(METER.capacity(c.entries())),
                    capacity(METER.capacity(c.diagnostics())),
                    conflict_capacity,
                    c.entries().len(),
                )
            }
            _ => (0, 0, 0, 0),
        };
    let (satellite_reports_capacity, reports_len) = match &result {
        Some(r) => {
            for (i, s) in r.satellites().iter().enumerate() {
                pass_capacities[i] = capacity(METER.capacity(s.passes()));
            }
            (
                capacity(METER.capacity(r.satellites())),
                r.satellites().len(),
            )
        }
        None => (0, 0),
    };
    let arrival_candidates_capacity = candidates
        .as_ref()
        .map_or(0, |c| capacity(METER.capacity(c)));
    let phase = METER.begin();
    drop(candidates);
    drop(result);
    drop(loaded);
    drop(copies);
    let destruction = METER.finish(phase);
    let (pipeline_peak, overflow) = METER.stop_pipeline();
    if !same
        || !capacities_valid
        || overflow
        || destruction.after.live != baseline.live
        || destruction.after.failures != 0
    {
        return Err(format!(
            "case {}: memory gate failed: same={same}, capacities={capacities_valid}, overflow={overflow}, released={}, failures={}",
            case.id,
            destruction.after.live == baseline.live,
            destruction.after.failures
        ));
    }
    // Normalize every occupancy to the SAME process baseline; unlike a phase
    // delta, aggregation includes the retained catalogue/diagnostics/RAM input.
    let normalize = |mut p: memory::Phase| {
        p.before.live -= baseline.live;
        p.after.live -= baseline.live;
        p.peak -= baseline.live;
        p
    };
    Ok(Memory {
        input_copy: normalize(input_copy),
        initialization: normalize(initialization),
        input_release: normalize(input_release),
        aggregation: aggregation.map(normalize),
        destruction: normalize(destruction),
        pipeline_peak: pipeline_peak - baseline.live,
        raw_array_capacities: observed.raw_capacities[..texts.len()].to_vec(),
        record_observations: observed.records[..observed.record_count].to_vec(),
        entries_capacity,
        core_diagnostics_capacity,
        parse_diagnostics_capacity,
        conflict_origins_capacity,
        memberships_capacities: memberships[..entries_len].to_vec(),
        satellite_reports_capacity,
        pass_capacities: pass_capacities[..reports_len].to_vec(),
        arrival_candidates_capacity,
    })
}

fn run() -> Result<Value> {
    let docs = inputs::documents();
    let pinned = inputs::check_pinned(&docs)?;
    let mut rows = Vec::new();
    for case in cases()? {
        let texts = case.texts(&docs);
        let bytes: usize = texts.iter().map(|t| t.len()).sum();
        let mut record_count = 0;
        for text in &texts {
            if *text == "[" {
                continue;
            }
            let records: Vec<&RawValue> = serde_json::from_str(text).map_err(|e| e.to_string())?;
            record_count += records.len();
        }
        if texts.len() > 4 || bytes > 32768 || record_count > 32 {
            return Err("case ceiling exceeded".into());
        }
        let loaded = load(&texts, None);
        let result = aggregate(&case, &loaded);
        verify(&case, &loaded, &result).map_err(|e| format!("case {}: {e}", case.id))?;
        let memory = measure(&case, &texts, &loaded, &result)?;
        if measure(&case, &texts, &loaded, &result)? != memory {
            return Err(format!(
                "case {}: allocation observations did not repeat",
                case.id
            ));
        }
        let accepted = loaded.catalogue.as_ref().ok().map(|c| c.entries().iter().map(|e|json!({"norad_id":e.elements().norad_id,"epoch_utc":format!("{:?}Z",e.elements().datetime),"groups":e.groups(),"selected_from":origin(e.selected_from())})).collect::<Vec<_>>());
        let groups: Vec<_> = case.groups.iter().enumerate().map(|(index,key)| {
            let doc = pinned["documents"].as_array().unwrap().iter().find(|d|d["key"]==*key).unwrap();
            json!({"index":index,"name":format!("m2-{}-group-{index}",case.id),"document":key,"bytes":doc["bytes"],"sha256":doc["sha256"],"records":doc["records"],"source_metadata":"static fixture references; no dynamic URLs or fetch strings in measured pipeline"})
        }).collect();
        rows.push(json!({"case":case,"groups":groups,"input_records":record_count,"input_json_bytes":bytes,"accepted":accepted,"diagnostics":diagnostics(&loaded),"search":search_report(&result),"requested_memory":memory}));
    }
    // RAM variants and equivalent/invalid ingestion must preserve full outputs,
    // not just work totals. Repeat equality of reports/passes against controls.
    let cases = cases()?;
    for derived in [13, 15, 17, 18, 19, 20] {
        let a = &cases[derived - 1];
        let control = a.control.unwrap_or(2);
        let b = &cases[usize::from(control - 1)];
        let a_load = load(&a.texts(&docs), None);
        let b_load = load(&b.texts(&docs), None);
        if aggregate(a, &a_load) != aggregate(b, &b_load)
            || (a.lifetime != Lifetime::FlashControl && !equivalent(&a_load, &b_load))
        {
            return Err("derived/control catalogue or search output mismatch".into());
        }
    }
    Ok(
        json!({"schema_version":1,"status":"candidate-host-preflight-not-frozen-manifest","host_os":env::consts::OS,"host_arch":env::consts::ARCH,"pointer_bits":usize::BITS,"measurement":"requested bytes with allocate-copy-free overlap; occupancies relative to one process baseline, not per-phase deltas; no backend footprint/timing/stack","threshold_deg":10,"detection_s":60,"crossing_tolerance_s":5,"per_satellite_allowance":LIMIT,"cases":rows}),
    )
}
fn main() -> ExitCode {
    let args: Vec<_> = env::args().skip(1).collect();
    if args == ["--help"] || args == ["-h"] {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let result = if args == ["--inputs"] {
        serde_json::to_value(inputs::documents()).map_err(|e| e.to_string())
    } else if args.is_empty() {
        run()
    } else {
        Err(USAGE.into())
    };
    match result.and_then(|r| serde_json::to_string_pretty(&r).map_err(|e| e.to_string())) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use overhead_core::OmmElements;
    #[test]
    fn documents_preserve_baseline_and_named_mutations() {
        let docs = inputs::documents();
        let get = |key| docs.iter().find(|d| d.key == key).unwrap().text.as_str();
        let base: Vec<OmmElements> = serde_json::from_str(get("historical")).unwrap();
        let conflict: Vec<OmmElements> = serde_json::from_str(get("conflict")).unwrap();
        assert_eq!(
            conflict[0].elements().datetime,
            base[0].elements().datetime - TimeDelta::days(1)
        );
        assert_eq!(
            conflict[1].elements().mean_anomaly,
            (base[0].elements().mean_anomaly + 1.) % 360.
        );
        assert_eq!(get("malformed"), "[");
    }
    #[test]
    fn invalid_records_are_not_failed_documents() {
        let docs = inputs::documents();
        for case in cases()
            .unwrap()
            .iter()
            .filter(|c| (13..=16).contains(&c.id))
        {
            let loaded = load(&case.texts(&docs), None);
            verify(case, &loaded, &aggregate(case, &loaded)).unwrap();
        }
    }
}
