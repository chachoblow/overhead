//! Live-UTC radar runtime on fetched elements (0074, refreshed under 0075;
//! CelesTrak's `visual` group under the 0077 operating budget), shared by the device application (`device`, 0076: event log only)
//! and the `live-fetch-radar` diagnostic (`Log::Diagnostic`: every telemetry
//! line). The two differ only in what they print. One display task owns app,
//! UTC gate, BOOT selection and SPI; the network side screens SNTP replies,
//! fetches elements over HTTPS after the first in-budget observation, then
//! again on 0075's schedule (`element_refresh`: one interval after a success
//! or anything past the request, short backoff only before it), and hands
//! each finished catalogue over with its fetch UTC. No fixture elements are
//! ever shown: until elements arrive the radar reports `NO DATA`; a failed
//! refresh keeps tracking the previous catalogue, flagged `OLD DATA` after
//! 24 h and unloaded once its newest epoch is 7 days old.
//! Observer: the build-time home site (`site`, ignored `.site.env`).
//!
//! Persistence (0079): each successful fetch's body and fetch UTC are saved,
//! one flash sector per step, to the `elements` partition (`element_store`,
//! `partitions.csv`). At boot the newest valid copy is loaded before Wi-Fi is
//! up (still `NO FRAME` until SNTP); at the first in-budget observation a copy
//! younger than the refresh interval defers the boot fetch to the end of its
//! interval, otherwise the fetch is due as before. Without the partition the
//! runtime fetches on every boot as before. Off in the fault build.
//!
//! Budget (0077): at most `OBJECT_LIMIT` objects per load (lowest NORAD IDs
//! kept, dropped count logged); each frame recomputes the selected object and
//! a round-robin share of the rest so every marker is under `STAGGER` old.
//! Heap: PSRAM is the first region, so the catalogue, TLS and the response
//! buffer live there; internal RAM serves the radio's internal-only requests.
//!
//! Trust: MbedTLS verifies chain and hostname against the two bundled roots;
//! every served certificate's validity dates are checked against the screened
//! UTC in `tls_dates` before the request is sent (0074).
//!
//! With `live-fetch-faults` the same image follows a build-time fault plan
//! (`fault_plan`): chosen attempts fail at their real DNS, TCP or TLS stage,
//! `OLD DATA` and expiry thresholds are shortened, and Wi-Fi is dropped once,
//! so 0075's failure handling can be captured in one run. Without the feature
//! every attempt is ordinary and the thresholds are 0075's.
//!
//! SNTP Kiss-o'-Death (0080): each server backs off on its own
//! (`sntp_policy`): `RATE` doubles its interval up to an hour, `DENY`/`RSTR`
//! leaves it alone for 24 h, and the next server is asked meanwhile. Fetches
//! follow the schedule on every poll whether or not a server answered, dated
//! by the newest observation still trusted under 0065's holdover. The fault
//! build can inject kiss codes per server and time window (`FAULT_KOD`).
use crate::{
    boot_capture,
    button_input::apply_presses,
    coast_measurement::Screen,
    device_timing::{DiagnosticAction, LifecycleAction},
    element_fetch::{self, HttpError},
    element_refresh::{self, Outcome, Schedule},
    element_store::{self, Did, Layout as Slots, Target, Writer},
    fault_plan::Fault,
    frame_stats::{CAUSES, Cause, FrameStats},
    ota_health::Check,
    knob::{self, Detents},
    live_utc,
    sharp_lines::{Kind, LineSender},
    site,
    sntp_policy::{self, Kiss, Policy},
    sntp_screen::{self, ScreenedObservation, SntpOutcome, SntpRequest},
    stack_watermark, tls_dates,
    utc_trust::{PROTOTYPE_DRIFT_PPM, UtcTrustPreflight},
};
use core::{
    mem::MaybeUninit,
    ops::Range,
    sync::atomic::{AtomicBool, Ordering},
};
use embassy_executor::Spawner;
use embassy_net::{
    IpEndpoint, Runner, Stack, StackResources,
    dns::DnsQueryType,
    tcp::TcpSocket,
    udp::{PacketMetadata, UdpSocket},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration as Wait, Timer, with_timeout};
use esp_hal::{
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    rtc_cntl::{Rwdt, RwdtStage, RwdtStageAction},
    pcnt::{Pcnt, channel, unit::Unit},
    peripherals::Peripherals,
    rng::{Trng, TrngSource},
    spi::{
        BitOrder, Mode,
        master::{Config as SpiConfig, Spi},
    },
    time::{Instant, Rate},
    timer::{PeriodicTimer, timg::TimerGroup},
};
use esp_println::println;
use esp_radio::wifi::{
    AuthenticationMethodConfig, Config, ControllerConfig, Interface, WifiController,
    sta::StationConfig,
};
use esp_storage::{FlashStorage, FlashStorageError};
use mbedtls_rs::{
    Certificate, ClientSessionConfig, Session, SessionConfig, Tls, TlsVersion, X509, io,
    io::Write as _,
};
use overhead_app::{Freshness, RadarApp, projection::View};
use overhead_core::{
    GeodeticPosition,
    catalogue::Catalogue,
    sgp4::chrono::{DateTime, NaiveDateTime, TimeDelta},
};
use overhead_render::{CoastLayer, Layout, PeriodCache};
use static_cell::{ConstStaticCell, StaticCell};

/// What the runtime prints over UART (0076). Blocking prints sit in the
/// display loop, so the device app keeps only rare event lines.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Log {
    /// Device app: boot, Wi-Fi, fetch outcomes and schedule, catalogue
    /// load/`OLD DATA`/expiry, no-frame renders, failures and panics.
    Events,
    /// Diagnostic: also per-poll SNTP, fetch stages, heap/stack, BOOT presses,
    /// loop statistics and sampled frames.
    Diagnostic,
}

static DIAGNOSTIC: AtomicBool = AtomicBool::new(false);
/// Set by every diagnostic print, so frame accounting can tell when blocking
/// UART output (absent from the device app) delayed the display loop.
static LOGGED: AtomicBool = AtomicBool::new(false);

/// Prints only in `Log::Diagnostic`; event lines use `println!` directly.
macro_rules! diag {
    ($($arg:tt)*) => {
        if DIAGNOSTIC.load(Ordering::Relaxed) {
            println!($($arg)*);
            LOGGED.store(true, Ordering::Relaxed);
        }
    };
}

#[path = "live_ota.rs"]
mod ota;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");
const SERVERS: [&str; 2] = ["time.cloudflare.com", "time.nist.gov"];
const POLL_S: u64 = sntp_policy::POLL_S;
const REPLY_TIMEOUT: Wait = Wait::from_secs(5);
const FETCH_TIMEOUT: Wait = Wait::from_secs(60);
/// 0065's total-error budget, required of the observation that dates the fetch.
const UTC_BUDGET_US: u64 = 100_000;
/// Vertical FOV of the wide view: wider than the ISS image's 5° so several
/// objects can share the view for BOOT cycling (0035 labels stay on).
const FOV_DEG: f64 = 60.0;
/// Internal-RAM heap. Before PSRAM (0077) one 160 KiB heap held everything
/// and peaked at ~137 KB with 20 STATIONS objects.
#[cfg(not(feature = "ble-spike"))]
const HEAP_SIZE: usize = 160 * 1024;
/// BLE spike: the controller adds ~33 KB of statics (mostly IRAM code in the
/// shared D/IRAM), so the static heap shrinks and the bootloader-reclaimed
/// `dram2` region (unused until now) becomes a second internal region.
#[cfg(feature = "ble-spike")]
const HEAP_SIZE: usize = 128 * 1024;
/// 0077: most objects tracked from one load; ~1.6× the 156-object `visual`
/// group of 2026-11-03.
const OBJECT_LIMIT: usize = 256;
/// 0077: oldest an unselected object's marker may be.
const STAGGER: TimeDelta = TimeDelta::seconds(1);
/// Sectigo Public Server Authentication Root R46 and USERTrust RSA
/// Certification Authority (0071), NUL-terminated for MbedTLS PEM parsing.
const ROOTS: &str = concat!(include_str!("../certs/celestrak-roots.pem"), "\0");
const SERVER_NAME: &core::ffi::CStr = c"celestrak.org";
/// 0086: RWDT timeout; the display loop feeds it every pass (~5 ms).
const WDT_S: u64 = 20;
/// The fault build keeps its per-attempt plan: no stored catalogue at boot.
const PERSIST: bool = !cfg!(feature = "live-fetch-faults");
/// Pause after each save step (one sector erase or program, interrupts off)
/// so the display loop runs between them.
const STORE_STEP_YIELD: Wait = Wait::from_millis(10);
/// Wait before a save so the display applies the new catalogue first: that
/// frame's full update (~270 ms with 156 objects) is `Load` cost, not flash.
const STORE_SETTLE: Wait = Wait::from_secs(2);

