#![no_std]
#![no_main]

use core::{hint::black_box, panic::PanicInfo};
use esp_hal::{
    clock::CpuClock,
    time::{Duration, Instant},
};
use esp_println::println;
use overhead_s3_benchmark::{CLASSES, Work, configs, predict, satellites, track};

esp_bootloader_esp_idf::esp_app_desc!();

const SAMPLES: usize = 5;

fn pause() {
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(20) {
        core::hint::spin_loop();
    }
}

fn measure(operation: &str, class: &str, slots: usize, mut run: impl FnMut() -> Work) {
    println!("BEGIN,{operation},{class},{slots}");
    pause(); // Let UART diagnostics drain before warm-up/timing.
    let expected = black_box(run());
    let mut elapsed = [0_u64; SAMPLES];
    for us in &mut elapsed {
        let start = Instant::now();
        let actual = black_box(run());
        *us = start.elapsed().as_micros();
        assert_eq!(actual, expected, "work counts changed");
    }
    // No serial output inside timed sections. One invocation per sample;
    // each invocation already does >=1441 orbital state evaluations.
    for (sample, us) in elapsed.into_iter().enumerate() {
        println!(
            "ROW,{operation},{class},{slots},{sample},{us},{},{},{}",
            expected.state_evaluations, expected.searches, expected.passes
        );
    }
}

#[esp_hal::main]
fn main() -> ! {
    let _peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::_240MHz));
    pause();
    println!("OVERHEAD_S3_BENCHMARK_V1");
    println!("compiler={}", env!("BENCH_RUSTC"));
    println!(
        "cpu_mhz=240,cores_used=1,psram=unused,allocator=none,math=f64_libm,debug_assertions={}",
        cfg!(debug_assertions)
    );
    println!(
        "tracking=1441_ticks_60s_epoch_minus12h_to_plus12h; prediction=24h_60s_detection_5s_tolerance_10deg"
    );
    println!("samples=5,warmups=1,iterations_per_sample=1,timer=esp_hal_Instant_us");
    println!(
        "sizeof_satellite={},sizeof_time_grid={}",
        core::mem::size_of::<overhead_core::Satellite>(),
        core::mem::size_of_val(&overhead_s3_benchmark::TIMES)
    );
    let satellites = satellites();
    let configs = configs();
    println!("operation,class,slots,sample,elapsed_us,state_evaluations,searches,passes");
    for (i, class) in CLASSES.iter().enumerate() {
        println!(
            "INPUT,{class},{},{}",
            satellites[i].norad_id(),
            satellites[i].epoch()
        );
        measure("state_at", class, 1, || track(&satellites, &[i], false));
        measure("state_at+ecef+look_angles", class, 1, || {
            track(&satellites, &[i], true)
        });
        measure("search_satellite", class, 1, || {
            predict(&satellites, &[i], &configs)
        });
    }
    let slots = [0, 1, 2, 3];
    // Larger batches are deliberately deferred: this baseline already takes
    // minutes with software f64 math. This is not a four-satellite capacity limit.
    measure(
        "state_at+ecef+look_angles",
        "mixed-repeated-fixtures",
        slots.len(),
        || track(&satellites, &slots, true),
    );
    measure(
        "search_satellite",
        "mixed-repeated-fixtures",
        slots.len(),
        || predict(&satellites, &slots, &configs),
    );
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
