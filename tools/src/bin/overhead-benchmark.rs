//! Offline host cost measurements, not device limits or detection guarantees.
use std::{env, hint::black_box, process::ExitCode, time::Instant};

use overhead_core::{
    GeodeticPosition, ecef_to_look_angles,
    passes::{EvaluationBudget, SearchConfig, search_satellite},
    sgp4::chrono::{NaiveDateTime, TimeDelta},
};
use overhead_tools::historical_orbits::{HistoricalOrbit, historical_orbits};
use serde::Serialize;

type Result<T> = std::result::Result<T, String>;
const USAGE: &str = "Usage: overhead-benchmark [--samples N] [--min-sample-ms N] [--label TEXT]\n\nOffline host propagation/geometry/pass-search benchmark; prints JSON.\nUse cargo run --release -p overhead-tools --bin overhead-benchmark -- ...\nDefaults: 5 samples, 20 ms minimum calibration target; not operating defaults.\nSamples: 1..50; minimum sample target: 1..1000 ms. Label records host/build context.\nNo hardware or network. See tools/README.md for methodology and exclusions.\nExit 0: all workloads complete, or --help/-h; 1: invalid input/failed workload.";
const SEARCH_LIMIT: u64 = 200_000;
const MAX_ITERATIONS: u64 = 1 << 20;
const TRACK_STEPS: usize = 1441;

#[derive(Debug, Serialize)]
struct Options {
    samples: usize,
    min_sample_ms: u64,
    label: String,
}

fn options(args: &[String]) -> Result<Options> {
    let mut options = Options {
        samples: 5,
        min_sample_ms: 20,
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
                options.samples = value.parse().map_err(|_| "invalid sample count")?;
                if !(1..=50).contains(&options.samples) {
                    return Err("samples must be in 1..50".into());
                }
            }
            "--min-sample-ms" => {
                options.min_sample_ms = value.parse().map_err(|_| "invalid sample duration")?;
                if !(1..=1000).contains(&options.min_sample_ms) {
                    return Err("min-sample-ms must be in 1..1000".into());
                }
            }
            "--label" => options.label = value.clone(),
            _ => return Err(format!("unexpected option: {key}\n{USAGE}")),
        }
    }
    Ok(options)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
struct Work {
    // Per workload invocation, not per timing sample (which may batch invocations).
    state_evaluations: u64,
    searches: u64,
    passes: u64,
}

#[derive(Serialize)]
struct Timing {
    iterations_per_sample: u64,
    elapsed_ns: Vec<u64>,
    min_ns_per_invocation: f64,
    median_ns_per_invocation: f64,
    max_ns_per_invocation: f64,
    work_per_invocation: Work,
}

fn summarize(elapsed_ns: Vec<u64>, iterations: u64, work: Work) -> Timing {
    let mut sorted = elapsed_ns.clone();
    sorted.sort_unstable();
    let n = sorted.len();
    let median = (sorted[(n - 1) / 2] as f64 + sorted[n / 2] as f64) / 2.0;
    Timing {
        iterations_per_sample: iterations,
        min_ns_per_invocation: sorted[0] as f64 / iterations as f64,
        median_ns_per_invocation: median / iterations as f64,
        max_ns_per_invocation: sorted[n - 1] as f64 / iterations as f64,
        elapsed_ns,
        work_per_invocation: work,
    }
}

fn timed_batch(
    iterations: u64,
    expected: Work,
    run: &mut impl FnMut() -> Result<Work>,
) -> Result<u64> {
    let start = Instant::now();
    for _ in 0..iterations {
        if black_box(run()?) != expected {
            return Err("work counts changed between benchmark invocations".into());
        }
    }
    start
        .elapsed()
        .as_nanos()
        .try_into()
        .map_err(|_| "timer overflow".into())
}

fn measure(options: &Options, mut run: impl FnMut() -> Result<Work>) -> Result<Timing> {
    let expected = black_box(run()?); // Untimed warm-up, also establishes work counts.
    let target_ns = options.min_sample_ms * 1_000_000;
    let mut iterations = 1;
    loop {
        if timed_batch(iterations, expected, &mut run)? >= target_ns {
            break;
        }
        if iterations == MAX_ITERATIONS {
            return Err("calibration target not reached within iteration guard".into());
        }
        iterations *= 2;
    }
    let elapsed = (0..options.samples)
        .map(|_| timed_batch(iterations, expected, &mut run))
        .collect::<Result<Vec<_>>>()?;
    Ok(summarize(elapsed, iterations, expected))
}