/// Fault build only: a public root (ISRG Root X1) that is not in CelesTrak's
/// chain, so the `verify` fault fails real chain verification.
#[cfg(feature = "live-fetch-faults")]
const WRONG_ROOT: &str = concat!(include_str!("../certs/fault-wrong-root.pem"), "\0");
/// `.invalid` never resolves (RFC 2606): a real DNS failure, no CelesTrak query.
#[cfg(feature = "live-fetch-faults")]
const FAULT_DNS_HOST: &str = "overhead-fault.invalid";
/// TEST-NET-1 (RFC 5737) is never routed: a real TCP connect timeout.
#[cfg(feature = "live-fetch-faults")]
const FAULT_TCP_ADDR: embassy_net::IpAddress = embassy_net::IpAddress::v4(192, 0, 2, 1);

/// Fault build configuration, all build-time and checked at boot.
#[cfg(feature = "live-fetch-faults")]
struct Faults {
    plan: crate::fault_plan::Plan,
    stale_s: Option<u64>,
    expire_after_load_s: Option<u64>,
    wifi_drop: Option<(u64, u64)>,
    kod: crate::fault_plan::KodPlan,
    kod_block_s: Option<u64>,
}

#[cfg(feature = "live-fetch-faults")]
fn faults() -> Faults {
    use crate::fault_plan::{Plan, minutes};
    let check = |r: Result<Option<u64>, &'static str>| match r {
        Ok(v) => v,
        Err(name) => panic!("fault configuration: {name} must be whole minutes > 0"),
    };
    let plan = match Plan::parse(option_env!("FAULT_PLAN")) {
        Ok(plan) => plan,
        Err(e) => panic!("fault configuration: {e}"),
    };
    let drop_at = check(minutes(
        option_env!("FAULT_WIFI_DROP_AT_MIN"),
        "FAULT_WIFI_DROP_AT_MIN",
    ));
    let drop_for = check(minutes(
        option_env!("FAULT_WIFI_DROP_FOR_MIN"),
        "FAULT_WIFI_DROP_FOR_MIN",
    ));
    Faults {
        plan,
        stale_s: check(minutes(option_env!("FAULT_STALE_MIN"), "FAULT_STALE_MIN")),
        expire_after_load_s: check(minutes(
            option_env!("FAULT_EXPIRE_AFTER_LOAD_MIN"),
            "FAULT_EXPIRE_AFTER_LOAD_MIN",
        )),
        wifi_drop: drop_at.map(|at| (at, drop_for.unwrap_or(600))),
        kod: match crate::fault_plan::KodPlan::parse(option_env!("FAULT_KOD"), SERVERS.len()) {
            Ok(kod) => kod,
            Err(e) => panic!("fault configuration: {e}"),
        },
        kod_block_s: check(minutes(
            option_env!("FAULT_KOD_BLOCK_MIN"),
            "FAULT_KOD_BLOCK_MIN",
        )),
    }
}

/// 0080: kiss code to inject in place of a query (fault build only).
fn kod_for(server: usize) -> Option<[u8; 4]> {
    #[cfg(feature = "live-fetch-faults")]
    return faults().kod.for_query(server, uptime_s());
    #[cfg(not(feature = "live-fetch-faults"))]
    {
        let _ = server;
        None
    }
}

/// 0080: how long a `DENY`/`RSTR` server is left alone.
fn kod_block_s() -> u64 {
    #[cfg(feature = "live-fetch-faults")]
    if let Some(s) = faults().kod_block_s {
        return s;
    }
    sntp_policy::DENY_BLOCK_S
}

fn fault_for(attempt: u32) -> Fault {
    #[cfg(feature = "live-fetch-faults")]
    return faults().plan.for_attempt(attempt);
    #[cfg(not(feature = "live-fetch-faults"))]
    {
        let _ = attempt;
        Fault::None
    }
}

// A successful send is never dropped; if full, network yields until the
// display owner drains it. Processing time, not packet time, is used for age.
static EVENTS: Channel<CriticalSectionRawMutex, SntpOutcome, 4> = Channel::new();
// One catalogue per successful fetch, with the trusted UTC it was fetched at.
static DATA: Channel<CriticalSectionRawMutex, (Catalogue, NaiveDateTime), 1> = Channel::new();
// Set by the network task around the whole fetch so the display owner can
// report its longest loop gap while TLS work shares the executor.
static FETCHING: AtomicBool = AtomicBool::new(false);
// Set by the network task while a save is in progress (0079).
static STORING: AtomicBool = AtomicBool::new(false);
/// Internal-RAM staging for every flash transfer: PSRAM sits behind the cache
/// that shares the MSPI bus with the ROM flash routines, so the response
/// buffer is never handed to them directly. Also holds the partition table.
#[repr(align(4))]
struct Bounce([u8; element_store::SECTOR as usize]);
static BOUNCE: ConstStaticCell<Bounce> =
    ConstStaticCell::new(Bounce([0; element_store::SECTOR as usize]));
static RESOURCES: StaticCell<StackResources<4>> = StaticCell::new();
static RX_META: StaticCell<[PacketMetadata; 4]> = StaticCell::new();
static TX_META: StaticCell<[PacketMetadata; 4]> = StaticCell::new();
static RX_BUF: StaticCell<[u8; 512]> = StaticCell::new();
static TX_BUF: StaticCell<[u8; 512]> = StaticCell::new();
// Const-initialized in .bss: `StaticCell::init([0; N])` would first build the
// array on the stack, which overflowed the guard on the first board run.
static TCP_RX: ConstStaticCell<[u8; 4096]> = ConstStaticCell::new([0; 4096]);
static TCP_TX: ConstStaticCell<[u8; 1024]> = ConstStaticCell::new([0; 1024]);
static CAPTURE: ConstStaticCell<[u8; tls_dates::CAPTURE]> =
    ConstStaticCell::new([0; tls_dates::CAPTURE]);
static TRNG: StaticCell<Trng> = StaticCell::new();
// MbedTLS keeps its RNG for the program's lifetime; a second TRNG handle on the
// same live source avoids sharing the network task's nonce generator.
static TLS_TRNG: StaticCell<Trng> = StaticCell::new();
// Display task is the sole owner; avoid putting large buffers/layer in its
// async future or on the small S3 stack.
static mut SCREEN: Screen = Screen::new();
static mut SENDER: LineSender = LineSender::new();
static mut LAYER: MaybeUninit<CoastLayer> = MaybeUninit::uninit();
static mut PERIOD: PeriodCache = PeriodCache::new();
// ~8 KiB of histograms; static so it never sits on the CPU0 stack.
static mut FRAME_STATS: FrameStats = FrameStats::new();
/// `FRAME_BOOT` (since-boot per-cause totals) every this many 10 s windows.
const FRAME_BOOT_WINDOWS: u32 = 60;

fn uptime_us() -> u64 {
    Instant::now().duration_since_epoch().as_micros()
}

fn heap_peak() -> usize {
    esp_alloc::HEAP.stats().max_usage
}

