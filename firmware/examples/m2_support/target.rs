//! Single CPU/no task harness. Formatting, checks and UART are outside timers.
use core::{
    fmt::{self, Write},
    hint::black_box,
    mem::MaybeUninit,
    panic::PanicInfo,
};
use esp_hal::{clock::CpuClock, time::Instant};
use esp_println::{print, println};
use overhead_s3_benchmark::{
    heap_meter::{Meter, Phase},
    m2::{self, Case, Lifetime, Observations},
    stack_watermark::{self, Watermark},
};

esp_bootloader_esp_idf::esp_app_desc!();
const HEAP_BYTES: usize = 65536;
#[repr(C, align(16))]
struct HeapStorage(MaybeUninit<[u8; HEAP_BYTES]>);
static mut HEAP_STORAGE: HeapStorage = HeapStorage(MaybeUninit::uninit());
#[global_allocator]
static ALLOCATOR: Meter<esp_alloc::EspHeap> = Meter::new(esp_alloc::EspHeap::empty());
const NAMES: [&str; 5] = [
    "input-copy",
    "initialization",
    "input-release",
    "aggregation",
    "destruction",
];

struct Uart;
impl Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        print!("{s}");
        Ok(())
    }
}
#[derive(Clone, Copy)]
struct Measurement {
    memory: Phase,
    us: u64,
    stack: Option<Watermark>,
}

#[inline(never)]
fn phase<T>(pattern: Option<u32>, run: impl FnOnce() -> T) -> (T, Measurement) {
    // SAFETY: CPU0 linker stack, initialized HAL, no task switching/second core.
    // No outstanding paint survives a phase; scan precedes UART/next paint.
    let painted = pattern.map(|p| unsafe { stack_watermark::paint_pattern(p) });
    let before = ALLOCATOR.begin();
    let start = Instant::now();
    let result = black_box(run());
    let us = if pattern.is_some() {
        0
    } else {
        start.elapsed().as_micros()
    };
    let memory = ALLOCATOR.finish(before);
    let stack = painted.map(|p| unsafe { p.scan() });
    assert_eq!(memory.failures, 0);
    (result, Measurement { memory, us, stack })
}

#[derive(PartialEq, Eq)]
struct Signature {
    phases: [Option<Phase>; 5],
    pipeline: Phase,
    observed: Observations,
    own: [usize; 8],
}

fn print_memory(tag: &str, name: &str, us: u64, p: Phase) {
    println!(
        "{tag},{name},{us},{},{},{},{},{},{},{}",
        p.requested_peak,
        p.requested_live,
        p.occupied_peak,
        p.occupied_live,
        p.calls,
        p.requested,
        p.failures
    );
}

