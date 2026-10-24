#![no_std]
#![no_main]
//! Single authorized representation feasibility image, NOT the device app.
use core::{hint::black_box, mem::MaybeUninit, panic::PanicInfo};
use esp_hal::{
    clock::CpuClock,
    time::{Duration, Instant},
};
use esp_println::println;
use overhead_s3_benchmark::{
    coast_measurement::{self, CASES, Mode, SAMPLES, Screen},
    heap_meter::Meter,
    representation_measurement, stack_watermark,
};

esp_bootloader_esp_idf::esp_app_desc!();

const HEAP_BYTES: usize = 64 * 1024;
#[repr(C, align(16))]
struct HeapStorage(MaybeUninit<[u8; HEAP_BYTES]>);
static mut HEAP_STORAGE: HeapStorage = HeapStorage(MaybeUninit::uninit());
static mut SCREEN: Screen = Screen::new();
// Measurement-only 12 KB pixel reference, present in both comparison images.
// Neither screen is coast-attributable scratch or a Sharp driver buffer.
static mut REFERENCE: Screen = Screen::new();
#[global_allocator]
static ALLOCATOR: Meter<esp_alloc::EspHeap> = Meter::new(esp_alloc::EspHeap::empty());

#[cfg(feature = "coast-representation-on")]
fn invoke(screen: &mut Screen, app: &overhead_app::RadarApp, mode: Mode) -> bool {
    representation_measurement::run(screen, app, mode)
}

#[cfg(not(feature = "coast-representation-on"))]
fn invoke(screen: &mut Screen, app: &overhead_app::RadarApp, mode: Mode) -> bool {
    assert_eq!(mode, Mode::FrameOff);
    coast_measurement::run_off(screen, app);
    true
}

fn pause() {
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(20) {
        core::hint::spin_loop();
    }
}

#[inline(never)]
fn sample(
    screen: &mut Screen,
    app: &overhead_app::RadarApp,
    mode: Mode,
) -> (u64, u32, u32, u32, usize) {
    if mode == Mode::Coast {
        screen.clear_for_coast();
    } else {
        screen.reset_counts();
    }
    let before = ALLOCATOR.begin();
    // SAFETY: this single-core image starts no task/second core, installs no
    // asynchronous stack owner, and switches no stacks. Paint/scan stay paired.
    let painted = unsafe { stack_watermark::paint() };
    let start = Instant::now();
    assert!(black_box(invoke(
        black_box(screen),
        black_box(app),
        black_box(mode)
    )));
    let us = start.elapsed().as_micros();
    let stack = unsafe { painted.scan() };
    let heap = ALLOCATOR.finish(before);
    assert_eq!(heap.calls, 0, "draw allocated");
    assert_eq!(heap.failures, 0);
    assert_eq!(heap.requested, 0);
    assert_eq!(heap.requested_peak, before.requested_live);
    assert_eq!(heap.requested_live, before.requested_live);
    assert_eq!(heap.occupied_peak, before.occupied_live);
    assert_eq!(heap.occupied_live, before.occupied_live);
    (
        us,
        black_box(screen.checksum()),
        screen.writes,
        screen.on_writes,
        stack.observed_bytes,
    )
}

// Untimed full 1-bit output, retained independently of checksums. No allocation.
fn dump(name: &str, mode: Mode, screen: &Screen) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for (y, row) in screen.pixels.chunks_exact(50).enumerate() {
        let mut text = [0_u8; 100];
        for (i, &byte) in row.iter().enumerate() {
            text[2 * i] = HEX[(byte >> 4) as usize];
            text[2 * i + 1] = HEX[(byte & 15) as usize];
        }
        println!(
            "MASK,{name},{mode:?},{y},{}",
            core::str::from_utf8(&text).unwrap()
        );
    }
}

#[esp_hal::main]
fn main() -> ! {
    let _ = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::_240MHz));
    // SAFETY: exclusive static region, initialized once before app fixture parsing.
    unsafe {
        ALLOCATOR.backend.add_region(esp_alloc::HeapRegion::new(
            (&raw mut HEAP_STORAGE.0).cast::<u8>(),
            HEAP_BYTES,
            esp_alloc::MemoryCapability::Internal.into(),
        ));
    }
    // SAFETY: no other owner of the static RAM target in this single-core image.
    let screen = unsafe { &mut *(&raw mut SCREEN) };
    // SAFETY: distinct static storage, exclusively owned by this image.
    let reference = unsafe { &mut *(&raw mut REFERENCE) };
    pause();
    println!("OVERHEAD_S3_COAST_REPRESENTATION_V1");
    println!(
        "coast_on={},cpu_mhz=240,buffer_bytes=12000,heap_reserved={},samples={SAMPLES}",
        cfg!(feature = "coast-representation-on"),
        HEAP_BYTES
    );
    println!("NOT a Sharp/frame-cadence measurement; stack is observed writes only");
    println!(
        "pending_bytes={}",
        overhead_render::measurement::pending_bytes()
    );
    println!("Every ROW asserts zero allocation calls/failures/requested/live/peak growth");
    println!(
        "heap_free={},heap_used={}",
        ALLOCATOR.backend.free(),
        ALLOCATOR.backend.used()
    );
    // SAFETY: same exclusive CPU0 stack ownership as sample(), after HAL init.
    // The smoke check cannot prove a safe stack bound.
    let (baseline, loaded) = unsafe { stack_watermark::self_check() };
    println!(
        "PROBE,baseline={},loaded={},stack_low={},stack_top={},paint_low={},paint_high={}",
        baseline.observed_bytes,
        loaded.observed_bytes,
        baseline.stack_low,
        baseline.stack_top,
        baseline.paint_low,
        baseline.paint_high
    );
    for (name, lat, lon, fov) in CASES {
        let app = coast_measurement::app(name, lat, lon, fov);
        let modes =
            representation_measurement::modes(name, cfg!(feature = "coast-representation-on"));
        // Same untimed no-coast control in both images, retaining the reference
        // buffer and a comparable baseline. Late controls overwrite it later.
        coast_measurement::run_off(reference, &app);
        println!("OFF_CONTROL,{name},{}", reference.checksum());
        let site = app.observer();
        println!(
            "CASE,{name},{:.6},{:.6},{fov},selected={}",
            site.latitude_rad.to_degrees(),
            site.longitude_rad.to_degrees(),
            app.selected_id().is_some()
        );
        for &mode in modes {
            // Warm-up outside timed samples; coast-only begins from a blank target.
            if mode == Mode::Coast {
                screen.clear_for_coast();
            }
            assert!(invoke(screen, &app, mode));
            pause();
            let mut samples = [(0_u64, 0_u32, 0_u32, 0_u32, 0_usize); SAMPLES];
            for result in &mut samples {
                *result = sample(screen, &app, mode);
            }
            // Report after all invocations; UART does not interleave samples.
            for (index, (us, checksum, writes, on_writes, stack)) in samples.into_iter().enumerate()
            {
                println!(
                    "ROW,{name},{mode:?},{index},{us},{checksum},{writes},{on_writes},{stack}"
                );
            }
            dump(name, mode, screen);
            #[cfg(feature = "coast-representation-on")]
            if matches!(mode, Mode::Late | Mode::LateWarning) {
                let warning = mode == Mode::LateWarning;
                representation_measurement::verify_late(screen, reference, &app, warning);
                println!(
                    "PIXEL_CONTROL,{name},{mode:?},matched,{}",
                    screen.checksum()
                );
            }
        }
    }
    println!("OVERHEAD_S3_COAST_REPRESENTATION_DONE");
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("OVERHEAD_S3_COAST_REPRESENTATION_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