/// Live allocation now; across refreshes it separates a leak from a peak.
fn heap_now() -> usize {
    esp_alloc::HEAP.stats().current_usage
}

/// (internal used, internal free, PSRAM used) now, from the region stats.
fn heap_regions() -> (usize, usize, usize) {
    let stats = esp_alloc::HEAP.stats();
    let (mut internal, mut free, mut external) = (0, 0, 0);
    for region in stats.region_stats.iter().flatten() {
        if region
            .capabilities
            .contains(esp_alloc::MemoryCapability::Internal)
        {
            internal += region.used;
            free += region.free;
        } else {
            external += region.used;
        }
    }
    (internal, free, external)
}

fn uptime_s() -> u64 {
    uptime_us() / 1_000_000
}

/// Home site from the build environment; a bad value stops the boot before
/// any request rather than tracking from the wrong place.
fn home_site() -> GeodeticPosition {
    match site::parse(
        env!("SITE_LAT_DEG"),
        env!("SITE_LON_DEG"),
        option_env!("SITE_ALT_M"),
    ) {
        Ok(site) => site,
        Err(e) => panic!("site configuration: {e}"),
    }
}

/// Buffers and TLS context created once and reused by every fetch:
/// `ConstStaticCell::take` and `Tls::new` may each succeed only once.
struct FetchResources {
    tls: Tls<'static>,
    tcp_rx: &'static mut [u8],
    tcp_tx: &'static mut [u8],
    response: &'static mut [u8],
    capture: &'static mut [u8],
    store: Option<Store>,
    /// The one flash handle: elements, settings, otadata and app slots.
    flash: EspFlash,
}

impl FetchResources {
    /// The network pieces an OTA exchange borrows (the response buffer is
    /// free between element fetches).
    fn net(&mut self, stack: Stack<'static>) -> (ota::Net<'_>, &mut EspFlash) {
        (
            ota::Net {
                stack,
                tls: &self.tls,
                tcp_rx: &mut *self.tcp_rx,
                tcp_tx: &mut *self.tcp_tx,
                buf: &mut *self.response,
                capture: &mut *self.capture,
            },
            &mut self.flash,
        )
    }
}

/// esp-storage through the internal bounce buffer.
struct EspFlash {
    storage: FlashStorage<'static>,
    bounce: &'static mut Bounce,
}

impl element_store::Flash for EspFlash {
    type Error = FlashStorageError;

    fn read(&mut self, offset: u32, buf: &mut [u8]) -> Result<(), Self::Error> {
        let mut at = offset;
        for chunk in buf.chunks_mut(self.bounce.0.len()) {
            let staged = &mut self.bounce.0[..chunk.len()];
            self.storage.read_nor(at, staged)?;
            chunk.copy_from_slice(staged);
            at += chunk.len() as u32;
        }
        Ok(())
    }

    fn erase_sector(&mut self, offset: u32) -> Result<(), Self::Error> {
        self.storage.erase(offset, offset + element_store::SECTOR)
    }

    fn write(&mut self, offset: u32, data: &[u8]) -> Result<(), Self::Error> {
        let staged = &mut self.bounce.0[..data.len()];
        staged.copy_from_slice(data);
        self.storage.write_nor(offset, staged)
    }
}

/// The open `elements` partition and where the next save goes.
struct Store {
    layout: Slots,
    next: Target,
}

/// The `elements` data partition's slots; `Err` names why persistence is off.
fn open_store(elements: Option<(u32, u32)>) -> Result<Slots, &'static str> {
    let (offset, len) = elements.ok_or("no_elements_partition")?;
    let layout = Slots::new(offset, len).map_err(|_| "partition_layout")?;
    diag!(
        "STORE partition offset=0x{offset:x} len=0x{len:x} slot_capacity={}",
        layout.capacity()
    );
    Ok(layout)
}

/// Loads the newest valid stored copy into `buf` and hands its catalogue to
/// the display owner; returns the open store and the copy's fetch UTC.
async fn load_stored(
    flash: &mut EspFlash,
    elements: Option<(u32, u32)>,
    buf: &mut [u8],
) -> (Option<Store>, Option<i64>) {
    let layout = match open_store(elements) {
        Ok(opened) => opened,
        Err(reason) => {
            println!("STORE unavailable reason={reason}; fetching every boot");
            return (None, None);
        }
    };
    let started = uptime_us();
    let report = element_store::load(flash, &layout, buf);
    let read_ms = (uptime_us() - started) / 1000;
    diag!(
        "STORE scan slot0={:?} slot1={:?} read_ms={read_ms}",
        report.slots[0],
        report.slots[1]
    );
    let next = report.next_target();
    let store = Some(Store { layout, next });
    let Some(loaded) = report.loaded else {
        println!(
            "STORE empty slot0={:?} slot1={:?}",
            report.slots[0], report.slots[1]
        );
        return (store, None);
    };
    let h = loaded.header;
    let Some(fetched_at) = DateTime::from_timestamp(h.fetched_unix_s, 0).map(|t| t.naive_utc())
    else {
        println!("STORE rejected slot={} reason=fetch_time", loaded.slot);
        return (store, None);
    };
    let Some(catalogue) = ingest_body(&buf[..h.body_len as usize]) else {
        println!("STORE rejected slot={} reason=ingest", loaded.slot);
        return (store, None);
    };
    println!(
        "STORE loaded slot={} seq={} fetched_at={fetched_at:?}Z body_bytes={} load_ms={}",
        loaded.slot,
        h.sequence,
        h.body_len,
        (uptime_us() - started) / 1000
    );
    DATA.send((catalogue, fetched_at)).await;
    (store, Some(h.fetched_unix_s))
}

/// Saves a fetched body, one sector step at a time with the display running
/// between steps. A failure leaves the previous copy as the newest valid one.
async fn save(store: &mut Store, flash: &mut EspFlash, body: &[u8], fetched_unix_s: i64) {
    let mut writer = match Writer::new(&store.layout, store.next, body, fetched_unix_s) {
        Ok(w) => w,
        Err(e) => {
            println!("STORE_FAILED {e:?}");
            return;
        }
    };
    Timer::after(STORE_SETTLE).await;
    STORING.store(true, Ordering::Relaxed);
    let started = uptime_us();
    let (mut erase_max_us, mut write_max_us) = (0u64, 0u64);
    while !writer.done() {
        let t = uptime_us();
        let result = writer.step(flash, body);
        let took = uptime_us() - t;
        match result {
            Ok(Did::Erase) => erase_max_us = erase_max_us.max(took),
            Ok(Did::Write) => write_max_us = write_max_us.max(took),
            Err(e) => {
                STORING.store(false, Ordering::Relaxed);
                println!("STORE_FAILED slot={} error={e:?}", store.next.slot);
                return;
            }
        }
        Timer::after(STORE_STEP_YIELD).await;
    }
    STORING.store(false, Ordering::Relaxed);
    let h = writer.header();
    println!(
        "STORE saved slot={} seq={} body_bytes={} steps={} total_ms={} erase_max_us={erase_max_us} \
         write_max_us={write_max_us}",
        store.next.slot,
        h.sequence,
        h.body_len,
        writer.steps(),
        (uptime_us() - started) / 1000
    );
    store.next = store.next.after();
}

