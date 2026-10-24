#![no_std]
#![no_main]
//! Prepared/cached coastline layer measurement image (0058/0059), NOT the
//! device app. No Sharp transfer, PSRAM, scheduling or frame cadence.
use core::{hint::black_box, mem::MaybeUninit, panic::PanicInfo};
use esp_hal::{
    clock::CpuClock,
    time::{Duration, Instant},
};
use esp_println::println;
use overhead_render::CoastLayer;
use overhead_s3_benchmark::{
    coast_measurement::{self, CASES, SAMPLES, Screen},
    heap_meter::Meter,
    prepared_measurement::{self, MODES, Mode},
    stack_watermark,
};

esp_bootloader_esp_idf::esp_app_desc!();

const HEAP_BYTES: usize = 64 * 1024;
#[repr(C, align(16))]
struct HeapStorage(MaybeUninit<[u8; HEAP_BYTES]>);
static mut HEAP_STORAGE: HeapStorage = HeapStorage(MaybeUninit::uninit());
static mut SCREEN: Screen = Screen::new();
// Measurement-only 12 KB candidate reference for untimed exactness checks.
static mut REFERENCE: Screen = Screen::new();
// Long-lived layer storage, as 0059 requires (not a transient stack frame).
// Uninitialized (.bss, not an 11 KB .data image); written once before use.
static mut LAYER: MaybeUninit<Option<CoastLayer>> = MaybeUninit::uninit();
#[global_allocator]
static ALLOCATOR: Meter<esp_alloc::EspHeap> = Meter::new(esp_alloc::EspHeap::empty());

fn pause() {
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(20) {
        core::hint::spin_loop();
    }
}

fn prepare_target(screen: &mut Screen, mode: Mode) {
    if matches!(mode, Mode::CoastRaster | Mode::CoastCached) {
        screen.clear_for_coast();
    } else {
        screen.reset_counts();
    }
}

#[inline(never)]
fn sample(
    screen: &mut Screen,
    app: &overhead_app::RadarApp,
    layer: &mut Option<CoastLayer>,
    mode: Mode,
) -> (u64, u32, u32, u32, usize) {
    prepare_target(screen, mode);
    let before = ALLOCATOR.begin();
    // SAFETY: single-core image with no tasks, second core or stack switching.
    let painted = unsafe { stack_watermark::paint() };
    let start = Instant::now();
    prepared_measurement::run(black_box(screen), black_box(app), black_box(layer), mode);
    let us = start.elapsed().as_micros();
    let stack = unsafe { painted.scan() };
    let heap = ALLOCATOR.finish(before);
    assert_eq!(heap.calls, 0, "draw allocated");
    assert_eq!(heap.failures, 0);
    assert_eq!(heap.requested, 0);
    assert_eq!(heap.requested_live, before.requested_live);
    assert_eq!(heap.occupied_live, before.occupied_live);
    (
        us,
        black_box(screen.checksum()),
        screen.writes,
        screen.on_writes,
        stack.observed_bytes,
    )
}

#[esp_hal::main]
fn main() -> ! {
    let _ = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::_240MHz));
    // SAFETY: exclusive static region, initialized once before fixture parsing.
    unsafe {
        ALLOCATOR.backend.add_region(esp_alloc::HeapRegion::new(
            (&raw mut HEAP_STORAGE.0).cast::<u8>(),
            HEAP_BYTES,
            esp_alloc::MemoryCapability::Internal.into(),
        ));
    }
    // SAFETY: sole owners of these statics in this single-core image.
    let screen = unsafe { &mut *(&raw mut SCREEN) };
    let reference = unsafe { &mut *(&raw mut REFERENCE) };
    let layer = unsafe { (*(&raw mut LAYER)).write(None) };
    pause();
    println!("OVERHEAD_S3_COAST_PREPARED_V1");
    println!(
        "cpu_mhz=240,buffer_bytes=12000,heap_reserved={HEAP_BYTES},samples={SAMPLES},layer_bytes={}",
        core::mem::size_of::<Option<CoastLayer>>()
    );
    println!("NOT a Sharp/frame-cadence measurement; stack is observed writes only");
    println!("Every ROW asserts zero allocation calls/failures/requested/live growth");
    // SAFETY: same exclusive CPU0 stack ownership as sample().
    let (baseline, loaded) = unsafe { stack_watermark::self_check() };
    println!(
        "PROBE,baseline={},loaded={},stack_low={},stack_top={}",
        baseline.observed_bytes, loaded.observed_bytes, baseline.stack_low, baseline.stack_top
    );
    for (name, lat, lon, fov) in CASES {
        let app = coast_measurement::app(name, lat, lon, fov);
        let site = app.observer();
        println!(
            "CASE,{name},{:.6},{:.6},{fov},selected={}",
            site.latitude_rad.to_degrees(),
            site.longitude_rad.to_degrees(),
            app.selected_id().is_some()
        );
        *layer = None;
        for mode in MODES {
            // Untimed warm-up; leaves a prepared layer and valid cache behind.
            prepare_target(screen, mode);
            prepared_measurement::run(screen, &app, layer, mode);
            pause();
            let mut samples = [(0_u64, 0_u32, 0_u32, 0_u32, 0_usize); SAMPLES];
            for result in &mut samples {
                *result = sample(screen, &app, layer, mode);
            }
            for (index, (us, checksum, writes, on_writes, stack)) in samples.into_iter().enumerate()
            {
                println!(
                    "ROW,{name},{mode:?},{index},{us},{checksum},{writes},{on_writes},{stack}"
                );
            }
            prepared_measurement::verify(screen, reference, &app, mode);
            println!("EXACT,{name},{mode:?}");
        }
    }
    println!("OVERHEAD_S3_COAST_PREPARED_DONE");
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("OVERHEAD_S3_COAST_PREPARED_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
