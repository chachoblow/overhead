#![no_std]
#![no_main]
//! Opt-in live-UTC ISS radar candidate, NOT a standalone device application.
//! One display task owns app, gate and SPI; the network side only sends screened
//! events over a bounded channel. Never flash without separate approval/gates.
use core::{mem::MaybeUninit, panic::PanicInfo};
use embassy_executor::Spawner;
use embassy_net::{
    IpEndpoint, Runner, Stack, StackResources,
    dns::DnsQueryType,
    udp::{PacketMetadata, UdpSocket},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration as Wait, Timer, with_timeout};
#[cfg(feature = "live-utc-boot")]
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    rng::Rng,
    spi::{
        BitOrder, Mode,
        master::{Config as SpiConfig, Spi},
    },
    time::{Instant, Rate},
    timer::timg::TimerGroup,
};
use esp_println::println;
use esp_radio::wifi::{
    AuthenticationMethodConfig, Config, ControllerConfig, Interface, WifiController,
    sta::StationConfig,
};
use overhead_core::sgp4::chrono::DateTime;
use overhead_render::{CoastLayer, Layout, PeriodCache};
#[cfg(feature = "live-utc-boot")]
use overhead_s3_benchmark::button_input::CompletedPress;
use overhead_s3_benchmark::{
    coast_measurement::{self, Screen},
    device_timing::{DiagnosticAction, LifecycleAction},
    live_utc,
    sharp_lines::{Kind, LineSender},
    sntp_screen::{self, SntpOutcome, SntpRequest},
    utc_trust::{PROTOTYPE_DRIFT_PPM, UtcTrustPreflight},
};
use static_cell::StaticCell;

esp_bootloader_esp_idf::esp_app_desc!();

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");
const SERVERS: [&str; 2] = ["time.cloudflare.com", "time.nist.gov"];
const POLL_S: u64 = 64;
const REPLY_TIMEOUT: Wait = Wait::from_secs(5);
// A successful send is never dropped; if full, network yields until the
// display owner drains it. Processing time, not packet time, is used for age.
static EVENTS: Channel<CriticalSectionRawMutex, SntpOutcome, 4> = Channel::new();
static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
static RX_META: StaticCell<[PacketMetadata; 4]> = StaticCell::new();
static TX_META: StaticCell<[PacketMetadata; 4]> = StaticCell::new();
static RX_BUF: StaticCell<[u8; 512]> = StaticCell::new();
static TX_BUF: StaticCell<[u8; 512]> = StaticCell::new();
// Display task is the sole owner; avoid putting large buffers/layer in its
// async future or on the small S3 stack.
static mut SCREEN: Screen = Screen::new();
static mut SENDER: LineSender = LineSender::new();
static mut LAYER: MaybeUninit<CoastLayer> = MaybeUninit::uninit();
static mut PERIOD: PeriodCache = PeriodCache::new();

fn uptime_us() -> u64 {
    Instant::now().duration_since_epoch().as_micros()
}

#[esp_hal::main]
async fn main(spawner: Spawner) -> ! {
    let p = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::_240MHz));
    esp_alloc::heap_allocator!(size: 96 * 1024);
    let timg0 = TimerGroup::new(p.TIMG0);
    esp_rtos::start(timg0.timer0, p.FROM_CPU_INTR0);
    println!("OVERHEAD_S3_LIVE_UTC_RADAR_V1 ISS fixture elements; not fetched or production");

    let cs = Output::new(p.GPIO10, Level::Low, OutputConfig::default());
    let spi = Spi::new(
        p.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(2))
            .with_mode(Mode::_0)
            .with_write_bit_order(BitOrder::LsbFirst),
    )
    .expect("SPI configuration")
    .with_sck(p.GPIO13)
    .with_mosi(p.GPIO11);
    // GPIO0 is the DevKitC-1 BOOT strap. Configure only after startup;
    // a held BOOT at startup is ignored by CompletedPress until released.
    #[cfg(feature = "live-utc-boot")]
    let boot = Input::new(p.GPIO0, InputConfig::default().with_pull(Pull::Up));
    spawner.spawn(
        display(
            spi,
            cs,
            #[cfg(feature = "live-utc-boot")]
            boot,
        )
        .unwrap(),
    );

    let station = Config::Station(
        StationConfig::default()
            .with_ssid(SSID.try_into().expect("SSID"))
            .with_authentication(AuthenticationMethodConfig::Wpa2Personal(
                PASSWORD.try_into().expect("PASSWORD"),
            )),
    );
    let controller = WifiController::new(
        p.WIFI,
        ControllerConfig::default().with_initial_config(station),
    )
    .expect("Wi-Fi controller");
    let rng = Rng::new();
    let seed = (u64::from(rng.random()) << 32) | u64::from(rng.random());
    let (stack, runner) = embassy_net::new(
        Interface::station(),
        embassy_net::Config::dhcpv4(Default::default()),
        RESOURCES.init(StackResources::new()),
        seed,
    );
    spawner.spawn(net(runner).unwrap());
    spawner.spawn(connection(controller).unwrap());
    stack.wait_config_up().await;
    let mut socket = UdpSocket::new(
        stack,
        RX_META.init([PacketMetadata::EMPTY; 4]),
        RX_BUF.init([0; 512]),
        TX_META.init([PacketMetadata::EMPTY; 4]),
        TX_BUF.init([0; 512]),
    );
    socket.bind(0).expect("SNTP UDP bind");
    let mut seq: u32 = 0;
    loop {
        stack.wait_config_up().await;
        seq = seq.wrapping_add(1);
        for server in SERVERS {
            match query(stack, &mut socket, &rng, server, seq).await {
                QueryResult::NoReply => continue, // fallback only after no reply
                QueryResult::Reply => break,
                QueryResult::KissOfDeath => {
                    // Conservative diagnostic handling: stop querying either
                    // server after any KoD; do not bypass RATE/DENY via fallback.
                    println!("SNTP KoD: stop queries until reboot; holdover expires normally");
                    loop {
                        Timer::after(Wait::from_secs(3600)).await;
                    }
                }
            }
        }
        Timer::after(Wait::from_secs(POLL_S)).await;
    }
}

