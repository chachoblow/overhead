//! 0087 setup window, target side: the boot-hold gesture, the BLE setup
//! service (LE Secure Connections, passkey on the Sharp), Wi-Fi scan and
//! test join, and saving the settings record (0084). The rules are
//! `setup_policy`; the bytes are `overhead_setup`.
//!
//! Setup is its own boot mode: the radar, SNTP and fetches do not run, so
//! BLE never shares the radio with a refresh. It ends in a reset: after a
//! save (into the normal run on the new settings, which prepares the
//! coastline for the new site), or, with settings stored, after
//! `SETUP_IDLE_S` without an app (to retry the stored network).
use super::{EspFlash, RESOURCES, SCREEN, SENDER, net, ota, uptime_us};
use crate::{
    element_store,
    settings_store::Wifi,
    setup_policy::{self, BootHold, NetworkList},
};
use core::{
    cell::Cell,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};
use embassy_executor::Spawner;
use embassy_net::{Stack, StackResources};
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use embassy_time::{Duration as Wait, Timer, with_timeout};
use esp_hal::{
    Blocking,
    gpio::{Input, Output},
    peripherals::{BT, WIFI},
    rtc_cntl::Rwdt,
    spi::master::Spi,
};
use esp_println::println;
use esp_radio::wifi::{
    AuthenticationMethod, AuthenticationMethodConfig, Config, ConnectionError, ControllerConfig,
    DisconnectReason, Interface, WifiController, scan::ScanConfig, sta::StationConfig,
};
use overhead_render::setup::{SetupScreen, WaitReason, draw_setup};
use overhead_setup::{
    self as proto, ConfigureResult, Info, Request, Response, Security, SetupReason,
};

/// Survives `software_reset` (not power-on): `JOIN_TIMEOUT_FLAG` when the
/// last boot gave up joining the stored network.
#[esp_hal::ram(unstable(rtc_fast, persistent))]
static mut REQUEST: u32 = 0;

/// Whether the previous boot asked for setup after a join timeout; clears it.
pub fn take_join_timeout() -> bool {
    // SAFETY: single-core access before the scheduler's tasks use it.
    unsafe {
        let v = core::ptr::read_volatile(&raw const REQUEST);
        core::ptr::write_volatile(&raw mut REQUEST, 0);
        v == setup_policy::JOIN_TIMEOUT_FLAG
    }
}

/// The stored network did not join in time: restart into setup.
pub fn restart_after_join_timeout() -> ! {
    println!(
        "SETUP join_timeout after_s={}; restarting into setup",
        setup_policy::JOIN_TIMEOUT_S
    );
    // SAFETY: as above; the reset follows immediately.
    unsafe { core::ptr::write_volatile(&raw mut REQUEST, setup_policy::JOIN_TIMEOUT_FLAG) };
    ota::reset()
}

/// The Sharp before the radar's display task owns it: setup screens with
/// VCOM toggled every 500 ms.
pub struct Panel {
    pub spi: Spi<'static, Blocking>,
    pub cs: Output<'static>,
    vcom: bool,
    toggled_us: u64,
}

impl Panel {
    pub fn new(spi: Spi<'static, Blocking>, cs: Output<'static>) -> Self {
        Self {
            spi,
            cs,
            vcom: false,
            toggled_us: 0,
        }
    }

    fn send(&mut self) {
        self.vcom = !self.vcom;
        self.toggled_us = uptime_us();
        // SAFETY: only one owner of the panel statics exists at a time:
        // this before setup or the radar, then setup's panel task or the
        // radar's display task.
        let (screen, sender) = unsafe { (&*(&raw const SCREEN), &mut *(&raw mut SENDER)) };
        self.cs.set_high();
        let r = sender.send(self.vcom, &screen.pixels, false, |p| self.spi.write(p));
        self.cs.set_low();
        if r.is_err() {
            panic!("setup SPI failed; panel state unknown");
        }
    }

    pub fn show(&mut self, s: &SetupScreen) {
        // SAFETY: see `send`.
        let screen = unsafe { &mut *(&raw mut SCREEN) };
        draw_setup(screen, s).expect("setup render");
        self.send();
    }

    /// Keeps VCOM alternating while the screen is unchanged.
    pub fn tick(&mut self) {
        if uptime_us() - self.toggled_us >= 500_000 {
            self.send();
        }
    }
}

