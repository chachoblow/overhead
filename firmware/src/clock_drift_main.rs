#![no_std]
#![no_main]
//! Opt-in S3 monotonic-clock drift capture, NOT the device application.
//! Prints `esp_hal::time::Instant` once per nominal second over UART0 so a
//! host with an NTP-disciplined clock can fit the rate. No network or radar.
//! The Sharp panel (same wiring as `sharp-test`) is held white with serial
//! VCOM alternating at ~1 Hz so a multi-hour run does not leave it unserviced.
use core::panic::PanicInfo;
use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    spi::{
        BitOrder, Mode,
        master::{Config, Spi},
    },
    time::{Duration, Instant, Rate},
};
use esp_println::println;
use overhead_s3_benchmark::sharp_test;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal::main]
fn main() -> ! {
    // Same clock configuration as the Sharp diagnostics; the measured rate
    // applies to this path only.
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::_240MHz));
    let mut cs = Output::new(peripherals.GPIO10, Level::Low, OutputConfig::default());
    let mut spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(2))
            .with_mode(Mode::_0)
            .with_write_bit_order(BitOrder::LsbFirst),
    )
    .expect("SPI configuration")
    .with_sck(peripherals.GPIO13)
    .with_mosi(peripherals.GPIO11);

    println!("OVERHEAD_S3_CLOCK_DRIFT_V1 instant_us per nominal second; host fits rate");
    let start = Instant::now();
    let mut seq: u64 = 0;
    loop {
        seq += 1;
        let target = start + Duration::from_secs(seq);
        while Instant::now() < target {
            core::hint::spin_loop();
        }
        // Read immediately before printing; the host fits these values, not
        // the nominal target, so loop latency does not bias the rate.
        let now = Instant::now().duration_since_epoch().as_micros();
        println!("DRIFT seq={seq} uptime_us={now}");
        cs.set_high();
        sharp_test::clear(seq % 2 != 0, |bytes| spi.write(bytes)).expect("Sharp VCOM");
        cs.set_low();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("OVERHEAD_S3_CLOCK_DRIFT_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
