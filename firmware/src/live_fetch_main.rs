#![no_std]
#![no_main]
//! 0071 `live-fetch` diagnostic, NOT the device application. Joins Wi-Fi,
//! screens one SNTP observation (0064, 0065's 100 ms budget), then makes
//! exactly one HTTPS GET of CelesTrak STATIONS through mbedtls-rs and reports
//! TLS, HTTP and catalogue results, timings and the heap high-water mark.
//! Any failure stops fetching until reboot: no retry. The Sharp panel is held
//! white with VCOM alternating ~1 Hz. Credentials come from build-time
//! `SSID`/`PASSWORD` (ignored `.wifi.env`).
//!
//! Trust: chain and hostname are verified against the two bundled roots.
//! Certificate validity dates are NOT checked: the prebuilt MbedTLS library
//! has no wall-clock hook (see the firmware README).
use core::panic::PanicInfo;
use embassy_executor::Spawner;
use embassy_net::{
    IpEndpoint, Runner, Stack, StackResources,
    dns::DnsQueryType,
    tcp::TcpSocket,
    udp::{PacketMetadata, UdpSocket},
};
use embassy_time::{Duration as Wait, Timer, with_timeout};
use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    rng::{Trng, TrngSource},
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
use mbedtls_rs::{
    Certificate, ClientSessionConfig, Session, SessionConfig, Tls, TlsVersion, X509, io::Write,
};
use overhead_s3_benchmark::{
    element_fetch::{self, HttpError},
    sharp_test,
    sntp_screen::{self, ScreenedObservation, SntpOutcome, SntpRequest},
};
use static_cell::StaticCell;

esp_bootloader_esp_idf::esp_app_desc!();

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");
const SNTP_SERVERS: [&str; 2] = ["time.cloudflare.com", "time.nist.gov"];
const DRIFT_PPM: u64 = 20; // 0065
const UTC_BUDGET_US: u64 = 100_000; // 0065
const REPLY_TIMEOUT: Wait = Wait::from_secs(5);
const FETCH_TIMEOUT: Wait = Wait::from_secs(60);
/// Sectigo Public Server Authentication Root R46 and USERTrust RSA
/// Certification Authority (0071), NUL-terminated for MbedTLS PEM parsing.
const ROOTS: &str = concat!(include_str!("../certs/celestrak-roots.pem"), "\0");
const SERVER_NAME: &core::ffi::CStr = c"celestrak.org";
const HEAP_SIZE: usize = 160 * 1024;

static RESOURCES: StaticCell<StackResources<4>> = StaticCell::new();
static RX_META: StaticCell<[PacketMetadata; 4]> = StaticCell::new();
static TX_META: StaticCell<[PacketMetadata; 4]> = StaticCell::new();
static UDP_RX: StaticCell<[u8; 512]> = StaticCell::new();
static UDP_TX: StaticCell<[u8; 512]> = StaticCell::new();
static TCP_RX: StaticCell<[u8; 4096]> = StaticCell::new();
static TCP_TX: StaticCell<[u8; 1024]> = StaticCell::new();
static RESPONSE: StaticCell<[u8; element_fetch::MAX_RESPONSE]> = StaticCell::new();
static TRNG: StaticCell<Trng> = StaticCell::new();

fn uptime_us() -> u64 {
    Instant::now().duration_since_epoch().as_micros()
}

fn heap_peak() -> usize {
    esp_alloc::HEAP.stats().max_usage
}