#[embassy_executor::task]
async fn display(
    mut spi: Spi<'static, esp_hal::Blocking>,
    mut cs: Output<'static>,
    #[cfg(feature = "live-utc-boot")] boot: Input<'static>,
) {
    // Fixture epoch initializes elements/observer only; the gate invalidates
    // this pre-populated sample before a cold-boot draw unless sync arrives.
    let mut app = coast_measurement::app("iss-ground-20", 0.0, 0.0, 5.0);
    let screen = unsafe { &mut *(&raw mut SCREEN) };
    let sender = unsafe { &mut *(&raw mut SENDER) };
    let period = unsafe { &mut *(&raw mut PERIOD) };
    let coast = unsafe { (*(&raw mut LAYER)).write(CoastLayer::new()) };
    // SAFETY: this task alone owns these statics and SPI; the network side
    // sends Copy events over EVENTS and never touches render/app/panel state.
    coast.prepare_for(app.observer());
    assert!(coast.error().is_none(), "coast preparation failed");
    let mut gate = UtcTrustPreflight::prototype();
    let mut retained = false;
    let mut frames = 0u64;
    // Provisional 20 ms qualification; sampling occurs at the display loop
    // boundary, not during blocking render/SPI or network execution.
    #[cfg(feature = "live-utc-boot")]
    let mut press = CompletedPress::new(20_000).unwrap();
    loop {
        while let Ok(event) = EVENTS.try_receive() {
            let now = uptime_us();
            let decision = live_utc::apply(&mut gate, &mut app, now, event);
            println!("LIVE_UTC source={decision:?} uptime_us={now}");
        }
        let now = uptime_us();
        #[cfg(feature = "live-utc-boot")]
        if press.poll_selection(now, boot.is_low(), &mut app) {
            println!(
                "BOOT completed uptime_us={now} valid={} selected={:?}",
                app.time().is_some(),
                app.selected_id()
            );
        }
        match gate.next(&mut app, now, retained) {
            LifecycleAction::RenderNoFrame => {
                overhead_render::draw_cached(screen, &app, Layout::wide(), false, coast, period)
                    .expect("NO FRAME render");
                let phase = gate.next_phase();
                cs.set_high();
                let result = sender.send(phase, &screen.pixels, true, |part| spi.write(part));
                cs.set_low();
                if result.is_err() {
                    gate.transfer_failed();
                    panic!("NO FRAME SPI failed; panel state unknown");
                }
                let done = uptime_us();
                gate.no_frame_succeeded(done, phase);
                retained = true;
                println!("LIVE_UTC no_frame_sent uptime_us={done}");
            }
            LifecycleAction::Scheduled(DiagnosticAction::Frame { sample, full }) => {
                // Time is derived from a screened live anchor, not the fixture.
                let utc = DateTime::from_timestamp_micros(sample.utc_unix_us)
                    .expect("gate checked UTC")
                    .naive_utc();
                app.update(utc).expect("checked ISS sample");
                overhead_render::draw_cached(screen, &app, Layout::wide(), false, coast, period)
                    .expect("ISS render");
                let phase = gate.next_phase();
                cs.set_high();
                let result = sender.send(phase, &screen.pixels, full, |part| spi.write(part));
                cs.set_low();
                let transfer = match result {
                    Ok(transfer) => transfer,
                    Err(_) => {
                        gate.transfer_failed();
                        panic!("ISS SPI failed; panel state unknown");
                    }
                };
                let done = uptime_us();
                gate.transfer_succeeded(done, phase, transfer.kind == Kind::Full);
                retained = true;
                frames += 1;
                if frames.is_multiple_of(20) || sample.skipped > 0 || transfer.kind == Kind::Full {
                    println!(
                        "LIVE_UTC frame={frames} utc_us={} skipped={} kind={:?} rows={} total_us={}",
                        sample.utc_unix_us,
                        sample.skipped,
                        transfer.kind,
                        transfer.rows,
                        done - now
                    );
                }
            }
            LifecycleAction::Scheduled(DiagnosticAction::RefreshRetained) => {
                let phase = gate.next_phase();
                cs.set_high();
                let result = sender.send(phase, &screen.pixels, true, |part| spi.write(part));
                cs.set_low();
                if result.is_err() {
                    gate.transfer_failed();
                    panic!("retained SPI failed; panel state unknown");
                }
                let done = uptime_us();
                gate.transfer_succeeded(done, phase, true);
                println!("LIVE_UTC retained_full uptime_us={done}");
            }
            LifecycleAction::Scheduled(DiagnosticAction::IdleVcom) => {
                let phase = gate.next_phase();
                cs.set_high();
                let result = sender.idle(phase, |part| spi.write(part));
                cs.set_low();
                if result.is_err() {
                    gate.transfer_failed();
                    panic!("idle SPI failed; panel state unknown");
                }
                gate.transfer_succeeded(uptime_us(), phase, false);
            }
            LifecycleAction::Scheduled(DiagnosticAction::Wait) => {}
        }
        // Independent of DNS, association and UDP awaits. BOOT sampling does
        // NOT preempt blocking SPI/render, driver calls or a stalled executor;
        // short presses can be missed and selection paints on the next frame.
        Timer::after(Wait::from_millis(5)).await;
    }
}