fn observer() -> GeodeticPosition {
    GeodeticPosition {
        latitude_rad: 39.007_f64.to_radians(),
        longitude_rad: (-104.883_f64).to_radians(),
        altitude_km: 2.187,
    }
}

struct Prepared<'a> {
    orbit: &'a HistoricalOrbit,
    times: Vec<NaiveDateTime>,
}

fn prepare(orbits: &[HistoricalOrbit]) -> Vec<Prepared<'_>> {
    orbits
        .iter()
        .map(|orbit| Prepared {
            orbit,
            times: (0..TRACK_STEPS)
                .map(|i| {
                    orbit.satellite.epoch() - TimeDelta::hours(12)
                        + TimeDelta::seconds(i as i64 * 60)
                })
                .collect(),
        })
        .collect()
}

// Repeated fixture slots intentionally do NOT fabricate a deduplicated catalogue.
fn track(slots: &[&Prepared<'_>], geometry: bool) -> Result<Work> {
    let observer = black_box(observer());
    for step in 0..TRACK_STEPS {
        for input in slots {
            let state = black_box(&input.orbit.satellite)
                .state_at(black_box(input.times[step]))
                .map_err(|e| e.to_string())?;
            if geometry {
                let ecef = state.to_ecef().map_err(|e| e.to_string())?;
                black_box(ecef_to_look_angles(ecef, observer).map_err(|e| e.to_string())?);
            } else {
                black_box(state);
            }
        }
    }
    Ok(Work {
        state_evaluations: (TRACK_STEPS * slots.len()) as u64,
        searches: 0,
        passes: 0,
    })
}

fn config(
    input: &Prepared<'_>,
    window_s: i64,
    detection_s: i64,
    tolerance_ms: i64,
) -> SearchConfig {
    SearchConfig::new(
        input.times[0],
        TimeDelta::seconds(window_s),
        10_f64.to_radians(),
        TimeDelta::seconds(detection_s),
        TimeDelta::milliseconds(tolerance_ms),
    )
    .expect("fixed benchmark configuration is valid")
}

fn predict(slots: &[&Prepared<'_>], configs: &[SearchConfig]) -> Result<Work> {
    let mut work = Work {
        state_evaluations: 0,
        searches: 0,
        passes: 0,
    };
    let mut budget = EvaluationBudget::new(SEARCH_LIMIT * slots.len() as u64);
    for (input, config) in slots.iter().zip(configs) {
        let report = search_satellite(
            black_box(&input.orbit.satellite),
            black_box(observer()),
            black_box(config),
            SEARCH_LIMIT,
            &mut budget,
            |pass| {
                black_box(pass);
                work.passes += 1;
            },
        );
        if !report.is_complete() {
            return Err(format!(
                "{}: incomplete search: {:?}",
                input.orbit.name, report.status
            ));
        }
        work.searches += 1;
        work.state_evaluations += report.evaluations;
        let _ = black_box(report);
    }
    if work.state_evaluations != budget.used() || slots.len() != configs.len() {
        return Err("inconsistent benchmark search accounting".into());
    }
    Ok(work)
}

#[derive(Serialize)]
struct Row {
    operation: &'static str,
    orbit_class: &'static str,
    slots: usize,
    tracking_ticks: usize,
    window_s: Option<i64>,
    detection_s: Option<i64>,
    tolerance_ms: Option<i64>,
    timing: Timing,
}

fn tracking_row(
    options: &Options,
    slots: &[&Prepared<'_>],
    geometry: bool,
    class: &'static str,
) -> Result<Row> {
    Ok(Row {
        operation: if geometry {
            "state_at+ecef+look_angles"
        } else {
            "state_at"
        },
        orbit_class: class,
        slots: slots.len(),
        tracking_ticks: TRACK_STEPS,
        window_s: None,
        detection_s: None,
        tolerance_ms: None,
        timing: measure(options, || track(slots, geometry))?,
    })
}

fn prediction_row(
    options: &Options,
    slots: &[&Prepared<'_>],
    class: &'static str,
    window_s: i64,
    detection_s: i64,
    tolerance_ms: i64,
) -> Result<Row> {
    let configs: Vec<_> = slots
        .iter()
        .map(|input| config(input, window_s, detection_s, tolerance_ms))
        .collect();
    Ok(Row {
        operation: "search_satellite",
        orbit_class: class,
        slots: slots.len(),
        tracking_ticks: 0,
        window_s: Some(window_s),
        detection_s: Some(detection_s),
        tolerance_ms: Some(tolerance_ms),
        timing: measure(options, || predict(slots, &configs))?,
    })
}

#[derive(Serialize)]
struct Input {
    name: &'static str,
    orbit_class: &'static str,
    norad_id: u64,
    epoch_utc: String,
    start_utc: String,
}

#[derive(Serialize)]
struct Report {
    schema_version: u32,
    scope: &'static str,
    timing_method: &'static str,
    workload_method: &'static str,
    os: &'static str,
    arch: &'static str,
    debug_assertions: bool,
    package_version: &'static str,
    options: Options,
    search_evaluation_guard_per_slot: u64,
    observer_lat_lon_height_deg_km: [f64; 3],
    threshold_deg: f64,
    inputs: Vec<Input>,
    rows: Vec<Row>,
}

fn run(options: Options) -> Result<String> {
    let orbits = historical_orbits()?;
    let prepared = prepare(&orbits);
    let mut rows = Vec::new();
    for input in &prepared {
        let slots = [input];
        for geometry in [false, true] {
            rows.push(tracking_row(&options, &slots, geometry, input.orbit.class)?);
        }
        for window_s in [3600, 86400] {
            for detection_s in [5, 30, 60] {
                for tolerance_ms in [250, 5000] {
                    rows.push(prediction_row(
                        &options,
                        &slots,
                        input.orbit.class,
                        window_s,
                        detection_s,
                        tolerance_ms,
                    )?);
                }
            }
        }
    }
    for count in [4, 16, 64] {
        let slots: Vec<_> = (0..count).map(|i| &prepared[i % prepared.len()]).collect();
        rows.push(tracking_row(
            &options,
            &slots,
            true,
            "mixed-repeated-fixtures",
        )?);
        rows.push(prediction_row(
            &options,
            &slots,
            "mixed-repeated-fixtures",
            86400,
            60,
            5000,
        )?);
    }
    let report = Report {
        schema_version: 1,
        scope: "host kernel throughput only; not ESP32 cost, catalogue capacity, scheduling, memory bounds, detection accuracy, or operating defaults",
        timing_method: "std::time::Instant wall time; one untimed warm-up; double batch iterations to minimum target; discard calibration; fixed-size repeated samples; black_box inputs/outputs; raw ns and min/median/max per invocation; no outlier removal",
        workload_method: "one fixture per class; starts at each own epoch -12h (not a common-UTC catalogue); tracking uses 1441 precomputed times spaced 60s through epoch +12h; mixed slots repeat LEO/HEO/GEO/GNSS equally; searches count/consume streamed passes without storing them; loading, initialization, time/config preparation, JSON, catalogue allocation/aggregation, UI and network excluded",
        os: env::consts::OS,
        arch: env::consts::ARCH,
        debug_assertions: cfg!(debug_assertions),
        package_version: env!("CARGO_PKG_VERSION"),
        options,
        search_evaluation_guard_per_slot: SEARCH_LIMIT,
        observer_lat_lon_height_deg_km: [39.007, -104.883, 2.187],
        threshold_deg: 10.0,
        inputs: prepared
            .iter()
            .map(|input| Input {
                name: input.orbit.name,
                orbit_class: input.orbit.class,
                norad_id: input.orbit.satellite.norad_id(),
                epoch_utc: format!("{}Z", input.orbit.satellite.epoch()),
                start_utc: format!("{}Z", input.times[0]),
            })
            .collect(),
        rows,
    };
    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() == 1 && matches!(args[0].as_str(), "--help" | "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    match options(&args).and_then(run) {
        Ok(report) => {
            println!("{report}");
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
    fn timing_summary_normalizes_batches_and_preserves_raw_order() {
        let work = Work {
            state_evaluations: 42,
            searches: 1,
            passes: 2,
        };
        let timing = summarize(vec![80, 20, 40, 100], 2, work);
        assert_eq!(timing.elapsed_ns, [80, 20, 40, 100]);
        assert_eq!(timing.min_ns_per_invocation, 10.0);
        assert_eq!(timing.median_ns_per_invocation, 30.0);
        assert_eq!(timing.max_ns_per_invocation, 50.0);
        assert_eq!(timing.work_per_invocation, work);
        assert_eq!(summarize(vec![9], 1, work).median_ns_per_invocation, 9.0);
    }

    #[test]
    fn changing_work_counts_and_failed_workloads_are_errors() {
        let work = Work {
            state_evaluations: 1,
            searches: 0,
            passes: 0,
        };
        assert!(timed_batch(1, work, &mut || Ok(Work { passes: 1, ..work })).is_err());
        assert_eq!(
            timed_batch(1, work, &mut || Err("failure".into())),
            Err("failure".into())
        );
    }
}
