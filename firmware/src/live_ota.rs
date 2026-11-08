//! Signed updates on the device (0085) and the health check (0086), a child
//! of `live_radar` (it shares that runtime's flash, TLS context and buffers).
//!
//! Boot: read `settings` and `otadata`, decide with `ota_health::on_boot`
//! (probation, ordinary, or recording a rolled-back build). Probation: the
//! health check needs a frame, Wi-Fi, trusted UTC, a catalogue and a verified
//! manifest within 10 minutes, then marks the slot `VALID`; otherwise it marks
//! it `INVALID` and resets into the old slot. Checks: after each successful
//! element refresh, and on every poll while on probation until a manifest
//! verifies. An offered build installs once nobody has turned the knob or
//! pressed BOOT for 2 minutes: the image streams into the other slot (hashed
//! as it goes, never held whole), `pending_build` is saved, `otadata`
//! selects the slot as `NEW`, and the device resets.
//!
//! Sources are tried in order and the first verified manifest wins: the
//! `ota-dev` build asks only the plain-HTTP dev host (`OTA_DEV_HOST`,
//! drills on the LAN, trusting `OTA_DEV_PUBLIC_KEY`); others ask
//! `overhead.wesklein.com`, then raw GitHub. No redirect is followed.
use super::{EspFlash, Recording, c_str, trusted_now_unix_s, uptime_s, uptime_us};
use crate::{
    element_fetch::{self, HttpError},
    element_store::{self, Flash as _},
    ota_health::{self, Boot, Check, Health, SlotState, Verdict},
    otadata::{self, Entry},
    settings_store::{self, Settings},
    sntp_screen::ScreenedObservation,
    tls_dates,
};
use alloc::{format, vec, vec::Vec};
use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};
use embassy_net::{IpAddress, Stack, dns::DnsQueryType, tcp::TcpSocket};
use embassy_time::{Duration as Wait, with_timeout};
use esp_println::println;
use mbedtls_rs::{
    Certificate, ClientSessionConfig, Session, SessionConfig, Tls, TlsVersion, X509, io,
};
use overhead_update::{self as update, Channel, ImageCheck, Manifest, Offer, Running};

/// Build number compiled in by the release build (`OVERHEAD_BUILD`); 0 for
/// developer builds, so any signed release is newer.
pub const BUILD: u32 = parse_u32(option_env!("OVERHEAD_BUILD"));
pub const VERSION: &str = match option_env!("OVERHEAD_VERSION") {
    Some(v) => v,
    None => "dev",
};
/// Channel written to an empty settings store (0085: gifts `stable`; the
/// user's board is built with `OTA_CHANNEL=beta`).
const SEED_CHANNEL: Channel = match option_env!("OTA_CHANNEL") {
    Some(c) if str_eq(c, "beta") => Channel::Beta,
    Some(c) if str_eq(c, "stable") => Channel::Stable,
    None => Channel::Stable,
    Some(_) => panic!("OTA_CHANNEL must be stable or beta"),
};

const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}
/// 0085's accepted release keys (public halves only).
const RELEASE_KEYS: &[[u8; 32]] = &crate::release_keys::KEYS;
/// Settings-record format this firmware holds (0084).
const SETTINGS_FORMAT: u32 = settings_store::FORMAT as u32;
/// 0085: install only after this long without knob or BOOT input.
const IDLE_S: u32 = 120;
/// Roots for the Pages and raw GitHub hosts (0085).
const OTA_ROOTS: &str = concat!(include_str!("../certs/ota-roots.pem"), "\0");
const TIMEOUT: Wait = Wait::from_secs(30);
/// An image read may stall this long between chunks before it is abandoned.
const IMAGE_TIMEOUT: Wait = Wait::from_secs(600);
const MAX_MANIFEST: usize = 2 * 1024;
const SECTOR: usize = element_store::SECTOR as usize;