#[esp_hal::main]
async fn main(spawner: Spawner) -> ! {
    let p = esp_hal::init(esp_hal::Config::default());
    esp_alloc::heap_allocator!(size: HEAP_SIZE);
    let timg0 = TimerGroup::new(p.TIMG0);
    esp_rtos::start(timg0.timer0, p.FROM_CPU_INTR0);
    println!("OVERHEAD_S3_LIVE_FETCH_V1 one STATIONS request per boot; no retry");

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

    // Entropy for TLS: keep the TRNG source alive for the whole boot.
    let _trng_source = TrngSource::new(p.RNG, p.ADC1);
    let trng = TRNG.init(Trng::try_new().expect("TRNG"));
    let seed = (u64::from(trng.random()) << 32) | u64::from(trng.random());

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
        println!("FETCH_NET ip={}", config.address);
    }

    // Trusted UTC first (0071): one screened observation within budget.
    let Some(utc) = trusted_utc(stack, trng).await else {
        println!("FETCH_STOP reason=no_trusted_utc");
        idle().await
    };
    println!(
        "FETCH_UTC utc={:?}Z u0_us={} heap_peak={}",
        utc.utc_at_midpoint,
        utc.uncertainty_us,
        heap_peak()
    );

    let tls = Tls::new(trng).expect("TLS context");
    let response = RESPONSE.init([0; element_fetch::MAX_RESPONSE]);
    let started = uptime_us();
    match with_timeout(FETCH_TIMEOUT, fetch(stack, &tls, response)).await {
        Ok(Ok(len)) => report(&response[..len]),
        Ok(Err(stage)) => println!("FETCH_FAILED {stage}"),
        Err(_) => println!("FETCH_FAILED timeout_s={}", FETCH_TIMEOUT.as_secs()),
    }
    println!(
        "FETCH_DONE total_ms={} heap_peak={} heap_size={HEAP_SIZE}",
        (uptime_us() - started) / 1000,
        heap_peak()
    );
    idle().await
}

/// DNS, TCP, TLS handshake, one GET and read to close. Returns the number of
/// response bytes; the caller parses them.
async fn fetch(
    stack: Stack<'static>,
    tls: &Tls<'_>,
    response: &mut [u8],
) -> Result<usize, &'static str> {
    let addrs = stack
        .dns_query(element_fetch::HOST, DnsQueryType::A)
        .await
        .map_err(|_| "stage=dns")?;
    let addr = *addrs.first().ok_or("stage=dns_empty")?;
    let mut socket = TcpSocket::new(stack, TCP_RX.init([0; 4096]), TCP_TX.init([0; 1024]));
    socket.set_timeout(Some(Wait::from_secs(20)));
    let t0 = uptime_us();
    socket
        .connect((addr, element_fetch::PORT))
        .await
        .map_err(|_| "stage=tcp_connect")?;
    let t1 = uptime_us();

    let config = ClientSessionConfig {
        ca_chain: Some(Certificate::new(X509::PEM(c_str(ROOTS))).map_err(|_| "stage=roots")?),
        server_name: Some(SERVER_NAME),
        min_version: TlsVersion::Tls1_2,
        ..ClientSessionConfig::new()
    };
    let mut session = Session::new(tls.reference(), socket, &SessionConfig::Client(config))
        .map_err(|_| "stage=tls_session")?;
    if let Err(e) = session.connect().await {
        println!(
            "FETCH_TLS ok=false error={e:?} verify=0x{:x} heap_peak={}",
            session.tls_verification_details(),
            heap_peak()
        );
        return Err("stage=tls_handshake");
    }
    let t2 = uptime_us();
    println!(
        "FETCH_TLS ok=true version={:?} tcp_ms={} handshake_ms={} heap_peak={}",
        session.tls_version(),
        (t1 - t0) / 1000,
        (t2 - t1) / 1000,
        heap_peak()
    );

    session
        .write_all(element_fetch::REQUEST.as_bytes())
        .await
        .map_err(|_| "stage=write")?;
    session.flush().await.map_err(|_| "stage=flush")?;
    let mut len = 0;
    loop {
        if len == response.len() {
            println!("FETCH_HTTP error=response_buffer_full bytes={len}");
            return Err("stage=read_overflow");
        }
        match session.read(&mut response[len..]).await {
            Ok(0) => break,
            Ok(n) => len += n,
            // Truncation is caught by the exact Content-Length check.
            Err(e) => {
                println!("FETCH_READ_END error={e:?} bytes={len}");
                break;
            }
        }
        // Stop early on a non-200 or oversized head; never read more.
        if let Err(e) = element_fetch::parse_head(&response[..len]) {
            print_http_error(e);
            return Err("stage=http_head");
        }
    }
    let _ = session.close().await;
    println!(
        "FETCH_TRANSFER bytes={len} transfer_ms={}",
        (uptime_us() - t2) / 1000
    );
    Ok(len)
}

