#![no_std]
#![no_main]
//! Isolated rotary-encoder wiring check, NOT the device application.
//! A->IO4, C->GND, B->IO5, switch->IO6 and GND; internal pull-ups.
//! PCNT unit 0 decodes quadrature (4 counts per full A/B cycle) with the
//! maximum glitch filter; the loop logs count changes, switch edges and a
//! periodic line of raw pin levels. No display, allocator or network.
use core::panic::PanicInfo;
use esp_hal::{
    clock::CpuClock,
    gpio::{Input, InputConfig, Pull},
    pcnt::{Pcnt, channel},
    time::{Duration, Instant},
};
use esp_println::println;

esp_bootloader_esp_idf::esp_app_desc!();

/// 1023 APB (80 MHz) cycles ≈ 12.8 µs: the largest filter the unit accepts.
const FILTER_CYCLES: u16 = 1023;
const POLL: Duration = Duration::from_millis(2);
/// Consecutive equal switch samples (×POLL) before a level is accepted.
const SWITCH_STABLE: u8 = 10;
const HEARTBEAT: Duration = Duration::from_secs(2);

#[esp_hal::main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::_240MHz));
    let pull_up = InputConfig::default().with_pull(Pull::Up);
    let pin_a = Input::new(p.GPIO4, pull_up);
    let pin_b = Input::new(p.GPIO5, pull_up);
    let switch = Input::new(p.GPIO6, pull_up);

    let pcnt = Pcnt::new(p.PCNT);
    let unit = pcnt.unit0;
    unit.set_filter(Some(FILTER_CYCLES)).expect("PCNT filter");
    unit.clear();
    let ch0 = &unit.channel0;
    ch0.set_ctrl_signal(pin_a.peripheral_input());
    ch0.set_edge_signal(pin_b.peripheral_input());
    ch0.set_ctrl_mode(channel::CtrlMode::Reverse, channel::CtrlMode::Keep);
    ch0.set_input_mode(channel::EdgeMode::Increment, channel::EdgeMode::Decrement);
    let ch1 = &unit.channel1;
    ch1.set_ctrl_signal(pin_b.peripheral_input());
    ch1.set_edge_signal(pin_a.peripheral_input());
    ch1.set_ctrl_mode(channel::CtrlMode::Reverse, channel::CtrlMode::Keep);
    ch1.set_input_mode(channel::EdgeMode::Decrement, channel::EdgeMode::Increment);
    unit.resume();

    println!("OVERHEAD_S3_KNOB_TEST_V1 IO4=A IO5=B IO6=SW filter_cycles={FILTER_CYCLES}");
    let mut last_raw = unit.value();
    let mut total: i32 = 0;
    let mut sw_level = switch.is_high();
    let mut sw_candidate = sw_level;
    let mut sw_run: u8 = 0;
    let mut presses: u32 = 0;
    let mut last_beat = Instant::now();
    loop {
        let start = Instant::now();
        let raw = unit.value();
        let delta = raw.wrapping_sub(last_raw);
        if delta != 0 {
            last_raw = raw;
            total += i32::from(delta);
            println!(
                "KNOB_COUNT t_ms={} delta={} total={} a={} b={}",
                start.duration_since_epoch().as_millis(),
                delta,
                total,
                pin_a.is_high() as u8,
                pin_b.is_high() as u8
            );
        }
        let sample = switch.is_high();
        if sample == sw_candidate {
            sw_run = sw_run.saturating_add(1);
        } else {
            sw_candidate = sample;
            sw_run = 1;
        }
        if sw_run >= SWITCH_STABLE && sw_candidate != sw_level {
            sw_level = sw_candidate;
            if !sw_level {
                presses += 1;
            }
            println!(
                "KNOB_SWITCH t_ms={} pressed={} presses={}",
                start.duration_since_epoch().as_millis(),
                !sw_level as u8,
                presses
            );
        }
        if last_beat.elapsed() >= HEARTBEAT {
            last_beat = Instant::now();
            println!(
                "KNOB_LEVELS a={} b={} sw={} total={} presses={}",
                pin_a.is_high() as u8,
                pin_b.is_high() as u8,
                switch.is_high() as u8,
                total,
                presses
            );
        }
        while start.elapsed() < POLL {
            core::hint::spin_loop();
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("OVERHEAD_S3_KNOB_TEST_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