/// Set by the display task: a frame reached the panel (0086 health).
pub static FRAME_SHOWN: AtomicBool = AtomicBool::new(false);
/// Set by the display task: a catalogue was loaded.
pub static CATALOGUE_LOADED: AtomicBool = AtomicBool::new(false);
/// Uptime of the last knob or BOOT input, in seconds.
pub static LAST_INPUT_S: AtomicU32 = AtomicU32::new(0);
/// `UPDATING n%` while an image streams in; 255 = not installing.
pub static UPDATING: AtomicU8 = AtomicU8::new(255);
/// Network-loop heartbeat (uptime s); the display stops feeding the
/// watchdog when it is older than [`NET_STALL_S`].
pub static NET_BEAT: AtomicU32 = AtomicU32::new(0);
pub const NET_STALL_S: u32 = 15 * 60;

const fn parse_u32(s: Option<&str>) -> u32 {
    let Some(s) = s else { return 0 };
    let b = s.as_bytes();
    let mut i = 0;
    let mut n: u32 = 0;
    while i < b.len() {
        assert!(b[i].is_ascii_digit(), "OVERHEAD_BUILD must be decimal");
        n = n * 10 + (b[i] - b'0') as u32;
        i += 1;
    }
    n
}

/// The partitions this runtime uses, from one read of the table.
pub struct Parts {
    pub elements: Option<(u32, u32)>,
    settings: Option<(u32, u32)>,
    otadata: Option<u32>,
    slots: [Option<(u32, u32)>; 2],
    booted: Option<usize>,
}

pub fn read_parts(flash: &mut EspFlash) -> Result<Parts, &'static str> {
    use esp_bootloader_esp_idf::partitions::{self, PARTITION_TABLE_MAX_LEN};
    let table = partitions::read_partition_table(
        &mut flash.storage,
        (&mut flash.bounce.0[..PARTITION_TABLE_MAX_LEN])
            .try_into()
            .expect("bounce holds a table"),
    )
    .map_err(|_| "partition_table")?;
    let mut p = Parts {
        elements: None,
        settings: None,
        otadata: None,
        slots: [None, None],
        booted: None,
    };
    for e in table.iter() {
        let span = Some((e.offset(), e.len()));
        match (e.raw_type(), e.raw_subtype(), e.label_as_str()) {
            (0, 0x10, _) => p.slots[0] = span,
            (0, 0x11, _) => p.slots[1] = span,
            (1, 0x00, _) if e.len() == 2 * otadata::SECTOR => p.otadata = Some(e.offset()),
            (1, _, "elements") => p.elements = span,
            (1, _, "settings") => p.settings = span,
            _ => {}
        }
    }
    if let Ok(Some(b)) = table.booted_partition() {
        p.booted = p
            .slots
            .iter()
            .position(|s| *s == Some((b.offset(), b.len())));
    }
    Ok(p)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drill {
    None,
    /// Panics on the first network-loop pass of a probation boot.
    Panic,
    /// Stops the whole executor after the first frame on probation.
    Hang,
    /// Asks for a manifest that does not exist while on probation.
    NoManifest,
}

/// `ota-drill` builds misbehave only on probation (0086 brick drill).
pub const fn drill() -> Drill {
    if !cfg!(feature = "ota-drill") {
        return Drill::None;
    }
    match option_env!("OTA_DRILL") {
        Some(d) if str_eq(d, "panic") => Drill::Panic,
        Some(d) if str_eq(d, "hang") => Drill::Hang,
        Some(d) if str_eq(d, "no-manifest") => Drill::NoManifest,
        None => Drill::None,
        Some(_) => panic!("OTA_DRILL must be panic, hang or no-manifest"),
    }
}

pub struct Ota {
    store: Option<settings_store::Store>,
    settings: Settings,
    otadata: Option<u32>,
    slots: [Option<(u32, u32)>; 2],
    booted: Option<usize>,
    pub boot: Boot,
    health: Option<Health>,
    offer: Option<(Manifest, usize)>,
    #[cfg(feature = "ota-dev")]
    last_check_s: u64,
}

fn read_entries(flash: &mut EspFlash, at: u32) -> [Option<Entry>; 2] {
    let mut out = [None, None];
    for (i, e) in out.iter_mut().enumerate() {
        let mut b = [0u8; otadata::ENTRY_LEN];
        if flash.read(at + i as u32 * otadata::SECTOR, &mut b).is_ok() {
            *e = Entry::decode(&b);
        }
    }
    out
}

