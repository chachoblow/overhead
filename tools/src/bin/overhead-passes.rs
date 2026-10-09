//! Offline, explicit-input catalogue pass reporting; no wall clock or network.
use std::{env, fmt::Write, path::Path, process::ExitCode};

use overhead_core::{
    GeodeticPosition,
    catalogue_passes::{CataloguePasses, search_catalogue},
    passes::{
        CrossingBracket, DEFAULT_CROSSING_TOLERANCE, DEFAULT_LOOK_AHEAD, DEFAULT_MIN_ELEVATION_RAD,
        EvaluationBudget, PassEnd, PassStart, SearchConfig, SearchStatus, StopReason,
    },
    sgp4::chrono::{NaiveDateTime, TimeDelta},
};
use overhead_tools::catalogue::load;

const USAGE: &str = "Usage: overhead-passes MANIFEST.json UTC LAT_DEG LON_DEG HEIGHT_KM DETECTION_S SAT_LIMIT TOTAL_LIMIT [LOOK_AHEAD_S MIN_ELEVATION_DEG TOLERANCE_S]

Local catalogue manifest: same format/path rules as overhead-catalogue.
UTC: YYYY-MM-DDTHH:MM:SS[.fraction]Z, 1957–2100; no leap seconds/offsets.
Observer: WGS-84 latitude [-90,90], east longitude [-180,180], ellipsoidal km.
Durations: positive integer seconds. Allowances: unsigned evaluation counts;
zero is valid. Detection interval and both allowances are required, not defaults.
Optional tuning trio defaults to 86400 seconds / 10 degrees / 5 seconds.
No network, wall clock, freshness cutoff, or scheduler. Ascending NORAD traversal.
Reports retain brackets, open passes, stop details, and unsearched entries.
Complete means the configured procedure finished; short events can be missed.
Exit 0: complete search/help; 2: incomplete search WITH stdout results;
1: invalid inputs/load error, no partial stdout report.";

struct Inputs {
    path: String,
    observer: GeodeticPosition,
    config: SearchConfig,
    satellite_limit: u64,
    total_limit: u64,
}

fn number(text: &str, label: &str) -> Result<f64, String> {
    let n: f64 = text
        .parse()
        .map_err(|_| format!("invalid {label}: {text}"))?;
    if !n.is_finite() {
        return Err(format!("{label} must be finite"));
    }
    Ok(n)
}

fn duration(text: &str, label: &str) -> Result<TimeDelta, String> {
    let seconds: i64 = text
        .parse()
        .map_err(|_| format!("invalid {label}: expected integer seconds"))?;
    TimeDelta::try_seconds(seconds)
        .filter(|d| *d > TimeDelta::zero())
        .ok_or_else(|| format!("{label} must be positive and representable"))
}

impl Inputs {
    fn parse(args: &[String]) -> Result<Self, String> {
        if args.len() != 8 && args.len() != 11 {
            return Err(format!(
                "expected eight arguments or eleven with tuning trio\n\n{USAGE}"
            ));
        }
        let utc = args[1].strip_suffix('Z').ok_or("UTC must end in Z")?;
        let start = NaiveDateTime::parse_from_str(utc, "%Y-%m-%dT%H:%M:%S%.f")
            .map_err(|e| format!("invalid UTC: {e}"))?;
        let lat = number(&args[2], "latitude")?;
        let lon = number(&args[3], "longitude")?;
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            return Err("observer latitude/longitude out of range".into());
        }
        let observer = GeodeticPosition {
            latitude_rad: lat.to_radians(),
            longitude_rad: lon.to_radians(),
            altitude_km: number(&args[4], "height")?,
        };
        observer
            .to_ecef()
            .map_err(|e| format!("invalid observer: {e}"))?;
        let detection = duration(&args[5], "detection interval")?;
        let satellite_limit = args[6].parse().map_err(|_| "invalid satellite allowance")?;
        let total_limit = args[7].parse().map_err(|_| "invalid total allowance")?;
        let (window, threshold, tolerance) = if args.len() == 11 {
            (
                duration(&args[8], "look-ahead")?,
                number(&args[9], "minimum elevation")?.to_radians(),
                duration(&args[10], "crossing tolerance")?,
            )
        } else {
            (
                DEFAULT_LOOK_AHEAD,
                DEFAULT_MIN_ELEVATION_RAD,
                DEFAULT_CROSSING_TOLERANCE,
            )
        };
        let config = SearchConfig::new(start, window, threshold, detection, tolerance)
            .map_err(|e| format!("invalid search configuration: {e}"))?;
        Ok(Self {
            path: args[0].clone(),
            observer,
            config,
            satellite_limit,
            total_limit,
        })
    }
}

fn utc(time: NaiveDateTime) -> String {
    format!("{}T{}Z", time.date(), time.time())
}

fn bracket(b: CrossingBracket) -> String {
    format!(
        "[{}, {}]; tolerance {}",
        utc(b.lower()),
        utc(b.upper()),
        if b.meets_tolerance() {
            "met"
        } else {
            "NOT met"
        }
    )
}

