//! Throwaway BLE + Wi-Fi coexistence spike (giftable-device proposal, F):
//! decides whether an iOS setup app over BLE can replace the served setup
//! page. Runs inside the `live-fetch-radar` diagnostic, so radar, SNTP and
//! HTTPS fetches run as usual while a BLE GATT peripheral advertises
//! `OVERHEAD-xxxx` and accepts writes to one characteristic.
//!
//! Evidence it prints (UART only; never a device image):
//! - `BLE_INIT` internal heap free before and after the controller starts;
//! - `BLE_MEM` every 60 s: lowest internal free seen (sampled every 50 ms),
//!   now, and whether a phone is connected;
//! - `BLE_CONNECTED` / `BLE_DISCONNECTED` and `BLE_WRITE len=` (the written
//!   bytes are never printed: a tester could type a real password).
//!
//! No pairing or encryption: a production setup service needs both (a code
//! shown on the Sharp), which this spike does not measure.
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_time::{Duration, Instant, Timer};
use esp_hal::peripherals::BT;
use esp_println::println;
use esp_radio::ble::controller::BleConnector;
use static_cell::StaticCell;
use trouble_host::prelude::*;

/// Lowest internal-heap free seen since boot (bytes), sampled by `monitor`.
static INTERNAL_FREE_MIN: AtomicUsize = AtomicUsize::new(usize::MAX);
static CONNECTED: AtomicBool = AtomicBool::new(false);

type Controller = ExternalController<BleConnector<'static>, 20>;
type Resources = HostResources<Controller, DefaultPacketPool, 1, 2>;
static RESOURCES: StaticCell<Resources> = StaticCell::new();
static NAME: StaticCell<heapless_name::Name> = StaticCell::new();

mod heapless_name {
    /// `OVERHEAD-xxxx`: last two MAC bytes in hex, as the setup network would be.
    pub struct Name(pub [u8; 13]);
    impl Name {
        pub fn as_str(&self) -> &str {
            core::str::from_utf8(&self.0).unwrap_or("OVERHEAD")
        }
    }
}

#[gatt_server]
struct Server {
    setup: SetupService,
}

/// Placeholder setup service: one writable field standing in for the Wi-Fi
/// and location payload, one status string the phone can read.
#[gatt_service(uuid = "6f760001-8a3e-4c1d-9b7a-0b5e1c2d3a40")]
struct SetupService {
    #[characteristic(uuid = "6f760002-8a3e-4c1d-9b7a-0b5e1c2d3a40", write, read)]
    payload: HeaplessString<96>,
    #[characteristic(uuid = "6f760003-8a3e-4c1d-9b7a-0b5e1c2d3a40", read, notify)]
    status: HeaplessString<32>,
}

/// Internal-RAM free bytes now, from the region stats (PSRAM excluded).
pub fn internal_free() -> usize {
    let stats = esp_alloc::HEAP.stats();
    stats
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

/// Starts the BLE controller and the spike's tasks. Call after the Wi-Fi
/// controller exists (coexistence is initialized with the radio).
pub fn start(spawner: Spawner, bt: BT<'static>) {
    let before = internal_free();
    let connector = BleConnector::new(bt, Default::default()).expect("BLE controller");
    let after = internal_free();
    println!(
        "BLE_INIT internal_free_before={before} internal_free_after={after} cost={}",
        before.saturating_sub(after)
    );
    let mac = esp_hal::efuse::base_mac_address();
    let m = mac.as_bytes();
    let mut name = *b"OVERHEAD-0000";
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for (i, b) in [m[4], m[5]].iter().enumerate() {
        name[9 + 2 * i] = HEX[usize::from(b >> 4)];
        name[10 + 2 * i] = HEX[usize::from(b & 0xf)];
    }
    let name = NAME.init(heapless_name::Name(name));
    // Static random address from the MAC (top two bits set, as BLE requires).
    let mut addr = [m[5], m[4], m[3], m[2], m[1], m[0]];
    addr[5] |= 0xc0;
    spawner.spawn(monitor().unwrap());
    spawner.spawn(ble(Controller::new(connector), Address::random(addr), name.as_str()).unwrap());
}

#[embassy_executor::task]
async fn monitor() {
    let mut next_print = Instant::now() + Duration::from_secs(60);
    loop {
        INTERNAL_FREE_MIN.fetch_min(internal_free(), Ordering::Relaxed);
        if Instant::now() >= next_print {
            next_print += Duration::from_secs(60);
            println!(
                "BLE_MEM internal_free_min={} internal_free_now={} connected={}",
                INTERNAL_FREE_MIN.load(Ordering::Relaxed),
                internal_free(),
                CONNECTED.load(Ordering::Relaxed)
            );
        }
        Timer::after(Duration::from_millis(50)).await;
    }
}

#[embassy_executor::task]
async fn ble(controller: Controller, address: Address, name: &'static str) {
    let resources = RESOURCES.init(HostResources::new());
    let stack = trouble_host::new(controller, resources)
        .set_random_address(address)
        .build();
    let mut peripheral = stack.peripheral();
    let runner = stack.runner();
    let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name,
        appearance: &appearance::UNKNOWN,
    }))
    .expect("GATT server");
    println!(
        "BLE_READY name={name} internal_free={}",
        internal_free()
    );
    let _ = join(run_host(runner), async {
        loop {
            match advertise(name, &mut peripheral, &server).await {
                Ok(conn) => {
                    CONNECTED.store(true, Ordering::Relaxed);
                    println!("BLE_CONNECTED internal_free={}", internal_free());
                    let _ = server.setup.status.set(&server, &HeaplessString::try_from("waiting").unwrap_or_default());
                    events(&server, &conn).await;
                    CONNECTED.store(false, Ordering::Relaxed);
                }
                Err(e) => {
                    println!("BLE_ADVERTISE_FAILED {e:?}");
                    Timer::after(Duration::from_secs(5)).await;
                }
            }
        }
    })
    .await;
}

async fn run_host<C: trouble_host::Controller, P: PacketPool>(mut runner: Runner<'_, C, P>) {
    loop {
        if let Err(e) = runner.run().await {
            println!("BLE_HOST_FAILED {e:?}");
            Timer::after(Duration::from_secs(1)).await;
        }
    }
}

async fn events<P: PacketPool>(server: &Server<'_>, conn: &GattConnection<'_, '_, P>) {
    let payload = server.setup.payload.handle;
    let status = &server.setup.status;
    let reason = loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => break reason,
            GattConnectionEvent::Gatt { event } => {
                let wrote = match &event {
                    GattEvent::Write(w) if w.handle() == payload => {
                        Some(w.with_data(|offset, data| (offset, data.len())))
                    }
                    _ => None,
                };
                match event.accept() {
                    Ok(reply) => reply.send().await,
                    Err(e) => println!("BLE_REPLY_FAILED {e:?}"),
                }
                if let Some((offset, len)) = wrote {
                    println!(
                        "BLE_WRITE offset={offset} len={len} internal_free={}",
                        internal_free()
                    );
                    let mut text: HeaplessString<32> = HeaplessString::new();
                    let _ = core::fmt::write(&mut text, format_args!("received {len} bytes"));
                    // Stores the value too, so a later read sees it.
                    let _ = status.notify(conn, &text, true).await;
                }
            }
            _ => {}
        }
    };
    println!("BLE_DISCONNECTED reason={reason:?}");
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
    let advertiser = peripheral
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data: &adv[..len],
                scan_data: &[],
            },
        )
        .await?;
    println!("BLE_ADVERTISING");
    Ok(advertiser.accept().await?.with_attribute_server(server)?)
}