/// Runs the radar forever. `p` comes from `esp_hal::init` at 240 MHz; `banner`
/// names the image in the first UART line.
pub async fn run(spawner: Spawner, p: Peripherals, log: Log, banner: &str) -> ! {
    DIAGNOSTIC.store(log == Log::Diagnostic, Ordering::Relaxed);
    // 0077: PSRAM first, so ordinary allocations go there; esp-radio and
    // esp-rtos ask for internal memory explicitly. Nothing here allocates
    // atomics, which must not live in PSRAM on the S3.
    esp_alloc::psram_allocator!(p.PSRAM, esp_hal::psram);
    esp_alloc::heap_allocator!(size: HEAP_SIZE);
    // 0083: 0x3FCDB700..0x3FCED710 (73,744 B) is free once the bootloader
    // (espflash's or 0086's, same layout) is done.
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 72 * 1024);
    // This image's .bss leaves ~21 KiB of CPU0 stack (the fetch-only image
    // had ~65 KiB), so record the written depth through the fetch.
    // SAFETY: CPU0 linker stack, painted before the scheduler starts; esp-rtos
    // threads (Wi-Fi) run on their own heap stacks. Scanned once, below.
    let painted = unsafe { stack_watermark::paint() };
    let timg0 = TimerGroup::new(p.TIMG0);
    esp_rtos::start(timg0.timer0, p.FROM_CPU_INTR0);
    let observer = home_site();
    let interval_s = match element_refresh::interval_from_minutes(option_env!("REFRESH_MINUTES")) {
        Ok(s) => s,
        Err(e) => panic!("refresh configuration: {e}"),
    };
    // The site itself is never printed: logs may be shared, coordinates not.
    println!("{banner}; refresh_s={interval_s}; home site");
    #[cfg(feature = "live-fetch-faults")]
    let wifi_drop = {
        let f = faults();
        println!(
            "FAULT_CONFIG plan={:?} stale_s={:?} expire_after_load_s={:?} wifi_drop_at_for_s={:?} \
             kod={:?} kod_block_s={:?}; diagnostic fault build",
            f.plan.steps(),
            f.stale_s,
            f.expire_after_load_s,
            f.wifi_drop,
            f.kod.windows(),
            f.kod_block_s
        );
        f.wifi_drop
    };
    #[cfg(not(feature = "live-fetch-faults"))]
    let wifi_drop: Option<(u64, u64)> = None;

    // One flash handle for every partition; 0086's boot decision before the
    // display starts, so probation is known from the first frame.
    let mut flash = EspFlash {
        storage: FlashStorage::new(p.FLASH),
        bounce: BOUNCE.take(),
    };
    let parts = match ota::read_parts(&mut flash) {
        Ok(parts) => parts,
        Err(e) => panic!("partition table: {e}"),
    };
    let mut ota = ota::Ota::boot(&mut flash, &parts);
    let hang_drill = ota.verifying() && ota::drill() == ota::Drill::Hang;
    // 0086: RWDT resets the whole chip if the display loop stops feeding it
    // (the loop also stops feeding when the network loop stalls).
    let mut wdt = esp_hal::rtc_cntl::Rtc::new(p.RTC_TIMER).rwdt;
    wdt.set_stage_action(RwdtStage::Stage0, RwdtStageAction::ResetSystem);
    wdt.set_timeout(RwdtStage::Stage0, esp_hal::time::Duration::from_secs(WDT_S));
    wdt.enable();
    ota::NET_BEAT.store(uptime_s() as u32, Ordering::Relaxed);

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
    // 0070: GPIO0 (DevKitC-1 BOOT strap) is sampled by a 5 ms TIMG1
    // interrupt, independent of render/SPI and the executor.
    let boot = Input::new(p.GPIO0, InputConfig::default().with_pull(Pull::Up));
    let timg1 = TimerGroup::new(p.TIMG1);
    boot_capture::start(PeriodicTimer::new(timg1.timer0), boot, 20_000);
    diag!("BOOT_CAPTURE_READY live fetch sample_ms=5 timer=TIMG1");
    // Knob zoom (0060): PCNT unit 0 decodes A=IO4/B=IO5 quadrature in
    // hardware, so render/SPI stalls cannot drop steps; the display task
    // reads the count once per loop. Wiring and polarity: firmware README.
    let pull_up = InputConfig::default().with_pull(Pull::Up);
    let knob_a = Input::new(p.GPIO4, pull_up);
    let knob_b = Input::new(p.GPIO5, pull_up);
    let pcnt = Pcnt::new(p.PCNT);
    let knob_unit = pcnt.unit0;
    knob_unit.set_filter(Some(1023)).expect("PCNT filter");
    knob_unit
        .set_high_limit(Some(knob::LIMIT))
        .expect("PCNT high limit");
    knob_unit
        .set_low_limit(Some(-knob::LIMIT))
        .expect("PCNT low limit");
    knob_unit.clear();
    let ch0 = &knob_unit.channel0;
    ch0.set_ctrl_signal(knob_a.peripheral_input());
    ch0.set_edge_signal(knob_b.peripheral_input());
    ch0.set_ctrl_mode(channel::CtrlMode::Reverse, channel::CtrlMode::Keep);
    ch0.set_input_mode(channel::EdgeMode::Increment, channel::EdgeMode::Decrement);
    let ch1 = &knob_unit.channel1;
    ch1.set_ctrl_signal(knob_b.peripheral_input());
    ch1.set_edge_signal(knob_a.peripheral_input());
    ch1.set_ctrl_mode(channel::CtrlMode::Reverse, channel::CtrlMode::Keep);
    ch1.set_input_mode(channel::EdgeMode::Decrement, channel::EdgeMode::Increment);
    knob_unit.resume();
    diag!(
        "KNOB_READY pcnt=unit0 a=IO4 b=IO5 counts_per_detent={}",
        knob::COUNTS_PER_DETENT
    );
    let knob = KnobInput {
        unit: knob_unit,
        _a: knob_a,
        _b: knob_b,
    };
    spawner.spawn(display(spi, cs, observer, knob, wdt, hang_drill).unwrap());

    // Entropy for TLS, DHCP and SNTP nonces: keep the TRNG source alive.
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
    spawner.spawn(connection(controller, wifi_drop).unwrap());
    // Throwaway coexistence spike (giftable proposal, F); never a device image.
    #[cfg(feature = "ble-spike")]
    crate::ble_spike::start(spawner, p.BT);
    // In PSRAM: the 0077 body cap is too large for internal RAM. Also the
    // load buffer for the stored copy (0079), before Wi-Fi is up.
    let response = alloc::vec![0u8; element_fetch::MAX_RESPONSE].leak();
    let (store, mut stored_at) = if PERSIST {
        load_stored(&mut flash, parts.elements, response).await
    } else {
        (None, None)
    };
    stack.wait_config_up().await;
    let mut socket = UdpSocket::new(
        stack,
        RX_META.init([PacketMetadata::EMPTY; 4]),
        RX_BUF.init([0; 512]),
        TX_META.init([PacketMetadata::EMPTY; 4]),
        TX_BUF.init([0; 512]),
    );
    socket.bind(0).expect("SNTP UDP bind");
    let mut resources = FetchResources {
        tls: Tls::new(TLS_TRNG.init(Trng::try_new().expect("TLS TRNG"))).expect("TLS context"),
        tcp_rx: TCP_RX.take(),
        tcp_tx: TCP_TX.take(),
        response,
        capture: CAPTURE.take(),
        store,
        flash,
    };
    let mut schedule = Schedule::new(interval_s);
    let mut attempts: u32 = 0;
    let mut painted = Some(painted);
    let mut seq: u32 = 0;
    // 0080: per-server KoD back-off, and the newest in-budget observation,
    // which dates fetches while 0065's holdover still trusts it.
    let mut policy = Policy::<{ SERVERS.len() }>::new(kod_block_s());
    let trust = UtcTrustPreflight::prototype();
    let mut latest: Option<ScreenedObservation> = None;
    let mut deferred = false;
    loop {
        ota::NET_BEAT.store(uptime_s() as u32, Ordering::Relaxed);
        stack.wait_config_up().await;
        ota::NET_BEAT.store(uptime_s() as u32, Ordering::Relaxed);
        ota.note(Check::WifiJoined);
        if ota::FRAME_SHOWN.load(Ordering::Relaxed) {
            ota.note(Check::Frame);
        }
        if ota::CATALOGUE_LOADED.load(Ordering::Relaxed) {
            ota.note(Check::CatalogueLoaded);
        }
        if ota.verifying() && ota::drill() == ota::Drill::Panic {
            panic!("OTA drill: panic on probation");
        }
        seq = seq.wrapping_add(1);
        let mut observation = None;
        for (i, server) in SERVERS.into_iter().enumerate() {
            let asked = uptime_s();
            if !policy.due(i, asked) {
                continue;
            }
            match query(stack, &mut socket, trng, i, server, seq).await {
                QueryResult::NoReply => policy.no_reply(i, asked),
                QueryResult::KissOfDeath(code) => {
                    let (kiss, wait) = policy.kiss(i, asked, code);
                    let action = match kiss {
                        Kiss::Rate => "backoff",
                        Kiss::Deny => "block",
                    };
                    println!(
                        "SNTP KoD server={server} code={} action={action} next_in_s={wait} \
                         interval_s={} uptime_s={asked}",
                        core::str::from_utf8(&code).unwrap_or("?"),
                        policy.interval_s(i)
                    );
                }
                QueryResult::Reply(o) => {
                    policy.replied(i, asked);
                    observation = o.filter(|o| o.uncertainty_us < UTC_BUDGET_US);
                    break;
                }
            }
            // No reply or a KoD: the next server, if due (0080 item 2).
        }
        if let Some(o) = observation {
            // 0082: date fetches by the tighter of the two, like the gate.
            latest = Some(trust.tighter(latest, o, uptime_us()));
            if let Some(fetched) = stored_at {
                // 0079: date the stored copy once, with fresh trusted UTC.
                stored_at = None;
                let now_unix = trusted_now_unix_s(o);
                let age_s = now_unix.saturating_sub(fetched);
                match schedule.resume_stored(uptime_s(), fetched, now_unix) {
                    Some(wait) => println!("FETCH_RESUME stored_age_s={age_s} next_in_s={wait}"),
                    None => println!("FETCH_RESUME stored_age_s={age_s} due=now"),
                }
            }
        }
        let trusted = latest.filter(|o| trust.projection_trusted(*o, uptime_us()));
        if trusted.is_some() {
            ota.note(Check::TrustedUtc);
        }
        let mut refreshed = false;
        // 0080: the schedule runs on uptime, whether or not a server answered.
        if schedule.due(uptime_s()) {
            match latest.filter(|o| trust.projection_trusted(*o, uptime_us())) {
                Some(o) => {
                    deferred = false;
                    attempts += 1;
                    diag!(
                        "FETCH_UTC age_s={} uncertainty_us={}",
                        (uptime_us() - o.midpoint_uptime_us) / 1_000_000,
                        o.uncertainty_us
                    );
                    let outcome = fetch_once(stack, &mut resources, o, attempts).await;
                    refreshed = outcome == Outcome::Success;
                    let now = uptime_s();
                    let next = schedule.record(now, outcome);
                    println!(
                        "FETCH_SCHEDULE attempt={attempts} outcome={outcome:?} \
                         next_in_s={} uptime_s={now}",
                        next - now
                    );
                    if let Some(painted) = painted.take() {
                        // SAFETY: same CPU0 stack and boot as the paint.
                        let w = unsafe { painted.scan() };
                        diag!(
                            "STACK_WATERMARK observed_bytes={} stack_bytes={}",
                            w.observed_bytes,
                            w.stack_top - w.stack_low
                        );
                    }
                }
                None if !deferred => {
                    deferred = true;
                    diag!("FETCH_DEFERRED no_trusted_utc uptime_s={}", uptime_s());
                }
                None => {}
            }
        }
        // 0085: check the channel after each refresh (and on probation until
        // a manifest verifies); install an offer once nobody is using it.
        if let Some(o) = trusted.filter(|_| refreshed || ota.check_due()) {
            let (mut net, _) = resources.net(stack);
            ota.check(&mut net, o).await;
            ota::NET_BEAT.store(uptime_s() as u32, Ordering::Relaxed);
        }
        if let Some(o) = trusted {
            let (mut net, flash) = resources.net(stack);
            ota.maybe_install(&mut net, flash, o).await;
        }
        ota.poll_health(&mut resources.flash);
        Timer::after(Wait::from_secs(POLL_S)).await;
    }
}