fn prediction_report(inputs: &Inputs, results: &CataloguePasses) -> String {
    let mut out = String::new();
    let c = &inputs.config;
    writeln!(
        out,
        "\nPass search window (UTC): [{}, {}]",
        utc(c.start()),
        utc(c.end())
    )
    .unwrap();
    writeln!(
        out,
        "Observer WGS-84 (lat deg, lon deg, ellipsoid km): {} {} {}",
        inputs.observer.latitude_rad.to_degrees(),
        inputs.observer.longitude_rad.to_degrees(),
        inputs.observer.altitude_km
    )
    .unwrap();
    writeln!(
        out,
        "Minimum elevation (deg): {}",
        c.min_elevation_rad().to_degrees()
    )
    .unwrap();
    writeln!(
        out,
        "Detection interval (s): {}; crossing tolerance (s): {}",
        c.detection_interval().num_seconds(),
        c.crossing_tolerance().num_seconds()
    )
    .unwrap();
    writeln!(
        out,
        "Allowances (evaluations): per satellite {}; total {}",
        inputs.satellite_limit, inputs.total_limit
    )
    .unwrap();
    writeln!(
        out,
        "Search order: ascending NORAD ID; synchronous, not a scheduler."
    )
    .unwrap();
    writeln!(
        out,
        "Aggregate: {}; evaluations: {}",
        if results.is_complete() {
            "complete"
        } else {
            "INCOMPLETE"
        },
        results.evaluations()
    )
    .unwrap();
    writeln!(out, "Scope: accepted catalogue only; skipped/conflicting inputs are not searched (see ingestion warnings).").unwrap();
    writeln!(out, "Detection limits: brief excursions/gaps and extra crossings inside brackets may be missed; completion is not proof of all events. Brackets are model timing, not real-world accuracy.").unwrap();
    let candidates = results.earliest_candidates();
    if !results.is_complete() {
        writeln!(
            out,
            "Catalogue-wide next arrival: unknown (incomplete search)."
        )
        .unwrap();
    }
    if candidates.is_empty() {
        writeln!(out, "{}", if results.is_complete() {
            "Upcoming arrivals: none found within the searched window, subject to detection limits."
        } else {
            "Upcoming arrivals: none detected so far; NOT a complete no-pass result."
        }).unwrap();
    } else {
        writeln!(
            out,
            "Earliest detected arrival candidates: {} ({})",
            candidates.len(),
            if candidates.len() == 1 {
                "ordering resolved among detected arrivals"
            } else {
                "ordering unresolved; not a simultaneity claim"
            }
        )
        .unwrap();
        writeln!(
            out,
            "Candidate display order is NORAD ID/pass number, not arrival order."
        )
        .unwrap();
        for arrival in candidates {
            let satellite = &results.satellites()[arrival.satellite_index];
            writeln!(
                out,
                "  NORAD {} pass {}: {}",
                satellite.norad_id(),
                arrival.pass_index + 1,
                bracket(arrival.bracket)
            )
            .unwrap();
        }
    }
    for satellite in results.satellites() {
        let report = satellite.report();
        writeln!(out, "\nPrediction NORAD {}:", satellite.norad_id()).unwrap();
        let (status, stop) = match &report.status {
            SearchStatus::Complete => ("complete", None),
            SearchStatus::Incomplete(stop) => ("incomplete", Some(stop)),
            SearchStatus::Unsearched(stop) => ("unsearched", Some(stop)),
        };
        writeln!(
            out,
            "  Status: {status}; evaluations: {}; passes: {}; upcoming: {}",
            report.evaluations, report.passes, report.upcoming_passes
        )
        .unwrap();
        if let Some(stop) = stop {
            let reason = match &stop.reason {
                StopReason::SatelliteLimit => "satellite allowance exhausted".to_owned(),
                StopReason::TotalLimit => "total allowance exhausted".to_owned(),
                StopReason::Evaluation(e) => e.to_string(),
                StopReason::InvalidElevation(e) => format!("invalid elevation: {e}"),
            };
            writeln!(
                out,
                "  Stop: {} at {} during {:?}",
                reason,
                utc(stop.at),
                stop.phase
            )
            .unwrap();
        }
        match report.last_sample {
            Some(sample) => writeln!(
                out,
                "  Last regular sample: {}; elevation (deg): {}",
                utc(sample.at),
                sample.elevation_rad.to_degrees()
            )
            .unwrap(),
            None => writeln!(out, "  Last regular sample: none").unwrap(),
        }
        for (index, pass) in satellite.passes().iter().enumerate() {
            let start = match pass.start {
                PassStart::InProgress => "in progress at window start; actual start unknown".into(),
                PassStart::BoundaryStart => "boundary start; no observed arrival".into(),
                PassStart::Crossing(b) => format!("upward crossing {}", bracket(b)),
            };
            let end = match pass.end {
                PassEnd::Crossing(b) => format!("downward crossing {}", bracket(b)),
                PassEnd::BeyondWindow => {
                    "above threshold at completed window end; end unknown beyond window".into()
                }
                PassEnd::BoundaryUncertain => "equality at window end; crossing uncertain".into(),
                PassEnd::Interrupted => "unknown due to interrupted search".into(),
            };
            writeln!(out, "  Pass {} start: {start}\n    End: {end}", index + 1).unwrap();
        }
    }
    out
}

fn run(inputs: Inputs) -> Result<(String, String, bool), String> {
    let loaded = load(Path::new(&inputs.path))?;
    let results = search_catalogue(
        &loaded.catalogue,
        inputs.observer,
        &inputs.config,
        inputs.satellite_limit,
        &mut EvaluationBudget::new(inputs.total_limit),
    );
    let report = loaded.report + &prediction_report(&inputs, &results);
    Ok((report, loaded.diagnostics, results.is_complete()))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() == 1 && matches!(args[0].as_str(), "--help" | "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    match Inputs::parse(&args).and_then(run) {
        Ok((report, diagnostics, complete)) => {
            eprint!("{diagnostics}");
            print!("{report}");
            if complete {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            }
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