fn write_entry(flash: &mut EspFlash, at: u32, sector: usize, e: Entry) -> Result<(), ()> {
    let o = at + sector as u32 * otadata::SECTOR;
    flash.erase_sector(o).map_err(|_| ())?;
    flash.write(o, &e.encode()).map_err(|_| ())
}

fn slot_state(state: u32) -> SlotState {
    use otadata::state::*;
    match state {
        NEW => SlotState::New,
        PENDING_VERIFY => SlotState::PendingVerify,
        VALID => SlotState::Valid,
        INVALID => SlotState::Invalid,
        ABORTED => SlotState::Aborted,
        _ => SlotState::Undefined,
    }
}

impl Ota {
    pub fn boot(flash: &mut EspFlash, parts: &Parts) -> Self {
        let mut store = parts
            .settings
            .and_then(|(o, l)| settings_store::Store::new(o, l));
        let (mut settings, mut changed) = (Settings::defaults(SEED_CHANNEL), true);
        if let Some(st) = store.as_mut() {
            let r = st.load(flash);
            if let Some(s) = r.loaded {
                (settings, changed) = (s, false);
            } else {
                println!(
                    "SETTINGS empty slot0={:?} slot1={:?}",
                    r.slots[0], r.slots[1]
                );
            }
        }
        let entries = parts.otadata.map(|at| read_entries(flash, at));
        let state = match (entries.and_then(otadata::selected), parts.booted) {
            (Some((_, e)), Some(b)) if e.slot() == b => slot_state(e.state),
            _ => SlotState::Undefined,
        };
        let (boot, c) = ota_health::on_boot(BUILD, state, &mut settings);
        changed |= c;
        changed |= seed_dev_settings(&mut settings);
        let mut ota = Self {
            store,
            settings,
            otadata: parts.otadata,
            slots: parts.slots,
            booted: parts.booted,
            boot,
            health: (boot == Boot::Verify).then(|| Health::new(uptime_us())),
            offer: None,
            #[cfg(feature = "ota-dev")]
            last_check_s: 0,
        };
        if changed {
            ota.save(flash);
        }
        println!(
            "OTA boot build={BUILD} version={VERSION} slot={:?} state={state:?} outcome={boot:?} \
             channel={} failed_build={:?} drill={:?}",
            parts.booted,
            ota.settings.channel.as_str(),
            ota.settings.failed_build,
            drill()
        );
        ota
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    fn save(&mut self, flash: &mut EspFlash) {
        let Some(st) = self.store.as_mut() else {
            println!("SETTINGS unavailable");
            return;
        };
        if let Err(e) = st.save(flash, &self.settings) {
            println!("SETTINGS save_failed {e:?}");
        }
    }

    pub fn verifying(&self) -> bool {
        self.health.is_some()
    }

    pub fn note(&mut self, c: Check) {
        if let Some(h) = self.health.as_mut()
            && !h.met(c)
        {
            h.note(c);
            println!(
                "OTA_HEALTH met={c:?} missing=0x{:02x} uptime_s={}",
                h.missing(),
                uptime_s()
            );
        }
    }

    /// Whether a manifest check is due now (besides after a refresh).
    pub fn check_due(&self) -> bool {
        if self.health.is_some_and(|h| !h.met(Check::ManifestVerified)) {
            return true;
        }
        #[cfg(feature = "ota-dev")]
        if let Some(m) = option_env!("OTA_CHECK_MINUTES").and_then(|m| m.parse::<u64>().ok()) {
            return uptime_s() >= self.last_check_s + m * 60;
        }
        false
    }

    /// Settles probation: `VALID` and clear `pending_build`, or `INVALID` and
    /// reset into the old slot (which records the failure).
    pub fn poll_health(&mut self, flash: &mut EspFlash) {
        let Some(h) = self.health else { return };
        let verdict = match h.verdict(uptime_us()) {
            // A drill image must fail on probation, never confirm itself.
            Verdict::Healthy if drill() != Drill::None => Verdict::Waiting,
            v => v,
        };
        match verdict {
            Verdict::Waiting => {}
            Verdict::Healthy => {
                let ok = self.mark(flash, otadata::state::VALID);
                self.health = None;
                self.boot = Boot::Normal;
                self.settings.pending_build = None;
                self.save(flash);
                println!(
                    "OTA_HEALTH valid build={BUILD} otadata_ok={ok} uptime_s={}",
                    uptime_s()
                );
            }
            Verdict::Failed => {
                let ok = self.mark(flash, otadata::state::INVALID);
                println!(
                    "OTA_HEALTH failed build={BUILD} missing=0x{:02x} otadata_ok={ok}; resetting",
                    h.missing()
                );
                reset();
            }
        }
    }

    fn mark(&mut self, flash: &mut EspFlash, state: u32) -> bool {
        let Some(at) = self.otadata else { return false };
        match otadata::set_state(read_entries(flash, at), state) {
            Some((sector, e)) => write_entry(flash, at, sector, e).is_ok(),
            None => false,
        }
    }

    fn running(&self) -> Running {
        Running {
            channel: self.settings.channel,
            build: BUILD,
            failed_build: self.settings.failed_build,
            settings_format: SETTINGS_FORMAT,
        }
    }

    /// Fetches and verifies the channel manifest; remembers an offered build.
    pub async fn check(&mut self, net: &mut Net<'_>, utc: ScreenedObservation) {
        #[cfg(feature = "ota-dev")]
        {
            self.last_check_s = uptime_s();
        }
        let channel = self.settings.channel.as_str();
        let path = if self.verifying() && drill() == Drill::NoManifest {
            format!("/ota/{channel}/missing.json")
        } else {
            format!("/ota/{channel}/manifest.json")
        };
        for (i, src) in sources().iter().enumerate() {
            let full = format!("{}{path}", src.prefix);
            let mut body: Vec<u8> = Vec::new();
            let got = net
                .get(src, &full, utc, MAX_MANIFEST, |chunk| {
                    body.extend_from_slice(chunk);
                    Ok(())
                })
                .await;
            if let Err(e) = got {
                println!("OTA_CHECK source={i} host={} failed={e}", src.host);
                continue;
            }
            let keys = keys();
            match update::verify(&body, &keys) {
                Ok((m, key)) => {
                    self.note(Check::ManifestVerified);
                    let offer = update::offer(&m, &self.running());
                    println!(
                        "OTA_CHECK source={i} build={} version={} key={key} offer={offer:?}",
                        m.build, m.version
                    );
                    self.offer = (offer == Offer::Install).then_some((m, i));
                    return;
                }
                Err(e) => println!("OTA_CHECK source={i} host={} refused={e}", src.host),
            }
        }
    }

    /// Installs a remembered offer once idle; resets on success.
    pub async fn maybe_install(
        &mut self,
        net: &mut Net<'_>,
        flash: &mut EspFlash,
        utc: ScreenedObservation,
    ) {
        if self.verifying() {
            return;
        }
        let Some((m, src)) = self.offer.take() else {
            return;
        };
        let idle = (uptime_s() as u32).saturating_sub(LAST_INPUT_S.load(Ordering::Relaxed));
        if idle < IDLE_S {
            self.offer = Some((m, src));
            return;
        }
        let Some(booted) = self.booted else {
            println!("OTA_INSTALL refused=no_booted_slot");
            return;
        };
        let target = 1 - booted;
        let Some((base, len)) = self.slots[target] else {
            println!("OTA_INSTALL refused=no_slot");
            return;
        };
        if m.length > len {
            println!(
                "OTA_INSTALL refused=too_large length={} slot={len}",
                m.length
            );
            return;
        }
        let src = &sources()[src];
        let path = format!("{}/ota/{}", src.prefix, m.image);
        println!(
            "OTA_INSTALL start build={} length={} slot={target} uptime_s={}",
            m.build,
            m.length,
            uptime_s()
        );
        let started = uptime_us();
        UPDATING.store(0, Ordering::Relaxed);
        let mut writer = SlotWriter::new(base, &m);
        let got = net
            .get_image(src, &path, utc, m.length as usize, &mut writer, flash)
            .await;
        let result = got.and_then(|()| writer.finish(flash));
        UPDATING.store(255, Ordering::Relaxed);
        if let Err(e) = result {
            println!(
                "OTA_INSTALL failed={e} written={} seen={} ms={}",
                writer.written,
                writer.seen(),
                (uptime_us() - started) / 1000
            );
            return;
        }
        let Some(at) = self.otadata else {
            println!("OTA_INSTALL refused=no_otadata");
            return;
        };
        self.settings.pending_build = Some(m.build);
        self.save(flash);
        let (sector, e) = otadata::activate(read_entries(flash, at), target);
        let ok = write_entry(flash, at, sector, e).is_ok();
        println!(
            "OTA_INSTALL done build={} ms={} otadata_seq={} ok={ok}; resetting",
            m.build,
            (uptime_us() - started) / 1000,
            e.seq
        );
        if ok {
            reset();
        }
    }
}

/// 0084 `dev-settings`: fills a missing Wi-Fi or site from the ignored
/// `.wifi.env`/`.site.env` (personal data; never in a published image).
/// Returns whether anything was filled.
#[cfg(feature = "dev-settings")]
fn seed_dev_settings(s: &mut Settings) -> bool {
    let mut changed = false;
    if s.wifi.is_none() {
        s.wifi = settings_store::Wifi::new(env!("SSID"), env!("PASSWORD"));
        assert!(s.wifi.is_some(), "dev-settings: SSID/PASSWORD out of range");
        changed = true;
    }
    if s.site.is_none() {
        let p = match crate::site::parse(
            env!("SITE_LAT_DEG"),
            env!("SITE_LON_DEG"),
            option_env!("SITE_ALT_M"),
        ) {
            Ok(p) => p,
            Err(e) => panic!("dev-settings site: {e}"),
        };
        s.site = Some(settings_store::Site {
            lat_deg: p.latitude_rad.to_degrees(),
            lon_deg: p.longitude_rad.to_degrees(),
            alt_m: p.altitude_km * 1000.0,
        });
        changed = true;
    }
    if changed {
        println!("SETTINGS seeded from dev-settings");
    }
    changed
}

#[cfg(not(feature = "dev-settings"))]
fn seed_dev_settings(_: &mut Settings) -> bool {
    false
}

pub fn reset() -> ! {
    // Let the UART drain the last line.
    let t = uptime_us();
    while uptime_us() - t < 50_000 {}
    esp_hal::system::software_reset()
}

fn keys() -> Vec<[u8; 32]> {
    #[allow(unused_mut)]
    let mut keys: Vec<[u8; 32]> = RELEASE_KEYS.to_vec();
    #[cfg(feature = "ota-dev")]
    if let Ok(k) = update::parse_hex::<32>(env!("OTA_DEV_PUBLIC_KEY")) {
        keys.push(k);
    }
    keys
}

pub struct Source {
    host: &'static str,
    /// Path before `/ota/…`.
    prefix: &'static str,
    /// `None` = HTTPS on 443 against [`OTA_ROOTS`].
    plain: Option<(IpAddress, u16)>,
}

#[cfg(not(feature = "ota-dev"))]
fn sources() -> [Source; 2] {
    [
        Source {
            host: "overhead.wesklein.com",
            prefix: "",
            plain: None,
        },
        Source {
            host: "raw.githubusercontent.com",
            prefix: "/chachoblow/overhead/pages",
            plain: None,
        },
    ]
}

/// `OTA_DEV_HOST` = `a.b.c.d:port` on the LAN; nothing else is asked.
#[cfg(feature = "ota-dev")]
fn sources() -> [Source; 1] {
    let (addr, port) = env!("OTA_DEV_HOST")
        .split_once(':')
        .expect("OTA_DEV_HOST a.b.c.d:port");
    let mut ip = [0u8; 4];
    for (o, part) in ip.iter_mut().zip(addr.split('.')) {
        *o = part.parse().expect("OTA_DEV_HOST address");
    }
    [Source {
        host: "ota-dev",
        prefix: "",
        plain: Some((
            IpAddress::v4(ip[0], ip[1], ip[2], ip[3]),
            port.parse().expect("OTA_DEV_HOST port"),
        )),
    }]
}

/// Streams an image into an app slot, one sector erase and program at a time.
struct SlotWriter {
    base: u32,
    written: u32,
    sector: Vec<u8>,
    fill: usize,
    check: Option<ImageCheck>,
    length: u32,
}

impl SlotWriter {
    fn new(base: u32, m: &Manifest) -> Self {
        Self {
            base,
            written: 0,
            sector: vec![0xFF; SECTOR],
            fill: 0,
            check: Some(ImageCheck::new(m)),
            length: m.length,
        }
    }

