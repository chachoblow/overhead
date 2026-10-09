#![no_std]
#![no_main]

use core::{hint::black_box, panic::PanicInfo};
use esp_hal::{
    clock::CpuClock,
    time::{Duration, Instant},
};
use esp_println::println;
use overhead_s3_benchmark::{
    CLASSES, Work, satellites,
    suites::{Suite, sample_count},
};

esp_bootloader_esp_idf::esp_app_desc!();

fn pause() {
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(20) {
        core::hint::spin_loop();
    }
}

fn measure(id: usize, samples: usize, mut run: impl FnMut() -> Work) {
    println!("BEGIN,{id}");
    pause(); // Let UART diagnostics drain before warm-up/timing.
    let expected = black_box(run());
    let mut elapsed = [0_u64; 5];
    for us in &mut elapsed[..samples] {
        let start = Instant::now();
        let actual = black_box(run());
        *us = start.elapsed().as_micros();
        assert_eq!(actual, expected, "work counts changed");
    }
    // No serial output inside timed sections. One invocation per sample.
    for (sample, us) in elapsed[..samples].iter().enumerate() {
        println!(
            "ROW,{id},{sample},{us},{},{},{}",
            expected.state_evaluations, expected.searches, expected.passes
        );
    }
}

#[esp_hal::main]
fn main() -> ! {
    let _peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::_240MHz));
    pause();
    println!("OVERHEAD_S3_BENCHMARK_V2");
    let suite = Suite::parse(env!("BENCH_SUITE")).expect("unknown BENCH_SUITE");
    let samples = sample_count(env!("BENCH_SAMPLES")).expect("invalid BENCH_SAMPLES");
    println!("SUITE,{},{samples},{}", suite.name(), suite.len());
    println!("compiler={}", env!("BENCH_RUSTC"));
    println!(
        "cpu_mhz=240,cores_used=1,psram=unused,allocator=none,math=f64_libm,debug_assertions={}",
        cfg!(debug_assertions)
    );
    println!(
        "age_days=grid_center_relative_to_epoch; start=center_minus12h; tracking=1441_ticks_60s; threshold_deg=10"
    );
    println!("warmups=1,iterations_per_sample=1,timer=esp_hal_Instant_us");
    println!(
        "sizeof_satellite={},sizeof_one_age_time_grid={},sizeof_prepared={}",
        core::mem::size_of::<overhead_core::Satellite>(),
        core::mem::size_of_val(&overhead_s3_benchmark::TIMES),
        core::mem::size_of::<overhead_s3_benchmark::suites::Prepared>()
    );
    let satellites = satellites();
    for (i, class) in CLASSES.iter().enumerate() {
        println!(
            "INPUT,{class},{},{}",
            satellites[i].norad_id(),
            satellites[i].epoch()
        );
    }
    println!("case_fields=id,operation,class,slots,age_days,window_s,detection_s,tolerance_ms");
    println!("row_fields=id,sample,elapsed_us,state_evaluations,searches,passes");
    for (id, workload) in suite.workloads().enumerate() {
        println!("CASE,{id},{workload}");
        let prepared = workload.prepare(); // Excluded from timing.
        measure(id, samples, || prepared.run(&satellites));
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
