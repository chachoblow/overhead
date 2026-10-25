#![no_std]
#![no_main]
//! Deliberately accelerated ISS replay on physical Sharp, NOT live tracking.
//! Dedicated opt-in image keeps the visually accepted static radar unchanged.
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
use overhead_render::{CoastLayer, Layout};
use overhead_s3_benchmark::{coast_measurement::Screen, sharp_test};

esp_bootloader_esp_idf::esp_app_desc!();

// The 12 KB framebuffer and prepared coastline stay in static internal RAM.
static mut SCREEN: Screen = Screen::new();
static mut LAYER: MaybeUninit<CoastLayer> = MaybeUninit::uninit();
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
    // Sharp CS is active high; SPI peripheral CS is active low, so use GPIO.
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
    // SAFETY: sole owner of the statics, single core, no scheduler or tasks.
    unsafe {
        ALLOCATOR.add_region(esp_alloc::HeapRegion::new(
            (&raw mut HEAP_STORAGE.0).cast::<u8>(),
            HEAP_BYTES,
            esp_alloc::MemoryCapability::Internal.into(),
        ));
    }
    println!("OVERHEAD_S3_SHARP_REPLAY_V2 fixed ISS fixture, 0..300 seconds loop, 5 deg FOV");
    // Name selects the fixture's computed ISS ground point; FOV is 5°, not 20°.
    let mut app = overhead_s3_benchmark::coast_measurement::app("iss-ground-20", 0.0, 0.0, 5.0);
    let epoch = app.time().expect("fixture epoch");
    let screen = unsafe { &mut *(&raw mut SCREEN) };
    let coast = unsafe { (*(&raw mut LAYER)).write(CoastLayer::new()) };
    coast.prepare_for(app.observer());
    assert!(coast.error().is_none(), "coast preparation failed");
    // Initial clear inverts VCOM relative to the first frame below.
    cs.set_high();
    sharp_test::clear(true, |bytes| spi.write(bytes)).expect("Sharp clear");
    cs.set_low();
    let mut tick: u32 = 0;
    loop {
        let start = Instant::now();
        let seconds = sharp_test::replay_second(tick);
        app.update(epoch + TimeDelta::seconds(seconds))
            .expect("fixture replay UTC");
        let update_us = start.elapsed().as_micros();
        // Shared renderer clears old ink; reuse the prepared coastline/cache.
        screen.reset_counts();
        overhead_render::draw(screen, &app, Layout::wide(), false, coast).unwrap();
        let render_us = start.elapsed().as_micros() - update_us;
        cs.set_high();
        sharp_test::frame_pixels(tick % 2 != 0, &screen.pixels, |bytes| spi.write(bytes))
            .expect("Sharp replay frame");
        cs.set_low();
        let total_us = start.elapsed().as_micros();
        // Report every frame: these are measured stage times, not proof of
        // whole-device 20 Hz, latency guarantees or stack safety.
        println!(
            "REPLAY second={} selected={:?} checksum={} update_us={} render_us={} transfer_us={} total_us={}",
            seconds,
            app.selected_id(),
            screen.checksum(),
            update_us,
            render_us,
            total_us - update_us - render_us,
            total_us
        );
        tick = tick.wrapping_add(1);
        wait_until(start, Duration::from_millis(500));
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("OVERHEAD_S3_SHARP_REPLAY_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