fn report(bytes: &[u8]) {
    let body = match element_fetch::parse_response(bytes) {
        Ok(body) => body,
        Err(e) => return print_http_error(e),
    };
    println!("FETCH_HTTP status=200 body_bytes={}", body.len());
    let t = uptime_us();
    match element_fetch::ingest(body) {
        Ok(ingested) => {
            let s = ingested.summary();
            println!(
                "FETCH_CATALOGUE records={} accepted={} record_errors={} invalid={} \
                 conflicts={} oldest={:?}Z newest={:?}Z ingest_ms={} heap_peak={}",
                s.records,
                s.accepted,
                s.record_errors,
                s.invalid,
                s.conflicts,
                s.oldest_epoch,
                s.newest_epoch,
                (uptime_us() - t) / 1000,
                heap_peak()
            );
            for e in &ingested.record_errors {
                println!("FETCH_RECORD_ERROR record={} {}", e.record, e.message);
            }
        }
        Err(e) => println!("FETCH_CATALOGUE failed {e:?}"),
    }
}

fn print_http_error(e: HttpError) {
    println!("FETCH_HTTP error={e:?}");
}

fn c_str(s: &'static str) -> &'static core::ffi::CStr {
    core::ffi::CStr::from_bytes_with_nul(s.as_bytes()).expect("NUL-terminated roots")
}

/// One screened SNTP observation within the 0065 budget, trying Cloudflare
/// then NIST once each.
async fn trusted_utc(stack: Stack<'static>, trng: &mut Trng) -> Option<ScreenedObservation> {
    let mut socket = UdpSocket::new(
        stack,
        RX_META.init([PacketMetadata::EMPTY; 4]),
        UDP_RX.init([0; 512]),
        TX_META.init([PacketMetadata::EMPTY; 4]),
        UDP_TX.init([0; 512]),
    );
    socket.bind(0).expect("bind");
    for server in SNTP_SERVERS {
        let Some(peer) = stack
            .dns_query(server, DnsQueryType::A)
            .await
            .ok()
            .and_then(|a| a.first().copied())
        else {
            println!("FETCH_SNTP server={server} outcome=DnsFailed");
            continue;
        };
        let peer = IpEndpoint::new(peer, 123);
        let nonce = (u64::from(trng.random()) << 32) | u64::from(trng.random()) | 1;
        let sent = uptime_us();
        let mut pending = Some(SntpRequest {
            transmit: nonce,
            sent_uptime_us: sent,
        });
        if socket
            .send_to(&sntp_screen::request(nonce), peer)
            .await
            .is_err()
        {
            println!("FETCH_SNTP server={server} outcome=SendFailed");
            continue;
        }
        let deadline = embassy_time::Instant::now() + REPLY_TIMEOUT;
        let mut buffer = [0u8; 96];
        loop {
            let remaining = deadline.saturating_duration_since(embassy_time::Instant::now());
            let Ok(Ok((len, meta))) = with_timeout(remaining, socket.recv_from(&mut buffer)).await
            else {
                println!("FETCH_SNTP server={server} outcome=Timeout");
                break;
            };
            let received = uptime_us();
            if meta.endpoint != peer {
                continue;
            }
            let rtt = received.saturating_sub(sent);
            let capture =
                sntp_screen::bracketing_capture_bound_us(rtt, DRIFT_PPM).unwrap_or(u64::MAX);
            let outcome = sntp_screen::screen(&mut pending, &buffer[..len], received, capture);
            if pending.is_some() {
                continue;
            }
            match outcome {
                SntpOutcome::Observation(o) if o.uncertainty_us < UTC_BUDGET_US => {
                    return Some(o);
                }
                // A KoD or out-of-budget reply ends the attempt (no retry).
                other => {
                    println!("FETCH_SNTP server={server} rtt_us={rtt} outcome={other:?}");
                    return None;
                }
            }
        }
    }
    None
}

async fn idle() -> ! {
    loop {
        Timer::after(Wait::from_secs(3600)).await;
    }
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
                println!("FETCH_WIFI connected");
                let _ = controller.wait_for_disconnect_async().await;
                println!("FETCH_WIFI disconnected");
            }
            Err(e) => println!("FETCH_WIFI connect_failed {e:?}"),
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
    println!("OVERHEAD_S3_LIVE_FETCH_FAILED: {info}");
    loop {
        core::hint::spin_loop();
    }
}