    fn push(&mut self, flash: &mut EspFlash, mut data: &[u8]) -> Result<(), &'static str> {
        let check = self.check.as_mut().ok_or("finished")?;
        check.feed(data).map_err(|_| "too_long")?;
        while !data.is_empty() {
            let take = (SECTOR - self.fill).min(data.len());
            self.sector[self.fill..self.fill + take].copy_from_slice(&data[..take]);
            self.fill += take;
            data = &data[take..];
            if self.fill == SECTOR {
                self.flush(flash)?;
            }
        }
        Ok(())
    }

    fn flush(&mut self, flash: &mut EspFlash) -> Result<(), &'static str> {
        if self.fill == 0 {
            return Ok(());
        }
        let at = self.base + self.written;
        flash.erase_sector(at).map_err(|_| "erase")?;
        let n = self.fill.next_multiple_of(4);
        self.sector[self.fill..n].fill(0xFF);
        flash.write(at, &self.sector[..n]).map_err(|_| "program")?;
        self.written += SECTOR as u32;
        self.fill = 0;
        self.sector.fill(0xFF);
        let pct = (u64::from(self.seen()) * 100 / u64::from(self.length.max(1))) as u8;
        UPDATING.store(pct, Ordering::Relaxed);
        Ok(())
    }

    fn seen(&self) -> u32 {
        self.check.as_ref().map_or(self.length, ImageCheck::seen)
    }

    fn finish(&mut self, flash: &mut EspFlash) -> Result<(), &'static str> {
        self.flush(flash)?;
        let check = self.check.take().ok_or("finished")?;
        check.finish().map_err(|e| match e {
            update::ImageError::Short => "short",
            update::ImageError::Hash => "hash",
            update::ImageError::TooLong => "too_long",
        })
    }
}