/// How long the knob switch (IO6, pressed = low) is held from boot, with
/// the hold screens at 3 s and 10 s. Returns at once if it isn't pressed.
pub async fn boot_hold(switch: &Input<'static>, panel: &mut Panel, wdt: &mut Rwdt) -> BootHold {
    if switch.is_high() {
        return BootHold::None;
    }
    let started = uptime_us();
    let mut shown = BootHold::None;
    let mut released_since: Option<u64> = None;
    loop {
        wdt.feed();
        let now = uptime_us();
        if switch.is_high() {
            // 50 ms debounce on release.
            let since = *released_since.get_or_insert(now);
            if now - since >= 50_000 {
                let held_ms = (since - started) / 1000;
                let hold = setup_policy::classify_hold(held_ms);
                println!("SETUP boot_hold held_ms={held_ms} hold={hold:?}");
                return hold;
            }
        } else {
            released_since = None;
        }
        let held = setup_policy::classify_hold((now - started) / 1000);
        if held != shown {
            shown = held;
            match held {
                BootHold::Setup => panel.show(&SetupScreen::HoldForSetup),
                BootHold::FactoryReset => panel.show(&SetupScreen::HoldForReset),
                BootHold::None => {}
            }
        } else if shown != BootHold::None {
            panel.tick();
        }
        Timer::after(Wait::from_millis(10)).await;
    }
}

/// Factory reset: settings to defaults (the failed build kept) and both
/// element slots erased.
pub fn factory_reset(
    panel: &mut Panel,
    flash: &mut EspFlash,
    o: &mut ota::Ota,
    elements: Option<(u32, u32)>,
) {
    panel.show(&SetupScreen::Erasing);
    let settings_ok = o.replace_settings(flash, setup_policy::factory_reset(o.settings()));
    let elements_ok = match elements.map(|(off, len)| element_store::Layout::new(off, len)) {
        Some(Ok(layout)) => element_store::erase(flash, &layout).is_ok(),
        _ => false,
    };
    println!("SETUP factory_reset settings_ok={settings_ok} elements_ok={elements_ok}");
}

/// What the panel task shows; `GENERATION` changes on every update.
#[derive(Clone, Copy)]
enum Show {
    Waiting,
    Passkey(u32),
    Paired,
    Testing(Wifi),
    Failed(Wifi, ConfigureResult),
    Saved(Wifi),
}

static SHOW: Mutex<CriticalSectionRawMutex, Cell<Show>> = Mutex::new(Cell::new(Show::Waiting));
static GENERATION: AtomicU32 = AtomicU32::new(0);
static CONNECTED: AtomicBool = AtomicBool::new(false);

fn show(s: Show) {
    SHOW.lock(|c| c.set(s));
    GENERATION.fetch_add(1, Ordering::Relaxed);
}

#[embassy_executor::task]
async fn panel_task(
    mut panel: Panel,
    mut wdt: Rwdt,
    name: &'static str,
    reason: WaitReason,
    stored: Option<Wifi>,
) {
    let mut shown = None;
    loop {
        // 0086: the executor is alive while this task runs.
        wdt.feed();
        let g = GENERATION.load(Ordering::Relaxed);
        if shown != Some(g) {
            shown = Some(g);
            let s = SHOW.lock(|c| c.get());
            let stored_ssid = stored.as_ref().map_or("", |w| w.ssid());
            match s {
                Show::Waiting => panel.show(&SetupScreen::Waiting {
                    name,
                    reason,
                    ssid: stored_ssid,
                }),
                Show::Passkey(code) => panel.show(&SetupScreen::Passkey { code }),
                Show::Paired => panel.show(&SetupScreen::Paired),
                Show::Testing(w) => panel.show(&SetupScreen::Testing { ssid: w.ssid() }),
                Show::Failed(w, r) => panel.show(&SetupScreen::Failed {
                    ssid: w.ssid(),
                    why: setup_policy::failure_phrase(r),
                }),
                Show::Saved(w) => panel.show(&SetupScreen::Saved { ssid: w.ssid() }),
            }
        } else {
            panel.tick();
        }
        Timer::after(Wait::from_millis(50)).await;
    }
}