/// One fetch attempt, classified for the 0075 schedule. Success hands the
/// catalogue and its fetch UTC to the display owner; a failure leaves the
/// display on whatever it already had (`NO DATA` or the previous catalogue).
async fn fetch_once(
    stack: Stack<'static>,
    r: &mut FetchResources,
    utc: ScreenedObservation,
    attempt: u32,
) -> Outcome {
    FETCHING.store(true, Ordering::Relaxed);
    diag!("FETCH_START attempt={attempt} uptime_s={}", uptime_s());
    let fault = fault_for(attempt);
    if fault != Fault::None {
        println!(
            "FAULT_INJECT attempt={attempt} fault={fault:?} expected={:?}",
            fault.expected()
        );
    }
    let started = uptime_us();
    let mut sent = false;
    let mut to_save: Option<(Range<usize>, i64)> = None;
    let result = with_timeout(FETCH_TIMEOUT, fetch(stack, r, utc, &mut sent, fault)).await;
    let outcome = match result {
        Ok(Ok(len)) => match ingest(&r.response[..len]) {
            Some((catalogue, body)) => {
                let fetched_unix_s = trusted_now_unix_s(utc);
                let fetched_at = DateTime::from_timestamp(fetched_unix_s, 0)
                    .expect("trusted UTC")
                    .naive_utc();
                DATA.send((catalogue, fetched_at)).await;
                to_save = Some((body, fetched_unix_s));
                Outcome::Success
            }
            None => Outcome::Rejected,
        },
        Ok(Err((stage, outcome))) => {
            println!("FETCH_FAILED {stage}");
            outcome
        }
        Err(_) => {
            println!(
                "FETCH_FAILED timeout_s={} request_sent={sent}",
                FETCH_TIMEOUT.as_secs()
            );
            if sent {
                Outcome::Rejected
            } else {
                Outcome::Transport
            }
        }
    };
    let (internal, internal_free, psram) = heap_regions();
    diag!(
        "FETCH_DONE attempt={attempt} total_ms={} heap_peak={} heap_now={} heap_size={HEAP_SIZE} \
         internal_used={internal} internal_free={internal_free} psram_used={psram}",
        (uptime_us() - started) / 1000,
        heap_peak(),
        heap_now()
    );
    if fault != Fault::None {
        println!(
            "FAULT_RESULT attempt={attempt} fault={fault:?} expected={:?} outcome={outcome:?} \
             match={}",
            fault.expected(),
            fault.expected() == outcome
        );
    }
    FETCHING.store(false, Ordering::Relaxed);
    if let (Some(store), Some((body, fetched_unix_s))) = (r.store.as_mut(), to_save) {
        save(store, &mut r.flash, &r.response[body], fetched_unix_s).await;
    }
    outcome
}

/// Records the server's first bytes so the clear-text TLS 1.2 Certificate
/// message can be date-checked after MbedTLS verifies the handshake.
struct Recording<'a, T> {
    inner: T,
    buf: &'a mut [u8],
    len: usize,
    /// `handshake` fault: report end of stream once the server has sent its
    /// first byte, as if it closed the connection mid-handshake.
    cut_after_first_byte: bool,
}

impl<T: io::ErrorType> io::ErrorType for Recording<'_, T> {
    type Error = T::Error;
}

impl<T: io::Read> io::Read for Recording<'_, T> {
    async fn read(&mut self, out: &mut [u8]) -> Result<usize, Self::Error> {
        if self.cut_after_first_byte && self.len > 0 {
            return Ok(0);
        }
        let n = self.inner.read(out).await?;
        let k = n.min(self.buf.len() - self.len);
        self.buf[self.len..self.len + k].copy_from_slice(&out[..k]);
        self.len += k;
        Ok(n)
    }
}

impl<T: io::Write> io::Write for Recording<'_, T> {
    async fn write(&mut self, data: &[u8]) -> Result<usize, Self::Error> {
        self.inner.write(data).await
    }
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush().await
    }
}