/// What a GET needs from the runtime: borrowed for one exchange.
pub struct Net<'a> {
    pub stack: Stack<'static>,
    pub tls: &'a Tls<'static>,
    pub tcp_rx: &'a mut [u8],
    pub tcp_tx: &'a mut [u8],
    pub buf: &'a mut [u8],
    pub capture: &'a mut [u8],
}

impl Net<'_> {
    /// One GET of a small file; `sink` receives the body in chunks.
    async fn get(
        &mut self,
        src: &Source,
        path: &str,
        utc: ScreenedObservation,
        max_body: usize,
        mut sink: impl FnMut(&[u8]) -> Result<(), &'static str>,
    ) -> Result<(), &'static str> {
        with_timeout(TIMEOUT, self.exchange(src, path, utc, max_body, &mut sink))
            .await
            .map_err(|_| "timeout")?
    }

    async fn get_image(
        &mut self,
        src: &Source,
        path: &str,
        utc: ScreenedObservation,
        length: usize,
        writer: &mut SlotWriter,
        flash: &mut EspFlash,
    ) -> Result<(), &'static str> {
        let mut sink = |chunk: &[u8]| writer.push(flash, chunk);
        with_timeout(
            IMAGE_TIMEOUT,
            self.exchange(src, path, utc, length, &mut sink),
        )
        .await
        .map_err(|_| "timeout")?
    }

    async fn exchange(
        &mut self,
        src: &Source,
        path: &str,
        utc: ScreenedObservation,
        max_body: usize,
        sink: &mut dyn FnMut(&[u8]) -> Result<(), &'static str>,
    ) -> Result<(), &'static str> {
        let request = format!(
            "GET {path} HTTP/1.0\r\nHost: {}\r\nUser-Agent: overhead-s3/{VERSION} \
             (+https://github.com/chachoblow/overhead)\r\nConnection: close\r\n\r\n",
            src.host
        );
        let (addr, port) = match src.plain {
            Some(ap) => ap,
            None => {
                let addrs = self
                    .stack
                    .dns_query(src.host, DnsQueryType::A)
                    .await
                    .map_err(|_| "dns")?;
                (*addrs.first().ok_or("dns_empty")?, 443)
            }
        };
        let mut socket = TcpSocket::new(self.stack, self.tcp_rx, self.tcp_tx);
        socket.set_timeout(Some(Wait::from_secs(20)));
        socket
            .connect((addr, port))
            .await
            .map_err(|_| "tcp_connect")?;
        if src.plain.is_some() {
            return transfer(&mut socket, request.as_bytes(), self.buf, max_body, sink).await;
        }
        let host = format!("{}\0", src.host);
        let server_name =
            core::ffi::CStr::from_bytes_with_nul(host.as_bytes()).map_err(|_| "host")?;
        let config = ClientSessionConfig {
            ca_chain: Some(Certificate::new(X509::PEM(c_str(OTA_ROOTS))).map_err(|_| "roots")?),
            server_name: Some(server_name),
            // TLS 1.3 encrypts the certificates, which hides them from the
            // 0074 date check (raw.githubusercontent.com prefers 1.3).
            min_version: TlsVersion::Tls1_2,
            max_version: Some(TlsVersion::Tls1_2),
            ..ClientSessionConfig::new()
        };
        let stream = Recording {
            inner: socket,
            buf: &mut *self.capture,
            len: 0,
            cut_after_first_byte: false,
        };
        let mut session =
            Session::new(self.tls.reference(), stream, &SessionConfig::Client(config))
                .map_err(|_| "tls_session")?;
        if session.connect().await.is_err() {
            println!(
                "OTA_TLS ok=false verify=0x{:x}",
                session.tls_verification_details()
            );
            return Err("tls_handshake");
        }
        let now = trusted_now_unix_s(utc);
        let recorded = session.stream().len;
        if let Err(e) = tls_dates::check(&mut session.stream().buf[..recorded], now) {
            println!("OTA_CERT_DATES ok=false error={e:?} now={now}");
            let _ = session.close().await;
            return Err("certificate_dates");
        }
        let r = transfer(&mut session, request.as_bytes(), self.buf, max_body, sink).await;
        let _ = session.close().await;
        r
    }
}

