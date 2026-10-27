#![no_std]
#![no_main]
//! Separate pinned-OMM live-UTC visibility diagnostic; no device fetch or age policy.
const BANNER: &str =
    "OVERHEAD_S3_LIVE_UTC_FRESH_ISS_V1 pinned 2026-10-26 elements; not fetched or production";

fn diagnostic_app() -> overhead_app::RadarApp {
    use overhead_s3_benchmark::coast_measurement::{LIVE_UTC_ISS_OBSERVER, app_from_omm};
    app_from_omm(
        include_str!("../fixtures/live-utc-iss-20261026.json"),
        LIVE_UTC_ISS_OBSERVER,
        5.0,
    )
}

include!("live_utc_fresh_program.rs");
