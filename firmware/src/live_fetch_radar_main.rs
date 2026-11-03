#![no_std]
#![no_main]
//! Opt-in live-UTC radar diagnostic on fetched STATIONS elements (0074/0075):
//! the shared `live_radar` runtime with every telemetry line (0076). NOT the
//! device application; that is `device_main.rs` on the same runtime.
//!
//! With `live-fetch-faults` the same image follows a build-time fault plan
//! (`fault_plan`): chosen attempts fail at their real DNS, TCP or TLS stage,
//! `OLD DATA` and expiry thresholds are shortened, and Wi-Fi is dropped once,
//! so 0075's failure handling can be captured in one run. Without the feature
//! every attempt is ordinary and the thresholds are 0075's.
use core::panic::PanicInfo;
use embassy_executor::Spawner;
use esp_hal::clock::CpuClock;
use esp_println::println;
use overhead_s3_benchmark::live_radar::{self, Log};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal::main]
async fn main(spawner: Spawner) -> ! {
    let p = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::_240MHz));
    live_radar::run(
        spawner,
        p,
        Log::Diagnostic,
        "OVERHEAD_S3_LIVE_FETCH_RADAR_V3 fetched STATIONS; not production",
    )
    .await
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("OVERHEAD_S3_LIVE_FETCH_RADAR_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