enum QueryResult {
    NoReply,
    Reply,
    KissOfDeath,
}

async fn query(
    stack: Stack<'static>,
    socket: &mut UdpSocket<'_>,
    rng: &Rng,
    server: &str,
    seq: u32,
) -> QueryResult {
    let Ok(addrs) = stack.dns_query(server, DnsQueryType::A).await else {
        println!("SNTP seq={seq} server={server} outcome=DnsFailed");
        return QueryResult::NoReply;
    };
    let Some(addr) = addrs.first().copied() else {
        return QueryResult::NoReply;
    };
    let peer = IpEndpoint::new(addr, 123);
    let nonce = loop {
        let n = (u64::from(rng.random()) << 32) | u64::from(rng.random());
        if n != 0 {
            break n;
        }
    };
    let mut pending = Some(SntpRequest {
        transmit: nonce,
        sent_uptime_us: 0,
    });
    let sent = uptime_us(); // before handing the request to the stack
    pending.as_mut().unwrap().sent_uptime_us = sent;
    if socket
        .send_to(&sntp_screen::request(nonce), peer)
        .await
        .is_err()
    {
        return QueryResult::NoReply;
    }
    let deadline = embassy_time::Instant::now() + REPLY_TIMEOUT;
    let mut buffer = [0u8; 96];
    loop {
        let remaining = deadline.saturating_duration_since(embassy_time::Instant::now());
        let Ok(Ok((len, meta))) = with_timeout(remaining, socket.recv_from(&mut buffer)).await
        else {
            println!("SNTP seq={seq} server={server} outcome=Timeout");
            return QueryResult::NoReply;
        };
        let received = uptime_us(); // after the stack hands back the reply
        if meta.endpoint != peer {
            continue; // wrong peer: never forward to trust gate
        }
        let Some(rtt) = received.checked_sub(sent) else {
            continue;
        };
        let capture =
            sntp_screen::bracketing_capture_bound_us(rtt, PROTOTYPE_DRIFT_PPM).unwrap_or(u64::MAX);
        let outcome = sntp_screen::screen(&mut pending, &buffer[..len], received, capture);
        if pending.is_some() {
            continue; // unmatched/short: still waiting for the correct reply
        }
        let kod = matches!(outcome, SntpOutcome::KissOfDeath(_));
        println!("SNTP seq={seq} server={server} outcome={outcome:?}");
        EVENTS.send(outcome).await;
        return if kod {
            QueryResult::KissOfDeath
        } else {
            QueryResult::Reply
        };
    }
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    loop {
        match controller.connect_async().await {
            Ok(_) => {
                println!("SNTP_WIFI connected");
                let _ = controller.wait_for_disconnect_async().await;
                println!("SNTP_WIFI disconnected");
            }
            Err(e) => println!("SNTP_WIFI connect_failed {e:?}"),
        }
        Timer::after(Wait::from_secs(5)).await;
    }
}

#[embassy_executor::task]
async fn net(mut runner: Runner<'static, Interface>) {
    runner.run().await
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("OVERHEAD_S3_LIVE_UTC_RADAR_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