/// DNS, TCP, TLS handshake, certificate dates, one GET and read to close.
/// Returns the number of response bytes; the caller parses them. Failures
/// carry their stage and 0075 class; `sent` is set once the GET is written.
async fn fetch(
    stack: Stack<'static>,
    r: &mut FetchResources,
    utc: ScreenedObservation,
    sent: &mut bool,
    fault: Fault,
) -> Result<usize, (&'static str, Outcome)> {
    use Outcome::{Handshake, Rejected, Transport};
    let FetchResources {
        tls,
        tcp_rx,
        tcp_tx,
        response,
        capture,
        ..
    } = r;
    #[cfg(feature = "live-fetch-faults")]
    let host = if fault == Fault::Dns {
        FAULT_DNS_HOST
    } else {
        element_fetch::HOST
    };
    #[cfg(not(feature = "live-fetch-faults"))]
    let host = element_fetch::HOST;
    let addrs = stack
        .dns_query(host, DnsQueryType::A)
        .await
        .map_err(|_| ("stage=dns", Transport))?;
    let addr = *addrs.first().ok_or(("stage=dns_empty", Transport))?;
    #[cfg(feature = "live-fetch-faults")]
    let addr = if fault == Fault::Tcp {
        FAULT_TCP_ADDR
    } else {
        addr
    };
    let mut socket = TcpSocket::new(stack, tcp_rx, tcp_tx);
    socket.set_timeout(Some(Wait::from_secs(20)));
    let t0 = uptime_us();
    socket
        .connect((addr, element_fetch::PORT))
        .await
        .map_err(|_| ("stage=tcp_connect", Transport))?;
    let t1 = uptime_us();

    #[cfg(feature = "live-fetch-faults")]
    let roots = if fault == Fault::Verify {
        WRONG_ROOT
    } else {
        ROOTS
    };
    #[cfg(not(feature = "live-fetch-faults"))]
    let roots = ROOTS;
    let config = ClientSessionConfig {
        ca_chain: Some(
            Certificate::new(X509::PEM(c_str(roots))).map_err(|_| ("stage=roots", Rejected))?,
        ),
        server_name: Some(SERVER_NAME),
        min_version: TlsVersion::Tls1_2,
        ..ClientSessionConfig::new()
    };
    let stream = Recording {
        inner: socket,
        buf: capture,
        len: 0,
        cut_after_first_byte: fault == Fault::Handshake,
    };
    let mut session = Session::new(tls.reference(), stream, &SessionConfig::Client(config))
        .map_err(|_| ("stage=tls_session", Transport))?;
    if let Err(e) = session.connect().await {
        let verify = session.tls_verification_details();
        println!(
            "FETCH_TLS ok=false error={e:?} verify=0x{verify:x} heap_peak={}",
            heap_peak()
        );
        // A certificate the roots reject will be rejected again.
        let class = if verify != 0 { Rejected } else { Handshake };
        return Err(("stage=tls_handshake", class));
    }
    let t2 = uptime_us();
    diag!(
        "FETCH_TLS ok=true version={:?} tcp_ms={} handshake_ms={} heap_peak={}",
        session.tls_version(),
        (t1 - t0) / 1000,
        (t2 - t1) / 1000,
        heap_peak()
    );

    // Dates before any request: an invalid certificate costs no GP download.
    let now = trusted_now_unix_s(utc);
    let recorded = session.stream().len;
    match tls_dates::check(&mut session.stream().buf[..recorded], now) {
        Ok(c) => diag!(
            "FETCH_CERT_DATES ok=true certificates={} now={now} leaf_not_before={} \
             leaf_not_after={} earliest_not_after={} recorded={recorded}",
            c.certificates,
            c.leaf.not_before,
            c.leaf.not_after,
            c.earliest_not_after
        ),
        Err(e) => {
            println!("FETCH_CERT_DATES ok=false error={e:?} now={now} recorded={recorded}");
            let _ = session.close().await;
            return Err(("stage=certificate_dates", Rejected));
        }
    }

    // From here the server may count the request: never retry early.
    *sent = true;
    session
        .write_all(element_fetch::REQUEST.as_bytes())
        .await
        .map_err(|_| ("stage=write", Rejected))?;
    session
        .flush()
        .await
        .map_err(|_| ("stage=flush", Rejected))?;
    let mut len = 0;
    loop {
        if len == response.len() {
            println!("FETCH_HTTP error=response_buffer_full bytes={len}");
            return Err(("stage=read_overflow", Rejected));
        }
        match session.read(&mut response[len..]).await {
            Ok(0) => break,
            Ok(n) => len += n,
            // Truncation is caught by the exact Content-Length check.
            Err(e) => {
                diag!("FETCH_READ_END error={e:?} bytes={len}");
                break;
            }
        }
        // Stop early on a non-200 or oversized head; never read more.
        if let Err(e) = element_fetch::parse_head(&response[..len]) {
            print_http_error(e);
            return Err(("stage=http_head", Rejected));
        }
    }
    let _ = session.close().await;
    diag!(
        "FETCH_TRANSFER bytes={len} transfer_ms={}",
        (uptime_us() - t2) / 1000
    );
    Ok(len)
}

/// Screened UTC carried forward by monotonic uptime (seconds resolution is
/// ample for certificate dates; uncertainty is under 100 ms).
fn trusted_now_unix_s(utc: ScreenedObservation) -> i64 {
    let elapsed = uptime_us().saturating_sub(utc.midpoint_uptime_us);
    utc.utc_at_midpoint.and_utc().timestamp() + (elapsed / 1_000_000) as i64
}

/// The catalogue and where its body sits in `bytes` (kept for the save).
fn ingest(bytes: &[u8]) -> Option<(Catalogue, Range<usize>)> {
    let body = match element_fetch::parse_response(bytes) {
        Ok(body) => body,
        Err(e) => {
            print_http_error(e);
            return None;
        }
    };
    diag!("FETCH_HTTP status=200 body_bytes={}", body.len());
    let start = body.as_ptr() as usize - bytes.as_ptr() as usize;
    let catalogue = ingest_body(body)?;
    Some((catalogue, start..start + body.len()))
}