/// Sends `request`, checks the head (status 200, one `Content-Length` up to
/// `max_body`, no transfer coding) and hands exactly that many body bytes to
/// `sink`. A redirect is a failure (0085).
async fn transfer<S: io::Read + io::Write>(
    s: &mut S,
    request: &[u8],
    buf: &mut [u8],
    max_body: usize,
    sink: &mut dyn FnMut(&[u8]) -> Result<(), &'static str>,
) -> Result<(), &'static str> {
    s.write_all(request).await.map_err(|_| "write")?;
    s.flush().await.map_err(|_| "flush")?;
    let mut len = 0;
    let head = loop {
        let n = s
            .read(&mut buf[len..element_fetch::MAX_HEAD])
            .await
            .map_err(|_| "read")?;
        if n == 0 {
            return Err("incomplete_head");
        }
        len += n;
        match element_fetch::parse_head_capped(&buf[..len], max_body) {
            Ok(Some(h)) => break h,
            Ok(None) => {}
            Err(HttpError::Status(code)) => {
                println!("OTA_HTTP status={code}");
                return Err("http_status");
            }
            Err(e) => {
                println!("OTA_HTTP error={e:?}");
                return Err("http_head");
            }
        }
    };
    let mut remaining = head.content_length;
    let first = (len - head.body_start).min(remaining);
    sink(&buf[head.body_start..head.body_start + first])?;
    remaining -= first;
    while remaining > 0 {
        let want = remaining.min(buf.len());
        let n = s.read(&mut buf[..want]).await.map_err(|_| "read")?;
        if n == 0 {
            return Err("truncated");
        }
        sink(&buf[..n])?;
        remaining -= n;
    }
    Ok(())
}