fn wait_reason(r: SetupReason) -> WaitReason {
    match r {
        SetupReason::Unconfigured => WaitReason::Unconfigured,
        SetupReason::JoinTimeout => WaitReason::JoinTimeout,
        SetupReason::Button => WaitReason::Button,
        SetupReason::FactoryReset => WaitReason::FactoryReset,
    }
}

fn security(auth: Option<AuthenticationMethod>) -> Security {
    use AuthenticationMethod::*;
    match auth {
        Some(None) => Security::Open,
        Some(Wpa | Wpa2Personal | WpaWpa2Personal | Wpa2Wpa3Personal) => Security::Personal,
        _ => Security::Unsupported,
    }
}

fn join_failure(e: &ConnectionError) -> ConfigureResult {
    use DisconnectReason::*;
    match e {
        ConnectionError::Failed(info) => match info.reason {
            NoAccessPointFound
            | NoAccessPointFoundWithCompatibleSecurity
            | NoAccessPointFoundInAuthmodeThreshold
            | NoAccessPointFoundInRssiThreshold => ConfigureResult::NetworkNotFound,
            FourWayHandshakeTimeout
            | HandshakeTimeout
            | MicFailure
            | AuthenticationFailed
            | _802_1xAuthenticationFailed => ConfigureResult::WrongPassphrase,
            _ => ConfigureResult::JoinFailed,
        },
        _ => ConfigureResult::JoinFailed,
    }
}

/// The parts of the device a setup session reads and changes.
struct Device {
    controller: WifiController<'static>,
    stack: Stack<'static>,
    flash: EspFlash,
    ota: ota::Ota,
    elements: Option<(u32, u32)>,
    mac: [u8; 6],
    reason: SetupReason,
    /// CPU0 stack paint, scanned once after the first pairing (P-256).
    painted: Option<crate::stack_watermark::Painted>,
    /// The last scan (made while no app was connected when possible), its
    /// result and uptime.
    cache: NetworkList,
    cache_ok: bool,
    cache_at_us: Option<u64>,
    /// This window's last `Configure` result, for a client that lost the link.
    last_result: Option<ConfigureResult>,
}

impl Device {
    fn info<'a>(&'a mut self, version: &'a str) -> Info<'a> {
        let last_fetch = self
            .elements
            .and_then(|(o, l)| element_store::Layout::new(o, l).ok())
            .and_then(|layout| element_store::newest_fetch(&mut self.flash, &layout));
        let s = self.ota.settings();
        let mut flags = 0;
        if s.wifi.is_some() {
            flags |= Info::WIFI_STORED;
        }
        if s.site.is_some() {
            flags |= Info::SITE_STORED;
        }
        Info {
            build: ota::BUILD,
            version,
            channel: match s.channel {
                overhead_update::Channel::Stable => proto::Channel::Stable,
                overhead_update::Channel::Beta => proto::Channel::Beta,
            },
            mac: self.mac,
            flags,
            reason: self.reason,
            ssid: s.wifi.as_ref().map_or("", |w| w.ssid()),
            last_fetch_unix_s: last_fetch.unwrap_or(0),
            failed_build: s.failed_build.unwrap_or(0),
            last_result: self.last_result,
        }
    }

    /// Rescans into the cache.
    async fn refresh(&mut self) {
        let mut list = NetworkList::new();
        self.cache_ok = self.scan(&mut list).await;
        self.cache = list;
        self.cache_at_us = Some(uptime_us());
    }

    fn cache_age_s(&self) -> Option<u64> {
        self.cache_at_us.map(|t| (uptime_us() - t) / 1_000_000)
    }

    /// Scans into `list`; false if the scan failed.
    async fn scan(&mut self, list: &mut NetworkList) -> bool {
        match self
            .controller
            .scan_async(&ScanConfig::default().with_max(64))
            .await
        {
            Ok(aps) => {
                for ap in &aps {
                    list.offer(
                        ap.ssid.as_str(),
                        ap.signal_strength,
                        security(ap.auth_method),
                        ap.channel,
                    );
                }
                println!("SETUP scan seen={} listed={}", aps.len(), list.len());
                true
            }
            Err(e) => {
                println!("SETUP scan_failed {e:?}");
                false
            }
        }
    }

