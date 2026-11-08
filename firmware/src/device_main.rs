#![no_std]
#![no_main]
//! Overhead device application (0076): the live-UTC radar at the build-time
//! home site on fetched `visual` elements (0077), refreshed under 0075, with BOOT
//! cycling (0068/0070). Same runtime as the `live-fetch-radar` diagnostic,
//! printing only the event log. On a panic it prints the reason and resets
//! (0086): a new image on probation then rolls back, and stored elements
//! (0079) keep a reset from costing a CelesTrak request.
#[cfg(feature = "live-fetch-faults")]
compile_error!("the device app never carries the fault plan; build the diagnostic instead");

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
        Log::Events,
        "OVERHEAD_DEVICE_V2 live radar on fetched visual group",
    )
    .await
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("OVERHEAD_DEVICE_FAILED: {info}; resetting");
    // Let the UART drain the line.
    let t = esp_hal::time::Instant::now();
    while t.elapsed() < esp_hal::time::Duration::from_millis(50) {}
    esp_hal::system::software_reset()
}
