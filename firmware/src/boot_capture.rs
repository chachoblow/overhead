//! Target-side 0070 button capture shared by the device runtime and the BOOT
//! diagnostic images. A periodic hardware-timer interrupt samples the
//! selection input (GPIO0 on the diagnostics, the selection button on the
//! device, 0088) and optionally the knob switch into `PressLatch`es
//! regardless of render, SPI or executor state. The display owner drains
//! completed presses at its own loop boundary and applies 0068 via
//! `button_input::apply_presses`; the interrupt never touches app or panel.
use crate::button_input::{PressLatch, SampleStats};
use core::cell::RefCell;
use critical_section::Mutex;
use esp_hal::{
    Blocking,
    gpio::Input,
    handler,
    time::{Duration, Instant},
    timer::PeriodicTimer,
};

/// 0070 sampling period.
pub const SAMPLE_PERIOD: Duration = Duration::from_millis(5);

struct Capture {
    timer: PeriodicTimer<'static, Blocking>,
    boot: Input<'static>,
    latch: PressLatch,
    info: Option<(Input<'static>, PressLatch)>,
}

// Shared only with `sample_boot`; drains hold it just long enough to take
// counts.
static CAPTURE: Mutex<RefCell<Option<Capture>>> = Mutex::new(RefCell::new(None));

#[handler]
fn sample_boot() {
    critical_section::with(|cs| {
        if let Some(capture) = CAPTURE.borrow_ref_mut(cs).as_mut() {
            capture.timer.clear_interrupt();
            let now = Instant::now().duration_since_epoch().as_micros();
            capture.latch.sample(now, capture.boot.is_low());
            if let Some((input, latch)) = capture.info.as_mut() {
                latch.sample(now, input.is_low());
            }
        }
    });
}

/// Starts 5 ms sampling of an active-low `boot` (selection) input on `timer`,
/// which must not be shared with esp-rtos or Wi-Fi. A press held through
/// startup is not counted until released and pressed again.
pub fn start(timer: PeriodicTimer<'static, Blocking>, boot: Input<'static>, debounce_us: u64) {
    start_with_info(timer, boot, None, debounce_us);
}

/// [`start`], also sampling an active-low `info` input (the knob switch,
/// 0087/0088) into its own latch, drained by [`take_info_presses`]. A setup
/// hold still pressed when sampling starts never counts.
pub fn start_with_info(
    mut timer: PeriodicTimer<'static, Blocking>,
    boot: Input<'static>,
    info: Option<Input<'static>>,
    debounce_us: u64,
) {
    let latch = PressLatch::new(debounce_us).expect("BOOT debounce interval");
    let info = info.map(|input| (input, latch));
    timer.set_interrupt_handler(sample_boot);
    critical_section::with(|cs| {
        timer.start(SAMPLE_PERIOD).expect("5 ms BOOT sample timer");
        timer.listen();
        CAPTURE.borrow_ref_mut(cs).replace(Capture {
            timer,
            boot,
            latch,
            info,
        });
    });
}

/// Completed presses since the last drain (0 before `start`).
pub fn take_presses() -> u16 {
    critical_section::with(|cs| {
        CAPTURE
            .borrow_ref_mut(cs)
            .as_mut()
            .map_or(0, |capture| capture.latch.take_presses())
    })
}

/// Completed info-input presses since the last drain (0 without one).
pub fn take_info_presses() -> u16 {
    critical_section::with(|cs| {
        CAPTURE
            .borrow_ref_mut(cs)
            .as_mut()
            .and_then(|capture| capture.info.as_mut())
            .map_or(0, |(_, latch)| latch.take_presses())
    })
}

/// Sample count and largest inter-sample gap since the last call. Expect
/// ~2000 samples per 10 s; `max_gap_us` is interrupt-latency evidence, since
/// critical sections (UART println, drivers, scheduler) can delay a sample.
pub fn take_stats() -> SampleStats {
    critical_section::with(|cs| {
        CAPTURE
            .borrow_ref_mut(cs)
            .as_mut()
            .map(|capture| capture.latch.take_stats())
    })
    .unwrap_or_default()
}
