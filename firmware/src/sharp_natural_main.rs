#![no_std]
#![no_main]
//! Opt-in natural-speed HISTORICAL fixture, not live UTC or device scheduling.
//! No controls/network; no 20 Hz or sustained-VCOM guarantee.
use core::{mem::MaybeUninit, panic::PanicInfo};
use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    spi::{
        BitOrder, Mode,
        master::{Config, Spi},
    },
    time::{Instant, Rate},
};
use esp_println::println;
use overhead_core::sgp4::chrono::TimeDelta;
use overhead_render::{CoastLayer, Layout, PeriodCache};
#[cfg(feature = "sharp-natural-audit")]
use overhead_s3_benchmark::natural_audit::NaturalAudit;
use overhead_s3_benchmark::{
    coast_measurement::Screen,
    device_timing::{DiagnosticAction, DiagnosticSchedule, historical_offset_us},
    sharp_lines::{Kind, LineSender},
    sharp_test,
};

esp_bootloader_esp_idf::esp_app_desc!();
static mut SCREEN: Screen = Screen::new();
static mut SENDER: LineSender = LineSender::new();
static mut LAYER: MaybeUninit<CoastLayer> = MaybeUninit::uninit();
static mut PERIOD: PeriodCache = PeriodCache::new();
const HEAP_BYTES: usize = 64 * 1024;
#[repr(C, align(16))]
struct HeapStorage(MaybeUninit<[u8; HEAP_BYTES]>);
static mut HEAP_STORAGE: HeapStorage = HeapStorage(MaybeUninit::uninit());
#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