#[inline(never)]
fn sample(case: &Case, index: usize, pattern: Option<u32>, emit: bool) -> Signature {
    let config = case.config(); // Configuration/static metadata outside pipeline.
    let mut observed = Observations::default();
    let mut phases = [None; 5];
    let baseline = ALLOCATOR.start_pipeline();
    assert_eq!(baseline.requested_live, 0);
    assert_eq!(baseline.occupied_live, 0);
    let (mut copies, p) = phase(pattern.filter(|_| case.lifetime != Lifetime::Flash), || {
        case.copy_inputs()
    });
    phases[0] = Some(p);
    let (loaded, p) = phase(pattern, || {
        m2::load(
            &case.texts(&copies)[..case.groups.len()],
            &mut observed,
            || ALLOCATOR.snapshot(),
        )
    });
    phases[1] = Some(p);
    let (_, p) = phase(None, || {
        if case.lifetime == Lifetime::Dropped {
            copies.fill(None);
        }
    });
    phases[2] = Some(p);
    let result = if loaded.catalogue.is_ok() {
        let (result, p) = phase(pattern, || m2::aggregate(case, &config, &loaded));
        phases[3] = Some(p);
        result
    } else {
        None
    };
    m2::verify(case, &loaded, &result);
    // Actual capacities only for owned Vecs with a public capacity API. For
    // core's private collections report LENGTHS, never infer host capacities.
    let mut own = [0; 8];
    own[0] = loaded.parse_errors.capacity();
    if let Ok(c) = &loaded.catalogue {
        own[2] = c.entries().len();
        own[3] = c.diagnostics().len();
        own[4] = c.entries().iter().map(|e| e.groups().len()).sum();
        own[7] = c
            .diagnostics()
            .iter()
            .map(|d| match d {
                overhead_core::catalogue::CatalogueDiagnostic::EpochConflict {
                    origins, ..
                } => origins.len(),
                _ => 0,
            })
            .sum();
    }
    if let Some((r, candidates)) = &result {
        own[1] = candidates.capacity();
        own[5] = r.satellites().len();
        own[6] = r.satellites().iter().map(|s| s.passes().len()).sum();
    }
    if emit {
        println!("ENDPOINT,{},{}", case.id, index);
        m2::outcomes(&mut Uart, &loaded, &result).unwrap();
        println!("ENDPOINT_DONE");
    }
    let (_, p) = phase(pattern, || {
        // Explicit original retained object destruction, including failure.
        if let Some((reports, candidates)) = result {
            drop(candidates);
            drop(reports);
        }
        drop(loaded);
        drop(copies);
    });
    phases[4] = Some(p);
    let pipeline = ALLOCATOR.finish_pipeline(baseline);
    assert_eq!(pipeline.requested_live, baseline.requested_live);
    assert_eq!(pipeline.occupied_live, baseline.occupied_live);
    assert_eq!(pipeline.failures, 0);
    if emit {
        for (name, p) in NAMES.iter().zip(phases) {
            if let Some(p) = p {
                // Diagnostic records are deliberately not timing evidence.
                print_memory(
                    "PHASE",
                    name,
                    if pattern.is_some() { 0 } else { p.us },
                    p.memory,
                );
            }
        }
        print_memory("PIPE", "whole", 0, pipeline);
        for (i, capacity) in observed.raw_capacities[..case.groups.len()]
            .iter()
            .enumerate()
        {
            println!("RAW,{i},{capacity}");
        }
        for r in &observed.records[..observed.count] {
            println!(
                "RECORD,{},{},{},{},{},{}",
                r.group, r.record, r.requested_live, r.occupied_live, r.calls, r.requested
            );
        }
        println!(
            "OWN,{},{},{},{},{},{},{},{}",
            own[0], own[1], own[2], own[3], own[4], own[5], own[6], own[7]
        );
        for (name, p) in NAMES.iter().zip(phases) {
            if let Some(s) = p.and_then(|p| p.stack) {
                println!(
                    "STACK,{name},{},{},{},{},{},{}",
                    s.stack_low,
                    s.stack_top,
                    s.paint_low,
                    s.paint_high,
                    s.untouched_bytes,
                    s.observed_bytes
                );
            }
        }
        println!("SAMPLE_DONE,{},{}", case.id, index);
    }
    Signature {
        phases: phases.map(|p| p.map(|p| p.memory)),
        pipeline,
        observed,
        own,
    }
}

pub fn run(diagnostic: bool) -> ! {
    let _peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::_240MHz));
    // SAFETY: one initialization of exclusive, aligned static internal storage.
    unsafe {
        ALLOCATOR.backend.add_region(esp_alloc::HeapRegion::new(
            (&raw mut HEAP_STORAGE).cast::<u8>(),
            HEAP_BYTES,
            esp_alloc::MemoryCapability::Internal.into(),
        ));
    }
    println!("OVERHEAD_S3_M2_V1");
    println!("MANIFEST,{},{}", m2::MANIFEST_ID, m2::MANIFEST_SHA256);
    println!("MODE,{}", if diagnostic { "diagnostic" } else { "measure" });
    println!(
        "HEAP,{HEAP_BYTES},{},{}",
        ALLOCATOR.backend.free(),
        ALLOCATOR.backend.used()
    );
    println!("compiler={}", env!("BENCH_RUSTC"));
    println!("cpu_mhz=240,cores_used=1,psram=unused,allocator=esp-alloc-0.11.0-LLFF,math=f64_libm");
    if diagnostic {
        #[cfg(feature = "numerical-reference")]
        {
            use overhead_s3_benchmark::numerical;
            let summary = numerical::run(|check, result| {
                println!("NUM,{:?},{},{result:?}", check.family, check.index);
            });
            assert_eq!(summary.checked, 75);
            assert_eq!(summary.failed, 0);
        }
        #[cfg(not(feature = "numerical-reference"))]
        panic!("diagnostic requires numerical-reference");
        #[cfg(feature = "numerical-reference")]
        for (index, pattern) in [0xa5a5_a5a5, 0x5a5a_5a5a].into_iter().enumerate() {
            // SAFETY: same exclusive CPU0 linker stack as phase().
            let (base, loaded) = unsafe { stack_watermark::self_check_pattern(pattern) };
            println!(
                "PROBE,{pattern},{},{}",
                base.observed_bytes, loaded.observed_bytes
            );
            for case in &m2::CASES {
                println!("BEGIN,{},{}", case.id, index);
                black_box(sample(case, index, Some(pattern), true));
            }
        }
    } else {
        for case in &m2::CASES {
            let warmup = sample(case, 0, None, false);
            println!("WARMUP,{},ok", case.id);
            for index in 0..3 {
                println!("BEGIN,{},{}", case.id, index);
                let current = sample(case, index, None, true);
                assert!(
                    current == warmup,
                    "nonrepeating heap/work/collection observations"
                );
            }
        }
    }
    println!("OVERHEAD_S3_M2_DONE");
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("OVERHEAD_S3_M2_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
