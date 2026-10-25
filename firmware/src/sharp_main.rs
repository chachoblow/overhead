#![no_std]
#![no_main]
//! Isolated Sharp #4694 wiring/refresh check, NOT the device application.
//! GND->G, VIN->5V, CLK->IO13, DI->IO11, CS->IO10.
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

fn wait_until(start: Instant, duration: Duration) {
    while start.elapsed() < duration {
        core::hint::spin_loop();
    }
}

#[esp_hal::main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::_240MHz));
    // The Sharp SCS is active HIGH. Do not use the SPI peripheral's active-low CS.
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

    println!("OVERHEAD_S3_SHARP_TEST_V1 IO13=CLK IO11=DI IO10=CS");
    // Initialization: clear panel memory white; alternate VCOM for every later
    // full refresh. The breakout straps EXTMODE for serial VCOM inversion.
    cs.set_high();
    sharp_test::clear(true, |bytes| spi.write(bytes)).expect("Sharp clear");
    cs.set_low();
    let mut tick: u32 = 0;
    loop {
        let start = Instant::now();
        let phase = (tick / 4) % 2 != 0;
        cs.set_high();
        sharp_test::frame(tick % 2 != 0, phase, |bytes| spi.write(bytes)).expect("Sharp frame");
        cs.set_low();
        if tick % 4 == 0 {
            println!(
                "SHARP_FRAME phase={} transfer_us={}",
                phase as u8,
                start.elapsed().as_micros()
            );
        }
        tick = tick.wrapping_add(1);
        wait_until(start, Duration::from_millis(500));
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("OVERHEAD_S3_SHARP_TEST_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
