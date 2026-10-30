#![no_std]
#![no_main]
//! Opt-in historical multi-OMM BOOT cycle experiment, not a live device app.
//! Fixed UTC/observer, no Wi-Fi, knob or data refresh; capture in README.
use core::{mem::MaybeUninit, panic::PanicInfo};
use esp_hal::{
    clock::CpuClock,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    spi::{
        BitOrder, Mode,
        master::{Config, Spi},
    },
    time::{Duration, Instant, Rate},
};
use esp_println::println;
use overhead_render::{CoastLayer, Layout};
use overhead_s3_benchmark::{
    button_input::CompletedPress,
    coast_measurement::{self, Screen},
    sharp_test,
};

esp_bootloader_esp_idf::esp_app_desc!();

// Keep large buffers out of the small S3 stack; one task owns all statics.
static mut SCREEN: Screen = Screen::new();
static mut LAYER: MaybeUninit<CoastLayer> = MaybeUninit::uninit();
const HEAP_BYTES: usize = 96 * 1024;
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
    let p = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::_240MHz));
    let mut cs = Output::new(p.GPIO10, Level::Low, OutputConfig::default());
    let mut spi = Spi::new(
        p.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(2))
            .with_mode(Mode::_0)
            .with_write_bit_order(BitOrder::LsbFirst),
    )
    .expect("SPI configuration")
    .with_sck(p.GPIO13)
    .with_mosi(p.GPIO11);
    // GPIO0 is a boot strap; a startup hold is not counted as a press.
    let boot = Input::new(p.GPIO0, InputConfig::default().with_pull(Pull::Up));
    unsafe {
        ALLOCATOR.add_region(esp_alloc::HeapRegion::new(
            (&raw mut HEAP_STORAGE.0).cast::<u8>(),
            HEAP_BYTES,
            esp_alloc::MemoryCapability::Internal.into(),
        ));
    }
    let mut app = coast_measurement::boot_cycle_app();
    let screen = unsafe { &mut *(&raw mut SCREEN) };
    let coast = unsafe { (*(&raw mut LAYER)).write(CoastLayer::new()) };
    coast.prepare_for(app.observer());
    assert!(coast.error().is_none(), "coast preparation failed");
    println!(
        "BOOT_CYCLE_READY historical UTC=2026-10-10T00:00:00Z visible={} selected={:?}",
        app.counts().visible,
        app.selected_id()
    );
    let mut input = CompletedPress::new(20_000).unwrap();
    let mut tick = 0u32;
    let mut last_transfer = Instant::now();
    let mut redraw = true;
    loop {
        let start = Instant::now();
        if input.poll_selection(
            start.duration_since_epoch().as_micros(),
            boot.is_low(),
            &mut app,
        ) {
            println!("BOOT_CYCLE selected={:?}", app.selected_id());
            redraw = true;
        }
        // Full frames on changes or every 500 ms for serial VCOM. Blocking
        // render/SPI can miss short presses; this is not a latency guarantee.
        if redraw || last_transfer.elapsed() >= Duration::from_millis(500) {
            if redraw {
                overhead_render::draw(screen, &app, Layout::wide(), false, coast)
                    .expect("historical radar render");
            }
            cs.set_high();
            let result =
                sharp_test::frame_pixels(tick % 2 != 0, &screen.pixels, |bytes| spi.write(bytes));
            cs.set_low();
            result.expect("Sharp historical frame");
            tick = tick.wrapping_add(1);
            last_transfer = Instant::now();
            redraw = false;
        }
        wait_until(start, Duration::from_millis(5));
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("BOOT_CYCLE_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