/// Parses a GP JSON body (fetched or stored) into a catalogue.
fn ingest_body(body: &[u8]) -> Option<Catalogue> {
    let t = uptime_us();
    match element_fetch::ingest(body) {
        Ok(ingested) => {
            let s = ingested.summary();
            diag!(
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
            Some(ingested.catalogue)
        }
        Err(e) => {
            println!("FETCH_CATALOGUE failed {e:?}");
            None
        }
    }
}

fn print_http_error(e: HttpError) {
    println!("FETCH_HTTP error={e:?}");
}

fn c_str(s: &'static str) -> &'static core::ffi::CStr {
    core::ffi::CStr::from_bytes_with_nul(s.as_bytes()).expect("NUL-terminated roots")
}

/// The PCNT unit plus the pins it reads; the pins stay owned so their
/// pull-ups and input routing are never released.
struct KnobInput {
    unit: Unit<'static, 0>,
    _a: Input<'static>,
    _b: Input<'static>,
}

#[embassy_executor::task]
async fn display(
    mut spi: Spi<'static, esp_hal::Blocking>,
    mut cs: Output<'static>,
    observer: GeodeticPosition,
    knob_input: KnobInput,
    mut wdt: Rwdt,
    hang_drill: bool,
) {
    // Clockwise counts negative on this encoder; clockwise zooms in.
    let mut detents = Detents::new(knob_input.unit.value(), true);
    // No elements until the fetch delivers them: NO FRAME, then NO DATA.
    // The visual group has no display role (0045 roles only rank the limit).
    let view = View::rectangular(FOV_DEG, Layout::wide().aspect_ratio()).expect("view");
    let mut app = RadarApp::awaiting_catalogue(observer, view)
        .expect("observer camera")
        .with_object_limit(OBJECT_LIMIT)
        .with_freshness(Freshness::DEVICE);
    // Longest staggered update per stats window, and since boot (0077).
    let (mut update_max_us, mut update_boot_max_us) = (0u64, 0u64);
    let mut loads = 0u32;
    let mut was_stale = false;
    let mut had_catalogue = false;
    let screen = unsafe { &mut *(&raw mut SCREEN) };
    let sender = unsafe { &mut *(&raw mut SENDER) };
    let period = unsafe { &mut *(&raw mut PERIOD) };
    let cadence = unsafe { &mut *(&raw mut FRAME_STATS) };
    let (mut windows, mut window_started) = (0u32, uptime_us());
    let mut last_selected = None;
    let coast = unsafe { (*(&raw mut LAYER)).write(CoastLayer::new()) };
    // SAFETY: this task alone owns these statics and SPI; the network side
    // sends events/catalogue over channels and never touches render/app/panel.
    coast.prepare_for(app.observer());
    assert!(coast.error().is_none(), "coast preparation failed");
    let mut gate = UtcTrustPreflight::prototype();
    let mut retained = false;
    let mut frames = 0u64;
    let mut last_stats = uptime_us();
    let mut last_loop = uptime_us();
    let (mut loop_max_us, mut fetch_max_us, mut was_fetching) = (0u64, 0u64, false);
    let (mut store_max_us, mut was_storing) = (0u64, false);
    loop {
        // 0086: feed only while the network loop is alive too.
        let beat = ota::NET_BEAT.load(Ordering::Relaxed);
        if (uptime_s() as u32).saturating_sub(beat) < ota::NET_STALL_S {
            wdt.feed();
        }
        if hang_drill && frames > 0 {
            println!("OTA drill: hanging the executor on probation");
            loop {
                core::hint::spin_loop();
            }
        }
        let updating = match ota::UPDATING.load(Ordering::Relaxed) {
            255 => None,
            pct => Some(pct),
        };
        let gap = uptime_us() - last_loop;
        loop_max_us = loop_max_us.max(gap);
        let fetching = FETCHING.load(Ordering::Relaxed);
        if fetching || was_fetching {
            fetch_max_us = fetch_max_us.max(gap);
            cadence.note(Cause::Fetch);
        }
        if was_fetching && !fetching {
            diag!("DISPLAY_FETCH_GAP max_loop_gap_us={fetch_max_us}");
            fetch_max_us = 0; // per fetch, not since boot
        }
        was_fetching = fetching;
        let storing = STORING.load(Ordering::Relaxed);
        if storing || was_storing {
            store_max_us = store_max_us.max(gap);
            cadence.note(Cause::Store);
        }
        if was_storing && !storing {
            diag!("DISPLAY_STORE_GAP max_loop_gap_us={store_max_us}");
            store_max_us = 0;
        }
        was_storing = storing;
        while let Ok(event) = EVENTS.try_receive() {
            let now = uptime_us();
            let decision = live_utc::apply(&mut gate, &mut app, now, event);
            cadence.note(Cause::Sntp);
            diag!("LIVE_UTC source={decision:?} uptime_us={now}");
        }
        if let Ok((catalogue, fetched_at)) = DATA.try_receive() {
            #[cfg(feature = "live-fetch-faults")]
            if loads == 0 {
                app = with_fault_freshness(app, &catalogue, fetched_at);
            }
            let before = app.selected_id();
            app.load_fetched_catalogue(catalogue, fetched_at);
            ota::CATALOGUE_LOADED.store(true, Ordering::Relaxed);
            cadence.note(Cause::Load);
            loads += 1;
            let c = app.counts();
            println!(
                "LIVE_FETCH loaded load={loads} fetched_at={fetched_at:?}Z catalogue={} dropped={} \
                 valid={} sky={} view={} selected_before={before:?} selected={:?}",
                c.catalogue,
                app.dropped_objects(),
                app.time().is_some(),
                c.above_horizon,
                c.visible,
                app.selected_id()
            );
        }
        let now = uptime_us();
        let presses = boot_capture::take_presses();
        if presses > 0 {
            ota::LAST_INPUT_S.store(uptime_s() as u32, Ordering::Relaxed);
            let applied = apply_presses(presses, &mut app);
            diag!(
                "BOOT_CAPTURE drained={presses} applied={applied} uptime_us={now} valid={} selected={:?}",
                app.time().is_some(),
                app.selected_id()
            );
        }
        let turned = detents.update(knob_input.unit.value());
        if turned != 0 {
            ota::LAST_INPUT_S.store(uptime_s() as u32, Ordering::Relaxed);
            app.zoom_detents(turned);
            cadence.note(Cause::Knob);
            diag!(
                "KNOB detents={turned} fov_deg={:.3} uptime_us={now} selected={:?}",
                app.view().fov_deg(),
                app.selected_id()
            );
        }
        let has_catalogue = app.counts().catalogue > 0;
        if had_catalogue && !has_catalogue {
            println!(
                "LIVE_FETCH expired frame_utc={:?}Z catalogue=0 heap_now={}",
                app.time(),
                heap_now()
            );
        }
        had_catalogue = has_catalogue;
        // A new anchor or a renewal beyond the 100 ms budget invalidates the
        // frame until the next render (0062/0063), when `data_stale()` is false for want of a frame UTC;
        // only frames with UTC decide the transition.
        if app.time().is_some() && app.data_stale() != was_stale {
            was_stale = app.data_stale();
            println!("LIVE_FETCH stale={was_stale} frame_utc={:?}Z", app.time());
        }
        if now.saturating_sub(last_stats) >= 10_000_000 {
            let stats = boot_capture::take_stats();
            let c = app.counts();
            let (internal, internal_free, psram) = heap_regions();
            let oldest_ms = match (app.time(), app.oldest_sample_utc()) {
                (Some(t), Some(o)) => (t - o).num_milliseconds(),
                _ => -1,
            };
            diag!(
                "BOOT_CAPTURE_STATS samples={} max_gap_us={} loop_max_us={loop_max_us} \
                 update_max_us={update_max_us} update_boot_max_us={update_boot_max_us} \
                 oldest_ms={oldest_ms} catalogue={} sky={} view={} stale={} heap_now={} \
                 internal_used={internal} internal_free={internal_free} psram_used={psram} \
                 selected={:?}",
                stats.samples,
                stats.max_gap_us,
                c.catalogue,
                c.above_horizon,
                c.visible,
                app.data_stale(),
                heap_now(),
                app.selected_id()
            );
            loop_max_us = 0;
            update_max_us = 0;
            last_stats = now;
            print_frame_window(cadence, now - window_started);
            window_started = now;
            windows += 1;
            if windows.is_multiple_of(FRAME_BOOT_WINDOWS) {
                print_frame_boot(cadence);
            }
        }
        match gate.next(&mut app, now, retained) {
            LifecycleAction::RenderNoFrame => {
                match updating {
                    Some(pct) => overhead_render::draw_updating(screen, pct),
                    None => overhead_render::draw_cached(
                        screen,
                        &app,
                        Layout::wide(),
                        false,
                        coast,
                        period,
                    ),
                }
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
                ota::FRAME_SHOWN.store(true, Ordering::Relaxed);
                retained = true;
                println!("LIVE_UTC no_frame_sent uptime_us={done}");
            }
            LifecycleAction::Scheduled(DiagnosticAction::Frame { sample, full }) => {
                let utc = DateTime::from_timestamp_micros(sample.utc_unix_us)
                    .expect("gate checked UTC")
                    .naive_utc();
                let started = uptime_us();
                app.update_staggered(utc, STAGGER)
                    .expect("checked UTC sample");
                let took = uptime_us() - started;
                update_max_us = update_max_us.max(took);
                update_boot_max_us = update_boot_max_us.max(took);
                match updating {
                    Some(pct) => overhead_render::draw_updating(screen, pct),
                    None => overhead_render::draw_cached(
                        screen,
                        &app,
                        Layout::wide(),
                        false,
                        coast,
                        period,
                    ),
                }
                .expect("radar render");
                let phase = gate.next_phase();
                cs.set_high();
                let result = sender.send(phase, &screen.pixels, full, |part| spi.write(part));
                cs.set_low();
                let transfer = match result {
                    Ok(transfer) => transfer,
                    Err(_) => {
                        gate.transfer_failed();
                        panic!("radar SPI failed; panel state unknown");
                    }
                };
                let done = uptime_us();
                gate.transfer_succeeded(done, phase, transfer.kind == Kind::Full);
                ota::FRAME_SHOWN.store(true, Ordering::Relaxed);
                retained = true;
                frames += 1;
                if app.selected_id() != last_selected {
                    last_selected = app.selected_id();
                    cadence.note(Cause::Selection);
                }
                if LOGGED.swap(false, Ordering::Relaxed) {
                    cadence.note(Cause::Log);
                }
                let own = if transfer.kind == Kind::Full {
                    Cause::Full
                } else {
                    Cause::Idle
                };
                cadence.frame(sample.skipped, done - started, own);
                if frames == 1 {
                    // Event: the first radar frame after boot (not repeated
                    // after a time-loss NO FRAME; that has its own line).
                    println!("RADAR first_frame utc_us={}", sample.utc_unix_us);
                }
                if frames.is_multiple_of(200) || sample.skipped > 0 || transfer.kind == Kind::Full {
                    diag!(
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
                cadence.note(Cause::Full);
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
        last_loop = uptime_us();
        // Independent of DNS, association, UDP and TLS awaits; CPU-bound TLS
        // steps in the network task still delay this loop (measured above).
        Timer::after(Wait::from_millis(5)).await;
    }
}

/// `FRAME_WINDOW`: the 10 s window's frames and charged skips per cause, as
/// `cause=frames/skipped/max_work_ms`; causes with neither are left out.
fn print_frame_window(stats: &mut FrameStats, span_us: u64) {
    let window = stats.take_window();
    if !DIAGNOSTIC.load(Ordering::Relaxed) {
        return;
    }
    let frames: u64 = window.iter().map(|w| w.frames).sum();
    let skipped: u64 = window.iter().map(|w| w.skipped).sum();
    esp_println::print!(
        "FRAME_WINDOW span_ms={} frames={frames} skipped={skipped}",
        span_us / 1000
    );
    for (cause, w) in CAUSES.iter().zip(window) {
        if w.frames > 0 || w.skipped > 0 {
            esp_println::print!(
                " {}={}/{}/{}",
                cause.name(),
                w.frames,
                w.skipped,
                w.max_work_us.div_ceil(1000)
            );
        }
    }
    println!();
    LOGGED.store(true, Ordering::Relaxed);
}

/// `FRAME_BOOT`: since-boot frames, charged skips and work-time percentiles
/// (whole-ms bounds; 256 means at least that) per cause with any activity.
fn print_frame_boot(stats: &FrameStats) {
    if !DIAGNOSTIC.load(Ordering::Relaxed) {
        return;
    }
    for cause in CAUSES {
        let t = stats.boot(cause);
        if t.window.frames == 0 && t.window.skipped == 0 {
            continue;
        }
        let p = |m| t.percentile_ms(m).unwrap_or(0);
        println!(
            "FRAME_BOOT cause={} frames={} skipped={} p50_ms={} p90_ms={} p99_ms={} p999_ms={} max_us={}",
            cause.name(),
            t.window.frames,
            t.window.skipped,
            p(500),
            p(900),
            p(990),
            p(999),
            t.window.max_work_us
        );
    }
    LOGGED.store(true, Ordering::Relaxed);
}

/// Fault build: replace 0075's thresholds on the first load so `OLD DATA`
/// and expiry happen within one capture. Expiry is relative to that load
/// because the newest element epoch may already be hours old when fetched.
#[cfg(feature = "live-fetch-faults")]
fn with_fault_freshness(
    app: RadarApp,
    catalogue: &Catalogue,
    fetched_at: NaiveDateTime,
) -> RadarApp {
    let f = faults();
    let mut freshness = Freshness::DEVICE;
    if let Some(stale_s) = f.stale_s {
        freshness.stale_after = TimeDelta::seconds(stale_s as i64);
    }
    if let (Some(after_s), Some(newest)) = (
        f.expire_after_load_s,
        catalogue
            .entries()
            .iter()
            .map(|e| e.satellite().epoch())
            .max(),
    ) {
        let expire_s = crate::fault_plan::expire_after_s(
            fetched_at.and_utc().timestamp(),
            newest.and_utc().timestamp(),
            after_s,
        );
        freshness.expire_after = TimeDelta::seconds(expire_s);
    }
    println!(
        "FAULT_FRESHNESS stale_after_s={} expire_after_s={} (newest epoch age + override)",
        freshness.stale_after.num_seconds(),
        freshness.expire_after.num_seconds()
    );
    app.with_freshness(freshness)
}

enum QueryResult {
    NoReply,
    /// A matched reply; the observation if it was one.
    Reply(Option<ScreenedObservation>),
    KissOfDeath([u8; 4]),
}

async fn query(
    stack: Stack<'static>,
    socket: &mut UdpSocket<'_>,
    trng: &mut Trng,
    index: usize,
    server: &str,
    seq: u32,
) -> QueryResult {
    if let Some(code) = kod_for(index) {
        // Fault build: no packet; the outcome a real KoD would produce.
        println!(
            "FAULT_INJECT_KOD seq={seq} server={server} code={} uptime_s={}",
            core::str::from_utf8(&code).unwrap_or("?"),
            uptime_s()
        );
        EVENTS.send(SntpOutcome::KissOfDeath(code)).await;
        return QueryResult::KissOfDeath(code);
    }
    let Ok(addrs) = stack.dns_query(server, DnsQueryType::A).await else {
        diag!("SNTP seq={seq} server={server} outcome=DnsFailed");
        return QueryResult::NoReply;
    };
    let Some(addr) = addrs.first().copied() else {
        return QueryResult::NoReply;
    };
    let peer = IpEndpoint::new(addr, 123);
    let nonce = (u64::from(trng.random()) << 32) | u64::from(trng.random()) | 1;
    let sent = uptime_us(); // before handing the request to the stack
    let mut pending = Some(SntpRequest {
        transmit: nonce,
        sent_uptime_us: sent,
    });
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
            diag!("SNTP seq={seq} server={server} outcome=Timeout");
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
        diag!("SNTP seq={seq} server={server} outcome={outcome:?}");
        EVENTS.send(outcome).await;
        return match outcome {
            SntpOutcome::KissOfDeath(code) => QueryResult::KissOfDeath(code),
            SntpOutcome::Observation(o) => QueryResult::Reply(Some(o)),
            _ => QueryResult::Reply(None),
        };
    }
}

/// `wifi_drop` (fault build only): once, at that uptime, disconnect and stay
/// off for the given seconds, then reconnect as after any loss.
#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>, mut wifi_drop: Option<(u64, u64)>) {
    loop {
        match controller.connect_async().await {
            Ok(_) => {
                println!("SNTP_WIFI connected uptime_s={}", uptime_s());
                if let Some((at_s, for_s)) = wifi_drop {
                    let wait = Wait::from_secs(at_s.saturating_sub(uptime_s()));
                    if with_timeout(wait, controller.wait_for_disconnect_async())
                        .await
                        .is_err()
                    {
                        wifi_drop = None;
                        println!("FAULT_WIFI drop uptime_s={} off_s={for_s}", uptime_s());
                        let r = controller.disconnect_async().await;
                        println!("FAULT_WIFI disconnected ok={}", r.is_ok());
                        Timer::after(Wait::from_secs(for_s)).await;
                        println!("FAULT_WIFI reconnecting uptime_s={}", uptime_s());
                        continue;
                    }
                } else {
                    let _ = controller.wait_for_disconnect_async().await;
                }
                println!("SNTP_WIFI disconnected uptime_s={}", uptime_s());
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
