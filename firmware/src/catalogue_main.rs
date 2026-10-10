#![no_std]
#![no_main]

use core::{hint::black_box, mem::MaybeUninit, panic::PanicInfo};
use esp_hal::{
    clock::CpuClock,
    time::{Duration, Instant},
};
use esp_println::println;
use overhead_s3_benchmark::{
    catalogue_memory::{self as experiment, Case, Work},
    heap_meter::{Meter, Phase},
    stack_watermark::{self, Watermark},
    suites::sample_count,
};

esp_bootloader_esp_idf::esp_app_desc!();

const HEAP_BYTES: usize = 64 * 1024;
#[repr(C, align(16))]
struct HeapStorage(MaybeUninit<[u8; HEAP_BYTES]>);
static mut HEAP_STORAGE: HeapStorage = HeapStorage(MaybeUninit::uninit());
#[global_allocator]
static ALLOCATOR: Meter<esp_alloc::EspHeap> = Meter::new(esp_alloc::EspHeap::empty());

fn pause() {
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(20) {
        core::hint::spin_loop();
    }
}

#[derive(Clone, Copy)]
struct Sample {
    initialization: Phase,
    aggregation: Phase,
    init_us: u64,
    agg_us: u64,
    work: Work,
    stack: Watermark,
}

fn phase<T>(run: impl FnOnce() -> T) -> (T, Phase, u64) {
    let before = ALLOCATOR.begin();
    let start = Instant::now();
    let value = black_box(run());
    let elapsed = start.elapsed().as_micros();
    let memory = ALLOCATOR.finish(before);
    assert_eq!(memory.failures, 0);
    (value, memory, elapsed)
}

#[inline(never)]
fn sample(case: Case) -> Sample {
    let config = case.config();
    let baseline = ALLOCATOR.snapshot();
    assert_eq!(baseline.requested_live, 0);
    // SAFETY: HAL main runs on CPU0's linker stack. This binary starts no
    // tasks/second core, switches no stacks, and scans before the next paint.
    let painted = unsafe { stack_watermark::paint() };
    let (catalogue, initialization, init_us) = phase(|| experiment::build(black_box(case.json())));
    let catalogue = catalogue.expect("catalogue initialization");
    experiment::validate_catalogue(&catalogue, case);
    let ((result, candidates), aggregation, agg_us) = phase(|| {
        experiment::aggregate(
            black_box(&catalogue),
            black_box(&config),
            black_box(case.allowance),
        )
    });
    let work = experiment::work(&result, &candidates);
    assert_eq!(work.complete, case.allowance > 1000);
    drop(candidates);
    drop(result);
    drop(catalogue);
    let released = ALLOCATOR.snapshot();
    assert_eq!(released.requested_live, baseline.requested_live);
    assert_eq!(released.occupied_live, baseline.occupied_live);
    // SAFETY: same CPU0 stack ownership as at paint; all work has returned.
    let stack = unsafe { painted.scan() };
    Sample {
        initialization,
        aggregation,
        init_us,
        agg_us,
        work,
        stack,
    }
}

fn print_phase(id: usize, sample: usize, name: &str, us: u64, phase: Phase) {
    println!(
        "PHASE,{id},{sample},{name},{us},{},{},{},{},{},{},{}",
        phase.requested_peak,
        phase.requested_live,
        phase.occupied_peak,
        phase.occupied_live,
        phase.calls,
        phase.requested,
        phase.failures
    );
}

#[esp_hal::main]
fn main() -> ! {
    let _peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::_240MHz));
    // SAFETY: one initialization, static aligned storage exclusively owned by
    // this heap thereafter. No allocator, task, or second CPU uses it earlier.
    unsafe {
        ALLOCATOR.backend.add_region(esp_alloc::HeapRegion::new(
            (&raw mut HEAP_STORAGE).cast::<u8>(),
            HEAP_BYTES,
            esp_alloc::MemoryCapability::Internal.into(),
        ));
    }
    pause();
    let samples = sample_count(env!("BENCH_SAMPLES")).expect("invalid BENCH_SAMPLES");
    println!("OVERHEAD_S3_CATALOGUE_MEMORY_V1");
    println!("MATRIX,{samples},{}", experiment::CASE_COUNT);
    println!("compiler={}", env!("BENCH_RUSTC"));
    println!("cpu_mhz=240,cores_used=1,psram=unused,allocator=esp-alloc-0.11.0-LLFF,math=f64_libm");
    println!(
        "input=flash_omm,start=2006-06-26T00:00:00Z,observer=39.007/-104.883/2.187,threshold_deg=10,detection_s=60,tolerance_s=5"
    );
    println!("warmups=1,iterations_per_sample=1,stack=written_watermark_including_probe_and_drop");
    println!(
        "HEAP,{HEAP_BYTES},{},{}",
        ALLOCATOR.backend.free(),
        ALLOCATOR.backend.used()
    );
    // SAFETY: same single-owner CPU0 linker stack as `sample`; no tasks or
    // second core are started. Check the probe before any catalogue measurement.
    let (baseline, loaded) = unsafe { stack_watermark::self_check() };
    println!(
        "PROBE,{},{}",
        baseline.observed_bytes, loaded.observed_bytes
    );
    println!(
        "phase_fields=id,sample,phase,us,requested_peak,requested_live,occupied_peak,occupied_live,calls,requested,failures"
    );
    println!("work_fields=id,sample,evaluations,passes,candidates,searched,complete");
    println!(
        "stack_fields=id,sample,stack_low,stack_top,paint_low,paint_high,untouched_bytes,observed_bytes"
    );
    for id in 0..experiment::CASE_COUNT {
        let case = Case::at(id).unwrap();
        println!(
            "CASE,{id},{},{},{},{}",
            case.size,
            case.window_s,
            case.allowance,
            case.json().len()
        );
        println!("BEGIN,{id}");
        pause();
        let warmup = sample(case);
        for index in 0..samples {
            let current = sample(case);
            assert_eq!(current.work, warmup.work);
            assert_eq!(current.initialization, warmup.initialization);
            assert_eq!(current.aggregation, warmup.aggregation);
            print_phase(id, index, "init", current.init_us, current.initialization);
            print_phase(id, index, "aggregate", current.agg_us, current.aggregation);
            let w = current.work;
            println!(
                "WORK,{id},{index},{},{},{},{},{}",
                w.evaluations,
                w.passes,
                w.candidates,
                w.searched,
                u8::from(w.complete)
            );
            let s = current.stack;
            println!(
                "STACK,{id},{index},{},{},{},{},{},{}",
                s.stack_low,
                s.stack_top,
                s.paint_low,
                s.paint_high,
                s.untouched_bytes,
                s.observed_bytes
            );
            pause(); // UART is outside phases and the per-sample painted lifetime.
        }
    }
    println!("OVERHEAD_S3_BENCHMARK_DONE");
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("OVERHEAD_S3_BENCHMARK_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
