#![no_std]
#![no_main]
//! Separate opt-in S3 + Sharp cost diagnostic; not a clocked device app.
//! Shows the physically accepted v2 replay, then measures one offscreen
//! control per frame. Never sends the control framebuffer to the panel.
use core::{mem::MaybeUninit, panic::PanicInfo};
use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    spi::{
        BitOrder, Mode,
        master::{Config, Spi},
    },
    time::{Duration, Instant, Rate},
};
use esp_println::println;
use overhead_core::sgp4::chrono::TimeDelta;
use overhead_render::{CoastLayer, Layout};
use overhead_s3_benchmark::{coast_measurement::Screen, sharp_profile::Control, sharp_test};

esp_bootloader_esp_idf::esp_app_desc!();

static mut SCREEN: Screen = Screen::new();
static mut LAYER: MaybeUninit<CoastLayer> = MaybeUninit::uninit();
const HEAP_BYTES: usize = 64 * 1024;
#[repr(C, align(16))]
struct HeapStorage(MaybeUninit<[u8; HEAP_BYTES]>);
static mut HEAP_STORAGE: HeapStorage = HeapStorage(MaybeUninit::uninit());
#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn wait_until(start: Instant, duration: Duration) {
    while start.elapsed() < duration {
        core::hint::spin_loop();
    }
}

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
        // SAFETY: single-core, sole owner of statics and allocator region.
        ALLOCATOR.add_region(esp_alloc::HeapRegion::new(
            (&raw mut HEAP_STORAGE.0).cast::<u8>(),
            HEAP_BYTES,
            esp_alloc::MemoryCapability::Internal.into(),
        ));
    }
    println!("OVERHEAD_S3_SHARP_PROFILE_GLYPH_V3 5deg, 0..300s historical ISS, 2MHz SPI");
    let mut app = overhead_s3_benchmark::coast_measurement::app("iss-ground-20", 0.0, 0.0, 5.0);
    let epoch = app.time().expect("fixture epoch");
    let screen = unsafe { &mut *(&raw mut SCREEN) };
    let coast = unsafe { (*(&raw mut LAYER)).write(CoastLayer::new()) };
    coast.prepare_for(app.observer());
    assert!(coast.error().is_none(), "coast preparation failed");
    cs.set_high();
    sharp_test::clear(true, |bytes| spi.write(bytes)).expect("Sharp clear");
    cs.set_low();

    let mut tick = 0_u32;
    loop {
        let start = Instant::now();
        let seconds = sharp_test::replay_second(tick);
        app.update(epoch + TimeDelta::seconds(seconds))
            .expect("fixture replay UTC");
        let update_us = start.elapsed().as_micros();
        screen.reset_counts();
        overhead_render::draw(screen, &app, Layout::wide(), false, coast).unwrap();
        let render_us = start.elapsed().as_micros() - update_us;
        let checksum = screen.checksum();
        cs.set_high();
        sharp_test::frame_pixels(tick % 2 != 0, &screen.pixels, |bytes| spi.write(bytes))
            .expect("Sharp profile frame");
        cs.set_low();
        let frame_us = start.elapsed().as_micros();
        println!(
            "FRAME second={seconds} selected={:?} checksum={checksum} update_us={update_us} render_us={render_us} transfer_us={} total_us={frame_us}",
            app.selected_id(),
            frame_us - render_us - update_us,
        );

        // Controls are offscreen after transfer and overwritten before the
        // next transfer. Skip if this frame was already very slow; the
        // remaining control is not preemptible, so VCOM still needs checking.
        if frame_us < 300_000 {
            let control = Control::for_tick(tick);
            let sample = Instant::now();
            let mut stamps = [0_u64; 9];
            let mut glyph_stamps = [0_u64; 5];
            let mut events = 0;
            let mut glyph_events = 0;
            control.run_traced_fields(
                screen,
                &app,
                coast,
                |stage| {
                    stamps[stage as usize] = sample.elapsed().as_micros();
                    events += 1;
                },
                |stage| {
                    glyph_stamps[stage as usize] = sample.elapsed().as_micros();
                    glyph_events += 1;
                },
            );
            let us = sample.elapsed().as_micros();
            // Coast control includes a fast RAM clear, so its cost is an
            // upper bound on the cached layer's isolated draw.
            println!(
                "CONTROL kind={} us={us} writes={} ink_writes={}",
                control.name(),
                screen.writes,
                screen.on_writes
            );
            if matches!(control, Control::FrameNoCoast) {
                assert_eq!(events, 9, "incomplete coast-free trace");
                assert_eq!(glyph_events, 5, "incomplete glyph trace");
                let [
                    clear,
                    coast,
                    earth,
                    radar,
                    markers,
                    label,
                    category,
                    glyphs,
                    status,
                ] = stamps;
                println!(
                    "TRACE clear_us={clear} coast_us={} earth_us={} radar_us={} markers_us={} label_us={} category_us={} glyphs_us={} status_us={} callback_tail_us={}",
                    coast - clear,
                    earth - coast,
                    radar - earth,
                    markers - radar,
                    label - markers,
                    category - label,
                    glyphs - category,
                    status - glyphs,
                    us - status,
                );
                let [separator, elevation, altitude, speed, period] = glyph_stamps;
                println!(
                    "GLYPH separator_us={} elevation_us={} altitude_us={} speed_us={} period_us={} callback_tail_us={}",
                    separator - category,
                    elevation - separator,
                    altitude - elevation,
                    speed - altitude,
                    period - speed,
                    glyphs - period,
                );
            }
        } else {
            println!("CONTROL skipped, render/transfer exceeded 300ms");
        }
        tick = tick.wrapping_add(1);
        wait_until(start, Duration::from_millis(500));
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("OVERHEAD_S3_SHARP_PROFILE_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
