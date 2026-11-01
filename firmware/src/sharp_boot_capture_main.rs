#![no_std]
#![no_main]
//! Opt-in 0070 BOOT capture diagnostic on the historical multi-OMM scene.
//! A 5 ms TIMG0 interrupt samples GPIO0 into `PressLatch` regardless of
//! render/SPI; the display loop drains completed presses and applies 0068.
//! Fixed UTC/observer, no Wi-Fi, knob or data refresh; capture in README.
use core::{cell::RefCell, mem::MaybeUninit, panic::PanicInfo};
use critical_section::Mutex;
use esp_hal::{
    Blocking,
    clock::CpuClock,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    handler,
    spi::{
        BitOrder, Mode,
        master::{Config, Spi},
    },
    time::{Duration, Instant, Rate},
    timer::{PeriodicTimer, timg::TimerGroup},
};
use esp_println::println;
use overhead_render::{CoastLayer, Layout};
use overhead_s3_benchmark::{
    button_input::{PressLatch, apply_presses},
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

/// Provisional 0068 interval and the 0070 sampling period.
const DEBOUNCE_US: u64 = 20_000;
const SAMPLE_PERIOD: Duration = Duration::from_millis(5);
const STATS_EVERY: Duration = Duration::from_secs(10);

struct Capture {
    timer: PeriodicTimer<'static, Blocking>,
    boot: Input<'static>,
    latch: PressLatch,
}

// Shared only with `sample_boot`; the display loop holds it just long enough
// to take counts. It never touches the app, renderer or panel.
static CAPTURE: Mutex<RefCell<Option<Capture>>> = Mutex::new(RefCell::new(None));

#[handler]
fn sample_boot() {
    critical_section::with(|cs| {
        if let Some(capture) = CAPTURE.borrow_ref_mut(cs).as_mut() {
            capture.timer.clear_interrupt();
            let now = Instant::now().duration_since_epoch().as_micros();
            capture.latch.sample(now, capture.boot.is_low());
        }
    });
}

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

    // GPIO0 is a boot strap; a startup hold is not counted as a press.
    // TIMG0 is free here: this image has no esp-rtos or Wi-Fi.
    let boot = Input::new(p.GPIO0, InputConfig::default().with_pull(Pull::Up));
    let timg0 = TimerGroup::new(p.TIMG0);
    let mut timer = PeriodicTimer::new(timg0.timer0);
    timer.set_interrupt_handler(sample_boot);
    critical_section::with(|cs| {
        timer.start(SAMPLE_PERIOD).expect("5 ms sample timer");
        timer.listen();
        CAPTURE.borrow_ref_mut(cs).replace(Capture {
            timer,
            boot,
            latch: PressLatch::new(DEBOUNCE_US).unwrap(),
        });
    });
    println!(
        "BOOT_CAPTURE_READY historical UTC=2026-10-10T00:00:00Z sample_ms=5 visible={} selected={:?}",
        app.counts().visible,
        app.selected_id()
    );
    let mut tick = 0u32;
    let mut last_transfer = Instant::now();
    let mut last_stats = Instant::now();
    let mut redraw = true;
    loop {
        let start = Instant::now();
        let presses = critical_section::with(|cs| {
            CAPTURE
                .borrow_ref_mut(cs)
                .as_mut()
                .map_or(0, |capture| capture.latch.take_presses())
        });
        if presses > 0 {
            let applied = apply_presses(presses, &mut app);
            println!(
                "BOOT_CAPTURE drained={presses} applied={applied} selected={:?}",
                app.selected_id()
            );
            redraw |= applied > 0;
        }
        if last_stats.elapsed() >= STATS_EVERY {
            let stats = critical_section::with(|cs| {
                CAPTURE
                    .borrow_ref_mut(cs)
                    .as_mut()
                    .map(|capture| capture.latch.take_stats())
            })
            .unwrap_or_default();
            // Expect ~2000 samples per 10 s; max_gap_us is ISR jitter evidence
            // (UART println holds a critical section and can delay a sample).
            println!(
                "BOOT_CAPTURE_STATS samples={} max_gap_us={}",
                stats.samples, stats.max_gap_us
            );
            last_stats = Instant::now();
        }
        // Full frames on changes or every 500 ms for serial VCOM. Blocking
        // render/SPI no longer hides presses; they paint on the next frame.
        if redraw || last_transfer.elapsed() >= Duration::from_millis(500) {
            if redraw {
                overhead_render::draw(screen, &app, Layout::wide(), false, coast)
                    .expect("historical radar render");
            }
            cs.set_high();
            let result =
                sharp_test::frame_pixels(!tick.is_multiple_of(2), &screen.pixels, |bytes| {
                    spi.write(bytes)
                });
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
    println!("BOOT_CAPTURE_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
