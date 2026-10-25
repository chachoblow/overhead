#![no_std]
#![no_main]
//! Opt-in selective-line historical ISS diagnostic, NOT an adopted device loop.
//! 2 MHz maximum panel clock; 500 ms tick, one idle/VCOM step in eight.
use core::{mem::MaybeUninit, panic::PanicInfo};
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
use overhead_core::sgp4::chrono::TimeDelta;
use overhead_render::{CoastLayer, Layout, PeriodCache};
use overhead_s3_benchmark::{
    coast_measurement::Screen,
    sharp_lines::{self, LineSender, Step},
    sharp_test,
};

esp_bootloader_esp_idf::esp_app_desc!();
static mut SCREEN: Screen = Screen::new();
static mut SENDER: LineSender = LineSender::new();
static mut LAYER: MaybeUninit<CoastLayer> = MaybeUninit::uninit();
static mut PERIOD: PeriodCache = PeriodCache::new();
const HEAP_BYTES: usize = 64 * 1024;
#[repr(C, align(16))]
struct HeapStorage(MaybeUninit<[u8; HEAP_BYTES]>);
static mut HEAP_STORAGE: HeapStorage = HeapStorage(MaybeUninit::uninit());
#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

#[esp_hal::main]
fn main() -> ! {
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
    unsafe {
        // SAFETY: sole owner of statics, single core, no tasks or scheduler.
        ALLOCATOR.add_region(esp_alloc::HeapRegion::new(
            (&raw mut HEAP_STORAGE.0).cast::<u8>(),
            HEAP_BYTES,
            esp_alloc::MemoryCapability::Internal.into(),
        ));
    }
    println!("OVERHEAD_S3_SHARP_LINES_V1 5deg fixture, 2MHz, periodic full/idle VCOM");
    let mut app = overhead_s3_benchmark::coast_measurement::app("iss-ground-20", 0.0, 0.0, 5.0);
    let epoch = app.time().expect("fixture epoch");
    let screen = unsafe { &mut *(&raw mut SCREEN) };
    let sender = unsafe { &mut *(&raw mut SENDER) };
    let period = unsafe { &mut *(&raw mut PERIOD) };
    let coast = unsafe { (*(&raw mut LAYER)).write(CoastLayer::new()) };
    coast.prepare_for(app.observer());
    assert!(coast.error().is_none(), "coast preparation failed");
    cs.set_high(); // Sharp CS is active high
    let result = sharp_test::clear(true, |part| spi.write(part));
    cs.set_low();
    result.expect("Sharp clear");

    let mut tick = 0_u32;
    loop {
        let start = Instant::now();
        let phase = tick % 2 != 0; // alternate with initial clear and each 500 ms step
        match sharp_lines::step(tick) {
            Step::Idle => {
                cs.set_high();
                let result = sender.idle(phase, |part| spi.write(part));
                cs.set_low();
                let transfer = result.expect("Sharp idle VCOM");
                let elapsed_us = start.elapsed().as_micros();
                println!(
                    "LINES tick={tick} kind={:?} second=300 checksum={} rows={} bytes={} transfer_us={elapsed_us} total_us={elapsed_us}",
                    transfer.kind,
                    screen.checksum(),
                    transfer.rows,
                    transfer.bytes,
                );
            }
            Step::Frame { second, force_full } => {
                app.update(epoch + TimeDelta::seconds(second))
                    .expect("fixture replay UTC");
                let update_us = start.elapsed().as_micros();
                screen.reset_counts();
                overhead_render::draw_cached(screen, &app, Layout::wide(), false, coast, period)
                    .unwrap();
                let render_us = start.elapsed().as_micros() - update_us;
                let checksum = screen.checksum();
                cs.set_high();
                let result = sender.send(phase, &screen.pixels, force_full, |part| spi.write(part));
                cs.set_low();
                let transfer = result.expect("Sharp frame transfer");
                let total_us = start.elapsed().as_micros();
                println!(
                    "LINES tick={tick} kind={:?} second={second} selected={:?} checksum={checksum} rows={} bytes={} update_us={update_us} render_us={render_us} transfer_us={} total_us={total_us}",
                    transfer.kind,
                    app.selected_id(),
                    transfer.rows,
                    transfer.bytes,
                    total_us - update_us - render_us
                );
            }
        }
        tick = tick.wrapping_add(1);
        while start.elapsed() < Duration::from_millis(500) {
            core::hint::spin_loop();
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("OVERHEAD_S3_SHARP_LINES_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