    /// Joins `wifi` and waits for a DHCP lease, then leaves the network.
    async fn test(&mut self, wifi: &Wifi) -> ConfigureResult {
        let mut config = StationConfig::default()
            .with_ssid(wifi.ssid().try_into().expect("checked SSID"))
            .with_authentication(AuthenticationMethodConfig::Wpa2Personal(
                wifi.passphrase().try_into().expect("checked passphrase"),
            ));
        // The cached channel spares the join a full sweep under the BLE link.
        let channel = self.cache.channel_of(wifi.ssid());
        if let Some(c) = channel {
            config = config.with_channel(c);
        }
        println!("SETUP test channel={channel:?}");
        let station = Config::Station(config);
        if let Err(e) = self.controller.set_config(&station) {
            println!("SETUP test set_config_failed {e:?}");
            return ConfigureResult::JoinFailed;
        }
        let started = uptime_us();
        let result = match with_timeout(Wait::from_secs(30), self.controller.connect_async()).await
        {
            Err(_) => ConfigureResult::JoinFailed,
            Ok(Err(e)) => {
                // The reason only: the disconnect info holds the SSID.
                match &e {
                    ConnectionError::Failed(info) => {
                        println!("SETUP test connect_failed reason={:?}", info.reason)
                    }
                    _ => println!("SETUP test connect_failed wifi_error"),
                }
                join_failure(&e)
            }
            Ok(Ok(_)) => match with_timeout(Wait::from_secs(20), self.stack.wait_config_up()).await
            {
                Ok(()) => ConfigureResult::Saved,
                Err(_) => ConfigureResult::NoAddress,
            },
        };
        println!(
            "SETUP test result={result:?} ms={}",
            (uptime_us() - started) / 1000
        );
        let _ = self.controller.disconnect_async().await;
        result
    }
}

/// Runs the setup window until a save or an idle timeout resets the chip.
#[allow(clippy::too_many_arguments)]
pub async fn run(
    spawner: Spawner,
    panel: Panel,
    wdt: Rwdt,
    flash: EspFlash,
    o: ota::Ota,
    elements: Option<(u32, u32)>,
    reason: SetupReason,
    wifi: WIFI<'static>,
    bt: BT<'static>,
    seed: u64,
    painted: crate::stack_watermark::Painted,
) -> ! {
    let mac = esp_hal::efuse::base_mac_address();
    let mac: [u8; 6] = mac.as_bytes().try_into().expect("6-byte MAC");
    let name: &'static str = {
        static NAME: static_cell::StaticCell<[u8; 13]> = static_cell::StaticCell::new();
        let n = NAME.init(setup_policy::ble_name(mac));
        core::str::from_utf8(n).expect("ASCII name")
    };
    let stored = o.settings().wifi;
    let configured = o.settings().wifi.is_some() && o.settings().site.is_some();
    println!(
        "SETUP open reason={reason:?} name={name} configured={configured} internal_free={}",
        ble::internal_free()
    );
    show(Show::Waiting);
    spawner.spawn(panel_task(panel, wdt, name, wait_reason(reason), stored).unwrap());

    let controller = WifiController::new(
        wifi,
        ControllerConfig::default().with_initial_config(Config::Station(StationConfig::default())),
    )
    .expect("Wi-Fi controller");
    let (stack, runner) = embassy_net::new(
        Interface::station(),
        embassy_net::Config::dhcpv4(Default::default()),
        RESOURCES.init(StackResources::new()),
        seed,
    );
    spawner.spawn(net(runner).unwrap());
    let device = Device {
        controller,
        stack,
        flash,
        ota: o,
        elements,
        mac,
        reason,
        painted: Some(painted),
        cache: NetworkList::new(),
        cache_ok: false,
        cache_at_us: None,
        last_result: None,
    };
    ble::serve(bt, name, mac, device, configured).await
}

/// The BLE side, apart so trouble-host's prelude names stay out of the rest.
mod ble {
    use super::*;
    use embassy_futures::join::join;
    use esp_radio::ble::controller::BleConnector;
    use static_cell::StaticCell;
    use trouble_host::prelude::*;

    // The macros take literal UUIDs; these checks keep them the protocol's.
    const _: () = assert!(proto::str_eq(
        proto::SERVICE_UUID,
        "6f760010-8a3e-4c1d-9b7a-0b5e1c2d3a40"
    ));
    const _: () = assert!(proto::str_eq(
        proto::REQUEST_UUID,
        "6f760011-8a3e-4c1d-9b7a-0b5e1c2d3a40"
    ));
    const _: () = assert!(proto::str_eq(
        proto::RESPONSE_UUID,
        "6f760012-8a3e-4c1d-9b7a-0b5e1c2d3a40"
    ));