#[esp_hal::main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::_240MHz));
    let mut cs = Output::new(peripherals.GPIO10, Level::Low, OutputConfig::default());
    let mut spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(2))
            .with_mode(Mode::_0)
            .with_write_bit_order(BitOrder::LsbFirst),
    )
    .expect("SPI configuration")
    .with_sck(peripherals.GPIO13)
    .with_mosi(peripherals.GPIO11);
    unsafe {
        // SAFETY: sole owner of statics, single core, no tasks or scheduler.
        ALLOCATOR.add_region(esp_alloc::HeapRegion::new(
            (&raw mut HEAP_STORAGE.0).cast::<u8>(),
            HEAP_BYTES,
            esp_alloc::MemoryCapability::Internal.into(),
        ));
    }
    #[cfg(feature = "sharp-natural-audit")]
    println!("OVERHEAD_S3_SHARP_NATURAL_AUDIT_V2 historical fixture, not trusted UTC");
    #[cfg(not(feature = "sharp-natural-audit"))]
    println!("OVERHEAD_S3_SHARP_NATURAL_V1 historical fixture, not trusted UTC");
    let mut app = overhead_s3_benchmark::coast_measurement::app("iss-ground-20", 0.0, 0.0, 5.0);
    let epoch = app.time().expect("fixture epoch");
    // This checked historical fixture epoch is NOT an SNTP sample. Only this
    // diagnostic supplies it to the monotonic frame grid; no device clock exists.
    let epoch_us = epoch.and_utc().timestamp_micros();
    let screen = unsafe { &mut *(&raw mut SCREEN) };
    let sender = unsafe { &mut *(&raw mut SENDER) };
    let period = unsafe { &mut *(&raw mut PERIOD) };
    let coast = unsafe { (*(&raw mut LAYER)).write(CoastLayer::new()) };
    coast.prepare_for(app.observer());
    assert!(coast.error().is_none(), "coast preparation failed");
    let start = Instant::now();
    let mut schedule = DiagnosticSchedule::new();
    #[cfg(feature = "sharp-natural-audit")]
    let mut audit = NaturalAudit::new();
    let phase = schedule.next_phase();
    #[cfg(feature = "sharp-natural-audit")]
    let clear_start_us = start.elapsed().as_micros();
    cs.set_high();
    let result = sharp_test::clear(phase, |part| spi.write(part));
    cs.set_low();
    result.expect("Sharp clear");
    let clear_done_us = start.elapsed().as_micros();
    schedule.transfer_succeeded(clear_done_us, phase, false);
    #[cfg(feature = "sharp-natural-audit")]
    assert!(
        audit.completed_command(clear_start_us, clear_done_us, phase, false),
        "invalid clear command telemetry"
    );
    let anchor_us = start.elapsed().as_micros();
    schedule.sync_utc(anchor_us, epoch_us);
    let mut confirmed = false;
    let mut frames = 0_u64;
    loop {
        let now_us = start.elapsed().as_micros();
        match schedule.next(now_us, confirmed) {
            DiagnosticAction::Frame { sample, full } => {
                // Exactly 300 fixture seconds per loop, at wall-time speed;
                // sample at actual frame start, never at a missed deadline.
                let offset_us = historical_offset_us(sample.utc_unix_us, epoch_us)
                    .expect("fixture time overflow");
                app.update(epoch + TimeDelta::microseconds(offset_us))
                    .expect("historical fixture sample");
                let update_done_us = start.elapsed().as_micros();
                screen.reset_counts();
                overhead_render::draw_cached(screen, &app, Layout::wide(), false, coast, period)
                    .expect("fixture render");
                let render_done_us = start.elapsed().as_micros();
                let phase = schedule.next_phase();
                #[cfg(feature = "sharp-natural-audit")]
                let command_start_us = start.elapsed().as_micros();
                cs.set_high();
                let result = sender.send(phase, &screen.pixels, full, |part| spi.write(part));
                cs.set_low();
                let transfer = match result {
                    Ok(transfer) => transfer,
                    Err(_) => {
                        schedule.transfer_failed();
                        panic!("Sharp frame transfer failed; panel state unknown");
                    }
                };
                let done_us = start.elapsed().as_micros();
                schedule.transfer_succeeded(done_us, phase, transfer.kind == Kind::Full);
                #[cfg(feature = "sharp-natural-audit")]
                assert!(
                    audit.completed_command(
                        command_start_us,
                        done_us,
                        phase,
                        transfer.kind == Kind::Full
                    ) && audit.completed_frame(now_us, done_us, sample.skipped),
                    "invalid frame telemetry"
                );
                confirmed = true;
                frames += 1;
                if frames.is_multiple_of(20) || transfer.kind == Kind::Full || sample.skipped > 0 {
                    println!(
                        "NATURAL frame={frames} fixture_offset_us={offset_us} skipped={} kind={:?} rows={} bytes={} update_us={} render_us={} transfer_us={} total_us={}",
                        sample.skipped,
                        transfer.kind,
                        transfer.rows,
                        transfer.bytes,
                        update_done_us - now_us,
                        render_done_us - update_done_us,
                        done_us - render_done_us,
                        done_us - now_us
                    );
                    #[cfg(feature = "sharp-natural-audit")]
                    println!(
                        "AUDIT_V2 frames={} skipped={} frame_over_budget={} max_frame_us={} commands={} full={} max_full_us={} max_command_start_gap_us={} max_no_command_us={}",
                        audit.frames,
                        audit.skipped_deadlines,
                        audit.over_budget_frames,
                        audit.max_frame_us,
                        audit.commands,
                        audit.full_commands,
                        audit.max_full_us,
                        audit.max_command_start_gap_us,
                        audit.max_no_command_us
                    );
                }
            }
            DiagnosticAction::RefreshRetained => {
                let phase = schedule.next_phase();
                #[cfg(feature = "sharp-natural-audit")]
                let command_start_us = start.elapsed().as_micros();
                cs.set_high();
                let result = sender.send(phase, &screen.pixels, true, |part| spi.write(part));
                cs.set_low();
                if result.is_err() {
                    schedule.transfer_failed();
                    panic!("Sharp retained refresh failed; panel state unknown");
                }
                let done_us = start.elapsed().as_micros();
                schedule.transfer_succeeded(done_us, phase, true);
                #[cfg(feature = "sharp-natural-audit")]
                {
                    assert!(
                        audit.completed_command(command_start_us, done_us, phase, true),
                        "invalid retained command telemetry"
                    );
                    println!(
                        "AUDIT_V2 retained_full start_us={command_start_us} done_us={done_us} transfer_us={}",
                        done_us - command_start_us
                    );
                }
                println!("NATURAL retained_full uptime_us={done_us}");
            }
            DiagnosticAction::IdleVcom => {
                // Before the first confirmed frame, the next action is a full
                // frame; no sender idle transaction is valid without it.
                if confirmed {
                    let phase = schedule.next_phase();
                    #[cfg(feature = "sharp-natural-audit")]
                    let command_start_us = start.elapsed().as_micros();
                    cs.set_high();
                    let result = sender.idle(phase, |part| spi.write(part));
                    cs.set_low();
                    if result.is_err() {
                        schedule.transfer_failed();
                        panic!("Sharp idle transfer failed; panel state unknown");
                    }
                    let done_us = start.elapsed().as_micros();
                    schedule.transfer_succeeded(done_us, phase, false);
                    #[cfg(feature = "sharp-natural-audit")]
                    assert!(
                        audit.completed_command(command_start_us, done_us, phase, false),
                        "invalid idle command telemetry"
                    );
                }
            }
            DiagnosticAction::Wait => core::hint::spin_loop(),
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("OVERHEAD_S3_SHARP_NATURAL_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
