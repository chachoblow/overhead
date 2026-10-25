#![no_std]
#![no_main]
//! First fixed-time, one-ISS radar on physical Sharp panel, not standalone app.
//! Reuses the prepared target fixture and shared renderer; no Wi-Fi or controls.
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
use overhead_render::{CoastLayer, Layout};
use overhead_s3_benchmark::{coast_measurement::Screen, sharp_test};

esp_bootloader_esp_idf::esp_app_desc!();

// Keep the framebuffer and prepared layer out of the limited S3 stack.
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
    // Active-high Sharp SCS must be GPIO, not the SPI peripheral's active-low CS.
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
    // SAFETY: single-core, no tasks or concurrent access to the heap region.
    unsafe {
        ALLOCATOR.add_region(esp_alloc::HeapRegion::new(
            (&raw mut HEAP_STORAGE.0).cast::<u8>(),
            HEAP_BYTES,
            esp_alloc::MemoryCapability::Internal.into(),
        ));
    }
    println!("OVERHEAD_S3_SHARP_RADAR_V1 fixed fixture, no live clock/network");
    // Pinned real ISS OMM at its epoch; observer at its computed sub-satellite
    // point, so the marker is guaranteed to be visible. Wide 20-degree view.
    let app = overhead_s3_benchmark::coast_measurement::app("iss-ground-20", 0.0, 0.0, 20.0);
    let screen = unsafe { &mut *(&raw mut SCREEN) };
    let coast = unsafe { (*(&raw mut LAYER)).write(CoastLayer::new()) };
    coast.prepare_for(app.observer());
    assert!(coast.error().is_none(), "coast preparation failed");
    let render_start = Instant::now();
    overhead_render::draw(screen, &app, Layout::wide(), false, coast).unwrap();
    println!(
        "RADAR_READY selected={:?} ink_writes={} checksum={} render_us={}",
        app.selected_id(),
        screen.on_writes,
        screen.checksum(),
        render_start.elapsed().as_micros()
    );
    assert_eq!(app.selected_id(), Some(25544));
    assert!(screen.on_writes > 0);

    // Serial VCOM must continue to alternate even for an unchanged scene.
    cs.set_high();
    sharp_test::clear(true, |bytes| spi.write(bytes)).expect("Sharp clear");
    cs.set_low();
    let mut tick: u32 = 0;
    loop {
        let start = Instant::now();
        cs.set_high();
        sharp_test::frame_pixels(tick % 2 != 0, &screen.pixels, |bytes| spi.write(bytes))
            .expect("Sharp radar frame");
        cs.set_low();
        if tick % 4 == 0 {
            println!("RADAR_TRANSFER us={}", start.elapsed().as_micros());
        }
        tick = tick.wrapping_add(1);
        wait_until(start, Duration::from_millis(500));
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("OVERHEAD_S3_SHARP_RADAR_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