    type Controller = ExternalController<BleConnector<'static>, 20>;
    type Resources = HostResources<Controller, DefaultPacketPool, 1, 2, 1, 1>;
    static RESOURCES: StaticCell<Resources> = StaticCell::new();

    #[gatt_server]
    struct Server {
        setup: SetupService,
    }

    /// 0087's one service; both characteristics need a passkey-paired link.
    #[gatt_service(uuid = "6f760010-8a3e-4c1d-9b7a-0b5e1c2d3a40")]
    struct SetupService {
        #[characteristic(
            uuid = "6f760011-8a3e-4c1d-9b7a-0b5e1c2d3a40",
            write,
            permissions(authenticated),
            value = [0; proto::MAX_FRAME]
        )]
        request: [u8; proto::MAX_FRAME],
        #[characteristic(
            uuid = "6f760012-8a3e-4c1d-9b7a-0b5e1c2d3a40",
            notify,
            permissions(authenticated),
            value = [0; proto::MAX_FRAME]
        )]
        response: [u8; proto::MAX_FRAME],
    }

    /// Internal-RAM free bytes now (PSRAM excluded).
    pub fn internal_free() -> usize {
        esp_alloc::HEAP
            .stats()
            .region_stats
            .iter()
            .flatten()
            .filter(|r| {
                r.capabilities
                    .contains(esp_alloc::MemoryCapability::Internal)
            })
            .map(|r| r.free)
            .sum()
    }

    pub async fn serve(
        bt: BT<'static>,
        name: &'static str,
        mac: [u8; 6],
        mut device: Device,
        configured: bool,
    ) -> ! {
        let before = internal_free();
        let connector = BleConnector::new(bt, Default::default()).expect("BLE controller");
        println!(
            "SETUP ble_init internal_free_before={before} internal_free_after={}",
            internal_free()
        );
        // Static random address from the MAC (top two bits set, as BLE requires).
        let mut addr = [mac[5], mac[4], mac[3], mac[2], mac[1], mac[0]];
        addr[5] |= 0xc0;
        let stack = trouble_host::new(
            Controller::new(connector),
            RESOURCES.init(HostResources::new()),
        )
        .set_random_address(Address::random(addr))
        .set_io_capabilities(IoCapabilities::DisplayOnly)
        .build();
        let mut peripheral = stack.peripheral();
        let runner = stack.runner();
        let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
            name,
            appearance: &appearance::UNKNOWN,
        }))
        .expect("GATT server");
        let _ = join(run_host(runner), async {
            let mut idle_since = uptime_us();
            loop {
                let idle_s = (uptime_us() - idle_since) / 1_000_000;
                if configured && idle_s >= setup_policy::SETUP_IDLE_S {
                    println!(
                        "SETUP idle_timeout after_s={}; restarting on the stored settings",
                        setup_policy::SETUP_IDLE_S
                    );
                    ota::reset();
                }
                // Scan while no app is connected (advertising pauses for it).
                if device
                    .cache_age_s()
                    .is_none_or(|a| a >= setup_policy::RESCAN_S)
                {
                    device.refresh().await;
                }
                let mut wait_s = setup_policy::RESCAN_S;
                if configured {
                    wait_s = wait_s.min(setup_policy::SETUP_IDLE_S.saturating_sub(idle_s));
                }
                let accepted = match with_timeout(
                    Wait::from_secs(wait_s.max(1)),
                    advertise(name, &mut peripheral, &server),
                )
                .await
                {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                match accepted {
                    Ok(conn) => {
                        CONNECTED.store(true, Ordering::Relaxed);
                        println!("SETUP ble_connected internal_free={}", internal_free());
                        session(&stack, &server, &conn, &mut device).await;
                        CONNECTED.store(false, Ordering::Relaxed);
                        idle_since = uptime_us();
                        show(Show::Waiting);
                    }
                    Err(e) => {
                        println!("SETUP advertise_failed {e:?}");
                        Timer::after(Wait::from_secs(5)).await;
                    }
                }
            }
        })
        .await;
        unreachable!("BLE host and session loops never end")
    }

    async fn run_host<C: trouble_host::Controller, P: PacketPool>(mut runner: Runner<'_, C, P>) {
        loop {
            if let Err(e) = runner.run().await {
                println!("SETUP ble_host_failed {e:?}");
                Timer::after(Wait::from_secs(1)).await;
            }
        }
    }

    async fn advertise<'v, 's, C: trouble_host::Controller>(
        name: &'v str,
        peripheral: &mut Peripheral<'v, C, DefaultPacketPool>,
        server: &'s Server<'v>,
    ) -> Result<GattConnection<'v, 's, DefaultPacketPool>, BleHostError<C::Error>> {
        let mut adv = [0; 31];
        let len = AdStructure::encode_slice(
            &[
                AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                AdStructure::CompleteLocalName(name.as_bytes()),
            ],
            &mut adv[..],
        )?;
        let uuid = proto::uuid_le(proto::SERVICE_UUID);
        let mut scan = [0; 31];
        let scan_len = AdStructure::encode_slice(
            &[AdStructure::CompleteServiceUuids128(&[uuid])],
            &mut scan[..],
        )?;
        let advertiser = peripheral
            .advertise(
                &Default::default(),
                Advertisement::ConnectableScannableUndirected {
                    adv_data: &adv[..len],
                    scan_data: &scan[..scan_len],
                },
            )
            .await?;
        println!("SETUP advertising name={name}");
        Ok(advertiser.accept().await?.with_attribute_server(server)?)
    }

    async fn notify<P: PacketPool>(
        server: &Server<'_>,
        conn: &GattConnection<'_, '_, P>,
        r: &Response<'_>,
    ) {
        let mut buf = [0u8; proto::MAX_FRAME];
        let Ok(n) = proto::encode_response(r, &mut buf) else {
            println!("SETUP encode_failed");
            return;
        };
        if let Err(e) = server
            .setup
            .response
            .notify_raw(conn, &buf[..n], false)
            .await
        {
            println!("SETUP notify_failed {e:?}");
        }
    }

    /// One phone connection: pairing, then requests until it disconnects.
    async fn session(
        stack: &trouble_host::Stack<'_, Controller, DefaultPacketPool>,
        server: &Server<'_>,
        conn: &GattConnection<'_, '_, DefaultPacketPool>,
        device: &mut Device,
    ) {
        // A Wi-Fi scan or join takes the shared radio for long stretches;
        // BlueZ's default 420 ms supervision timeout drops the link then.
        // Ask for 4 s (Apple's accessory rules: 2–6 s, max interval at least
        // 15 ms above min).
        let params = RequestedConnParams {
            min_connection_interval: embassy_time::Duration::from_millis(30),
            max_connection_interval: embassy_time::Duration::from_millis(50),
            max_latency: 0,
            min_event_length: embassy_time::Duration::from_millis(0),
            max_event_length: embassy_time::Duration::from_millis(0),
            supervision_timeout: embassy_time::Duration::from_secs(4),
        };
        match conn.raw().update_connection_params(stack, &params).await {
            Ok(()) => println!("SETUP conn_params requested supervision_ms=4000"),
            Err(e) => println!("SETUP conn_params_failed {e:?}"),
        }
        // The device's view of the link, to tell weak RF from coexistence.
        match conn.raw().rssi(stack).await {
            Ok(r) => println!("SETUP link rssi_dbm={r}"),
            Err(e) => println!("SETUP link rssi_failed {e:?}"),
        }
        // Ask the phone to pair now, so the code shows at once (a request
        // before pairing is refused by the characteristic permissions).
        if let Err(e) = conn.raw().request_security() {
            println!("SETUP request_security_failed {e:?}");
        }
        let request = server.setup.request.handle;
        let reason = loop {
            match conn.next().await {
                GattConnectionEvent::Disconnected { reason } => break reason,
                GattConnectionEvent::PassKeyDisplay(key) => {
                    #[cfg(feature = "setup-test")]
                    println!("SETUP_PASSKEY {key}");
                    println!("SETUP passkey_shown");
                    show(Show::Passkey(key.value()));
                }
                GattConnectionEvent::PairingComplete {
                    security_level,
                    bond,
                } => {
                    println!(
                        "SETUP paired level={security_level:?} bonded={} internal_free={}",
                        bond.is_some(),
                        internal_free()
                    );
                    show(Show::Paired);
                    if let Some(p) = device.painted.take() {
                        // SAFETY: same CPU0 stack and boot as the paint.
                        let w = unsafe { p.scan() };
                        println!(
                            "SETUP stack_watermark observed_bytes={} stack_bytes={}",
                            w.observed_bytes,
                            w.stack_top - w.stack_low
                        );
                    }
                }
                GattConnectionEvent::ConnectionParamsUpdated {
                    conn_interval,
                    peripheral_latency,
                    supervision_timeout,
                } => {
                    println!(
                        "SETUP conn_params interval_ms={} latency={peripheral_latency} \
                         supervision_ms={}",
                        conn_interval.as_millis(),
                        supervision_timeout.as_millis()
                    );
                }
                GattConnectionEvent::PairingFailed(e) => {
                    println!("SETUP pairing_failed {e:?}");
                    show(Show::Waiting);
                }
                GattConnectionEvent::Gatt { event } => {
                    let mut frame = [0u8; proto::MAX_FRAME];
                    let mut len = None;
                    if let GattEvent::Write(w) = &event
                        && w.handle() == request
                    {
                        len = w.with_data(|offset, data| {
                            (offset == 0 && data.len() <= frame.len()).then(|| {
                                frame[..data.len()].copy_from_slice(data);
                                data.len()
                            })
                        });
                    }
                    match event.accept() {
                        Ok(reply) => reply.send().await,
                        Err(e) => println!("SETUP reply_failed {e:?}"),
                    }
                    if let Some(n) = len {
                        handle(server, conn, device, &frame[..n]).await;
                    }
                }
                _ => {}
            }
        };
        println!("SETUP ble_disconnected reason={reason:?}");
    }

    async fn handle<P: PacketPool>(
        server: &Server<'_>,
        conn: &GattConnection<'_, '_, P>,
        device: &mut Device,
        frame: &[u8],
    ) {
        let request = match proto::decode_request(frame) {
            Ok(f) => f.message,
            Err(e) => {
                println!("SETUP request_refused {e:?}");
                let r = Response::Error {
                    code: e.code(),
                    request_type: proto::type_of(frame),
                };
                notify(server, conn, &r).await;
                return;
            }
        };
        match request {
            Request::GetInfo => {
                println!("SETUP request get_info");
                let info = device.info(ota::VERSION);
                notify(server, conn, &Response::Info(info)).await;
            }
            Request::Scan => {
                println!("SETUP request scan");
                let age = device.cache_age_s();
                if age.is_none_or(|a| a >= setup_policy::SCAN_CACHE_S) || !device.cache_ok {
                    device.refresh().await;
                } else {
                    println!("SETUP scan cached age_s={age:?}");
                }
                let ok = device.cache_ok;
                let mut list = device.cache.clone();
                let count = list.len() as u8;
                for n in list.sorted() {
                    notify(server, conn, &Response::Network(n)).await;
                }
                notify(server, conn, &Response::ScanDone { count, ok }).await;
            }
            Request::Configure(c) => {
                // The site and passphrase are never printed.
                println!("SETUP request configure");
                let (wifi, site) = match setup_policy::accept(&c) {
                    Ok(v) => v,
                    Err(r) => {
                        println!("SETUP configure_refused {r:?}");
                        notify(server, conn, &Response::ConfigureResult(r)).await;
                        return;
                    }
                };
                notify(server, conn, &Response::Testing).await;
                show(Show::Testing(wifi));
                let mut result = device.test(&wifi).await;
                if result == ConfigureResult::Saved {
                    let next = setup_policy::configured(device.ota.settings(), wifi, site);
                    if !device.ota.replace_settings(&mut device.flash, next) {
                        result = ConfigureResult::SaveFailed;
                    }
                }
                device.last_result = Some(result);
                notify(server, conn, &Response::ConfigureResult(result)).await;
                if result == ConfigureResult::Saved {
                    show(Show::Saved(wifi));
                    println!("SETUP saved; restarting on the new settings");
                    // Let the notification and the screen go out.
                    Timer::after(Wait::from_secs(3)).await;
                    ota::reset();
                }
                show(Show::Failed(wifi, result));
            }
        }
    }
}
