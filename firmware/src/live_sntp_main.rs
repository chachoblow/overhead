#![no_std]
#![no_main]
//! Opt-in live SNTP source-evidence diagnostic, NOT the device application.
//! Joins Wi-Fi, queries one non-smearing server every 64 s (0064 screen,
//! bracketing captures, 0065 D), and logs RTT, U0 and the end-to-end
//! consistency between successive accepted observations. No radar or trust
//! gate on target; the Sharp panel is held white with VCOM alternating ~1 Hz.
//! Credentials come from build-time `SSID`/`PASSWORD` (ignored `.wifi.env`).
use core::panic::PanicInfo;
use embassy_executor::Spawner;
use embassy_net::{
    IpEndpoint, Runner, Stack, StackResources,
    dns::DnsQueryType,
    udp::{PacketMetadata, UdpSocket},
};
use embassy_time::{Duration as Wait, Timer, with_timeout};
use esp_hal::{
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
use overhead_s3_benchmark::{
    sharp_test,
    sntp_screen::{self, ScreenedObservation, SntpOutcome, SntpRequest},
};
use static_cell::StaticCell;

esp_bootloader_esp_idf::esp_app_desc!();

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");
/// Documented non-smearing operators (proposal); never mixed with smearing ones.
const SERVERS: [&str; 2] = ["time.cloudflare.com", "time.nist.gov"];
const DRIFT_PPM: u64 = 20; // 0065
const POLL_S: u64 = 64;
const REPLY_TIMEOUT: Wait = Wait::from_secs(5);

static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
static RX_META: StaticCell<[PacketMetadata; 4]> = StaticCell::new();
static TX_META: StaticCell<[PacketMetadata; 4]> = StaticCell::new();
static RX_BUF: StaticCell<[u8; 512]> = StaticCell::new();
static TX_BUF: StaticCell<[u8; 512]> = StaticCell::new();

fn uptime_us() -> u64 {
    Instant::now().duration_since_epoch().as_micros()
}

#[esp_hal::main]
async fn main(spawner: Spawner) -> ! {
    let p = esp_hal::init(esp_hal::Config::default());
    esp_alloc::heap_allocator!(size: 96 * 1024);
    let timg0 = TimerGroup::new(p.TIMG0);
    esp_rtos::start(timg0.timer0, p.FROM_CPU_INTR0);
    println!("OVERHEAD_S3_LIVE_SNTP_V1 source evidence only; not a trusted device clock");

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
    spawner.spawn(panel(spi, cs).unwrap());

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
    if let Some(config) = stack.config_v4() {
        println!("SNTP_NET ip={}", config.address);
    }
    let mut socket = UdpSocket::new(
        stack,
        RX_META.init([PacketMetadata::EMPTY; 4]),
        RX_BUF.init([0; 512]),
        TX_META.init([PacketMetadata::EMPTY; 4]),
        TX_BUF.init([0; 512]),
    );
    socket.bind(0).expect("bind");

    let mut previous: Option<ScreenedObservation> = None;
    let mut seq: u32 = 0;
    loop {
        seq = seq.wrapping_add(1);
        for server in SERVERS {
            if query(stack, &mut socket, &rng, server, seq, &mut previous).await {
                break;
            }
        }
        Timer::after(Wait::from_secs(POLL_S)).await;
    }
}

/// One request/reply exchange; `true` if a reply was screened (any outcome
/// other than timeout/DNS failure), so the fallback is not tried.
async fn query(
    stack: Stack<'static>,
    socket: &mut UdpSocket<'_>,
    rng: &Rng,
    server: &str,
    seq: u32,
    previous: &mut Option<ScreenedObservation>,
) -> bool {
    let Ok(addrs) = stack.dns_query(server, DnsQueryType::A).await else {
        println!("SNTP seq={seq} server={server} outcome=DnsFailed");
        return false;
    };
    let Some(addr) = addrs.first().copied() else {
        println!("SNTP seq={seq} server={server} outcome=DnsEmpty");
        return false;
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
    // Bracketing capture: read before handing the packet to the stack.
    let sent = uptime_us();
    if let Some(request) = pending.as_mut() {
        request.sent_uptime_us = sent;
    }
    if socket
        .send_to(&sntp_screen::request(nonce), peer)
        .await
        .is_err()
    {
        println!("SNTP seq={seq} server={server} outcome=SendFailed");
        return false;
    }
    let deadline = embassy_time::Instant::now() + REPLY_TIMEOUT;
    let mut buffer = [0u8; 96];
    loop {
        let remaining = deadline.saturating_duration_since(embassy_time::Instant::now());
        let Ok(Ok((len, meta))) = with_timeout(remaining, socket.recv_from(&mut buffer)).await
        else {
            println!("SNTP seq={seq} server={server} outcome=Timeout");
            return false;
        };
        // ...and after the stack returns the reply.
        let received = uptime_us();
        if meta.endpoint != peer {
            continue; // not from the queried endpoint: ignore, keep waiting
        }
        let rtt = received.saturating_sub(sent);
        let capture = sntp_screen::bracketing_capture_bound_us(rtt, DRIFT_PPM).unwrap_or(u64::MAX);
        let outcome = sntp_screen::screen(&mut pending, &buffer[..len], received, capture);
        if pending.is_some() {
            continue; // unmatched/malformed: still waiting for ours
        }
        match outcome {
            SntpOutcome::Observation(o) => {
                let utc_us = o.utc_at_midpoint.and_utc().timestamp_micros();
                print_observation(seq, server, rtt, &o, utc_us);
                if let Some(earlier) = *previous {
                    match sntp_screen::consistency(earlier, o, DRIFT_PPM) {
                        Some((diff, bound)) => println!(
                            "SNTP_CHECK seq={seq} diff_us={diff} bound_us={bound} within={}",
                            diff.unsigned_abs() <= bound
                        ),
                        None => println!("SNTP_CHECK seq={seq} invalid"),
                    }
                }
                *previous = Some(o);
            }
            other => println!("SNTP seq={seq} server={server} rtt_us={rtt} outcome={other:?}"),
        }
        return true;
    }
}

fn print_observation(seq: u32, server: &str, rtt: u64, o: &ScreenedObservation, utc_us: i64) {
    println!(
        "SNTP seq={seq} server={server} outcome=Observation rtt_us={rtt} u0_us={} \
         mid_uptime_us={} utc_us={utc_us} budget_ok={}",
        o.uncertainty_us,
        o.midpoint_uptime_us,
        o.uncertainty_us < 100_000
    );
}

#[embassy_executor::task]
async fn panel(mut spi: Spi<'static, esp_hal::Blocking>, mut cs: Output<'static>) {
    let mut vcom = false;
    loop {
        cs.set_high();
        sharp_test::clear(vcom, |bytes| spi.write(bytes)).expect("Sharp VCOM");
        cs.set_low();
        vcom = !vcom;
        Timer::after(Wait::from_secs(1)).await;
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
    println!("OVERHEAD_S3_LIVE_SNTP_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
