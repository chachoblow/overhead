#![no_std]
#![no_main]
//! Opt-in cached-vs-uncached 5° historical ISS replay. Not a live device app.
//! Only cached frames reach Sharp; the uncached byte oracle stays offscreen.
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
use overhead_s3_benchmark::{coast_measurement::Screen, sharp_cache, sharp_test};

esp_bootloader_esp_idf::esp_app_desc!();

static mut SCREEN: Screen = Screen::new();
static mut REFERENCE: Screen = Screen::new();
static mut LAYER: MaybeUninit<CoastLayer> = MaybeUninit::uninit();
static mut PERIOD: PeriodCache = PeriodCache::new();
const HEAP_BYTES: usize = 64 * 1024;
#[repr(C, align(16))]
struct HeapStorage(MaybeUninit<[u8; HEAP_BYTES]>);
static mut HEAP_STORAGE: HeapStorage = HeapStorage(MaybeUninit::uninit());
#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn wait_until(start: Instant, duration: Duration) {
    while start.elapsed() < duration {
        core::hint::spin_loop();
    }
}

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
        // SAFETY: single-core, sole owner of statics and allocator region.
        ALLOCATOR.add_region(esp_alloc::HeapRegion::new(
            (&raw mut HEAP_STORAGE.0).cast::<u8>(),
            HEAP_BYTES,
            esp_alloc::MemoryCapability::Internal.into(),
        ));
    }
    println!("OVERHEAD_S3_SHARP_CACHE_V1 5deg, 0..300s historical ISS, 2MHz SPI");
    let mut app = overhead_s3_benchmark::coast_measurement::app("iss-ground-20", 0.0, 0.0, 5.0);
    let epoch = app.time().expect("fixture epoch");
    let screen = unsafe { &mut *(&raw mut SCREEN) };
    let reference = unsafe { &mut *(&raw mut REFERENCE) };
    let period = unsafe { &mut *(&raw mut PERIOD) };
    let coast = unsafe { (*(&raw mut LAYER)).write(CoastLayer::new()) };
    coast.prepare_for(app.observer());
    assert!(coast.error().is_none(), "coast preparation failed");
    cs.set_high();
    sharp_test::clear(true, |bytes| spi.write(bytes)).expect("Sharp clear");
    cs.set_low();

    let mut tick = 0_u32;
    loop {
        let start = Instant::now();
        let seconds = sharp_test::replay_second(tick);
        app.update(epoch + TimeDelta::seconds(seconds))
            .expect("fixture replay UTC");
        let update_us = start.elapsed().as_micros();
        screen.reset_counts();
        overhead_render::draw_cached(screen, &app, Layout::wide(), false, coast, period).unwrap();
        let render_us = start.elapsed().as_micros() - update_us;
        let checksum = screen.checksum();
        cs.set_high();
        sharp_test::frame_pixels(tick % 2 != 0, &screen.pixels, |bytes| spi.write(bytes))
            .expect("Sharp cached frame");
        cs.set_low();
        let frame_us = start.elapsed().as_micros();
        println!(
            "CACHED_FRAME second={seconds} selected={:?} checksum={checksum} update_us={update_us} render_us={render_us} transfer_us={} total_us={frame_us}",
            app.selected_id(),
            frame_us - render_us - update_us,
        );
        // This is a separate full draw, not a cost to add to cached render.
        // The reference framebuffer is never transferred to Sharp.
        let control = Instant::now();
        assert!(
            sharp_cache::reference_matches(reference, screen, &app, coast),
            "uncached reference differs from cached frame"
        );
        println!(
            "REFERENCE parity=bytes_equal second={seconds} checksum={} render_us={}",
            reference.checksum(),
            control.elapsed().as_micros(),
        );
        tick = tick.wrapping_add(1);
        wait_until(start, Duration::from_millis(500));
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("OVERHEAD_S3_SHARP_CACHE_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
