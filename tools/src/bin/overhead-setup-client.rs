//! `overhead-setup-client`: drives a device's BLE setup service (0087) from
//! a Linux host through BlueZ, with the same protocol crate as the firmware.
//! A test tool for devbox, not the setup app. Usage: tools/README.md.
//!
//! The passphrase and site are never printed.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("overhead-setup-client needs Linux (BlueZ)");
    std::process::exit(2);
}

#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    match rt.block_on(linux::main()) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::ExitCode::from(1)
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use bluer::{
        Adapter, AdapterEvent, Address, Device, Session, Uuid,
        agent::{Agent, ReqError, RequestPasskey},
        gatt::{
            WriteOp,
            remote::{Characteristic, CharacteristicWriteRequest},
        },
    };
    use futures::{Stream, StreamExt, pin_mut};
    use overhead_setup::{self as proto, Configure, Request, Response};
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        pin::Pin,
        process::ExitCode,
        time::Duration,
    };
    use tokio::time::{Instant, sleep, timeout};

    type Error = Box<dyn std::error::Error>;

    const USAGE: &str = "usage: overhead-setup-client [--name OVERHEAD-xxxx] \
        [--passkey NNNNNN | --passkey-log UART_LOG] <command>
commands:
  info                         GetInfo
  scan                         Scan (networks the device sees)
  configure --wifi-env FILE --site-env FILE [--passphrase P]
                               Configure from firmware/.wifi.env/.site.env
                               (--passphrase overrides, e.g. a wrong one)
  raw HEX                      send one frame, print the answers for 5 s
Without --passkey/--passkey-log the passkey is read from stdin.";

    enum Passkey {
        Fixed(u32),
        /// The newest `SETUP_PASSKEY nnnnnn` line written to the UART log
        /// after this client started (a `setup-test` image prints it).
        Log {
            path: PathBuf,
            from: u64,
        },
        Stdin,
    }

    enum Command {
        Info,
        Scan,
        Configure {
            ssid: String,
            passphrase: String,
            lat: f64,
            lon: f64,
            alt: f64,
        },
        Raw(Vec<u8>),
    }

    fn env_file(path: &Path) -> Result<HashMap<String, String>, Error> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(text
            .lines()
            .filter_map(|l| {
                let l = l.trim();
                if l.starts_with('#') {
                    return None;
                }
                let (k, v) = l.split_once('=')?;
                let v = v.trim().trim_matches('"').trim_matches('\'');
                Some((k.trim().to_owned(), v.to_owned()))
            })
            .collect())
    }

    fn hex(s: &str) -> Result<Vec<u8>, Error> {
        let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        if !s.len().is_multiple_of(2) {
            return Err("odd hex length".into());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(Into::into))
            .collect()
    }

    fn parse() -> Result<(Option<String>, Passkey, Command), Error> {
        let mut args = std::env::args().skip(1);
        let (mut name, mut passkey) = (None, Passkey::Stdin);
        let mut words = Vec::new();
        while let Some(a) = args.next() {
            let mut value = || args.next().ok_or_else(|| format!("{a} needs a value"));
            match a.as_str() {
                "--name" => name = Some(value()?),
                "--passkey" => passkey = Passkey::Fixed(value()?.parse()?),
                "--passkey-log" => {
                    let path = PathBuf::from(value()?);
                    let from = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    passkey = Passkey::Log { path, from };
                }
                "-h" | "--help" => return Err(USAGE.into()),
                _ => words.push(a),
            }
        }
        let mut w = words.into_iter();
        let command = match w.next().as_deref() {
            Some("info") => Command::Info,
            Some("scan") => Command::Scan,
            Some("raw") => Command::Raw(hex(&w.next().ok_or("raw needs HEX")?)?),
            Some("configure") => {
                let (mut wifi, mut site, mut pass) = (None, None, None);
                while let Some(a) = w.next() {
                    let v = w.next().ok_or_else(|| format!("{a} needs a value"))?;
                    match a.as_str() {
                        "--wifi-env" => wifi = Some(env_file(Path::new(&v))?),
                        "--site-env" => site = Some(env_file(Path::new(&v))?),
                        "--passphrase" => pass = Some(v),
                        _ => return Err(format!("unknown configure option {a}").into()),
                    }
                }
                let wifi = wifi.ok_or("configure needs --wifi-env")?;
                let site = site.ok_or("configure needs --site-env")?;
                let get = |m: &HashMap<String, String>, k: &str| {
                    m.get(k).cloned().ok_or_else(|| format!("missing {k}"))
                };
                Command::Configure {
                    ssid: get(&wifi, "SSID")?,
                    passphrase: match pass {
                        Some(p) => p,
                        None => get(&wifi, "PASSWORD")?,
                    },
                    lat: get(&site, "SITE_LAT_DEG")?.parse()?,
                    lon: get(&site, "SITE_LON_DEG")?.parse()?,
                    alt: site
                        .get("SITE_ALT_M")
                        .filter(|v| !v.is_empty())
                        .map_or(Ok(0.0), |v| v.parse())?,
                }
            }
            _ => return Err(USAGE.into()),
        };
        Ok((name, passkey, command))
    }

    async fn passkey_from_log(path: &Path, from: u64) -> Option<u32> {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            if let Ok(bytes) = tokio::fs::read(path).await {
                let new = bytes.get(from as usize..).unwrap_or(&[]);
                let text = String::from_utf8_lossy(new);
                if let Some(code) = text
                    .lines()
                    .filter_map(|l| l.split("SETUP_PASSKEY ").nth(1))
                    .filter_map(|c| c.trim().get(..6)?.parse().ok())
                    .last()
                {
                    return Some(code);
                }
            }
            sleep(Duration::from_millis(200)).await;
        }
        None
    }

    fn agent(passkey: Passkey) -> Agent {
        let passkey = std::sync::Arc::new(passkey);
        Agent {
            request_default: true,
            request_passkey: Some(Box::new(move |req: RequestPasskey| {
                let passkey = passkey.clone();
                Box::pin(async move {
                    println!("PAIRING passkey requested by {}", req.device);
                    let code = match &*passkey {
                        Passkey::Fixed(c) => Some(*c),
                        Passkey::Log { path, from } => passkey_from_log(path, *from).await,
                        Passkey::Stdin => {
                            use tokio::io::AsyncBufReadExt;
                            println!("type the code on the Sharp:");
                            let mut line = String::new();
                            let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
                            stdin.read_line(&mut line).await.ok();
                            line.trim().parse().ok()
                        }
                    };
                    code.ok_or(ReqError::Rejected)
                })
            })),
            ..Default::default()
        }
    }

    async fn find(adapter: &Adapter, name: Option<&str>) -> Result<Device, Error> {
        let events = adapter.discover_devices().await?;
        pin_mut!(events);
        let found = timeout(Duration::from_secs(30), async {
            while let Some(e) = events.next().await {
                if let AdapterEvent::DeviceAdded(addr) = e {
                    let d = adapter.device(addr)?;
                    let n = d.name().await?.unwrap_or_default();
                    let matches = match name {
                        Some(want) => n == want,
                        None => n.starts_with(proto::NAME_PREFIX),
                    };
                    if matches {
                        println!("FOUND {n} {addr} rssi={:?}", d.rssi().await?);
                        return Ok::<Device, Error>(d);
                    }
                }
            }
            Err("discovery ended".into())
        })
        .await
        .map_err(|_| "no OVERHEAD device found in 30 s")??;
        Ok(found)
    }

    async fn connect(d: &Device) -> Result<(), Error> {
        for attempt in 1..=5 {
            let started = Instant::now();
            match timeout(Duration::from_secs(15), d.connect()).await {
                Ok(Ok(())) => {
                    println!(
                        "CONNECTED attempt={attempt} ms={}",
                        started.elapsed().as_millis()
                    );
                    return Ok(());
                }
                Ok(Err(e)) => println!("CONNECT_FAILED attempt={attempt} {e}"),
                Err(_) => println!("CONNECT_FAILED attempt={attempt} timeout"),
            }
            sleep(Duration::from_secs(1)).await;
        }
        Err("could not connect".into())
    }

    async fn pair(d: &Device) -> Result<(), Error> {
        let started = Instant::now();
        // The device asks to pair on connect; BlueZ may already be pairing.
        if !d.is_paired().await?
            && let Err(e) = d.pair().await
        {
            println!("PAIR_CALL {e}");
        }
        while !d.is_paired().await? {
            if started.elapsed() > Duration::from_secs(40) {
                return Err("pairing did not complete".into());
            }
            sleep(Duration::from_millis(200)).await;
        }
        println!("PAIRED ms={}", started.elapsed().as_millis());
        Ok(())
    }

    async fn characteristics(d: &Device) -> Result<(Characteristic, Characteristic), Error> {
        let started = Instant::now();
        while !d.is_services_resolved().await? {
            if started.elapsed() > Duration::from_secs(20) {
                return Err("services not resolved".into());
            }
            sleep(Duration::from_millis(100)).await;
        }
        let (service, request, response) = (
            Uuid::parse_str(proto::SERVICE_UUID)?,
            Uuid::parse_str(proto::REQUEST_UUID)?,
            Uuid::parse_str(proto::RESPONSE_UUID)?,
        );
        for s in d.services().await? {
            if s.uuid().await? != service {
                continue;
            }
            let (mut req, mut resp) = (None, None);
            for c in s.characteristics().await? {
                let u = c.uuid().await?;
                if u == request {
                    req = Some(c);
                } else if u == response {
                    resp = Some(c);
                }
            }
            return Ok((
                req.ok_or("no request characteristic")?,
                resp.ok_or("no response characteristic")?,
            ));
        }
        Err("no setup service".into())
    }

    fn describe(r: &Response) -> String {
        match r {
            Response::Info(i) => format!(
                "INFO build={} version={} channel={:?} mac={} flags=0b{:02b} reason={:?} \
                 ssid_stored={} last_fetch_unix_s={} failed_build={} last_result={:?}",
                i.build,
                i.version,
                i.channel,
                i.mac
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect::<Vec<_>>()
                    .join(":"),
                i.flags,
                i.reason,
                !i.ssid.is_empty(),
                i.last_fetch_unix_s,
                i.failed_build,
                i.last_result
            ),
            Response::Network(n) => {
                format!(
                    "NETWORK rssi={} security={:?} ssid={:?}",
                    n.rssi_dbm, n.security, n.ssid
                )
            }
            Response::ScanDone { count, ok } => format!("SCAN_DONE count={count} ok={ok}"),
            Response::Testing => "TESTING".into(),
            Response::ConfigureResult(c) => format!("CONFIGURE_RESULT {c:?}"),
            Response::Error { code, request_type } => {
                format!("ERROR code={code:?} request_type=0x{request_type:02x}")
            }
        }
    }

    /// Whether `r` ends the exchange for `request_type`.
    fn ends(request_type: u8, r: &Response) -> bool {
        match r {
            Response::Error { .. } => true,
            Response::Info(_) => request_type == 0x01,
            Response::ScanDone { .. } => request_type == 0x02,
            Response::ConfigureResult(_) => request_type == 0x03,
            _ => false,
        }
    }

    async fn exchange(
        req: &Characteristic,
        notes: &mut Pin<Box<dyn Stream<Item = Vec<u8>>>>,
        frame: &[u8],
        wait: Duration,
    ) -> Result<Option<String>, Error> {
        let started = Instant::now();
        let write = CharacteristicWriteRequest {
            op_type: WriteOp::Request,
            ..Default::default()
        };
        req.write_ext(frame, &write).await?;
        let ty = proto::type_of(frame);
        let mut last = None;
        loop {
            let left = wait.saturating_sub(started.elapsed());
            let Ok(Some(bytes)) = timeout(left, notes.next()).await else {
                println!("NO_MORE_ANSWERS after_ms={}", started.elapsed().as_millis());
                return Ok(last);
            };
            match proto::decode_response(&bytes) {
                Ok(f) => {
                    let line = describe(&f.message);
                    println!(
                        "{line} v={} ms={}",
                        f.version,
                        started.elapsed().as_millis()
                    );
                    if ends(ty, &f.message) {
                        return Ok(Some(line));
                    }
                    last = Some(line);
                }
                Err(e) => println!("UNDECODABLE {e:?} bytes={bytes:02x?}"),
            }
        }
    }

    pub async fn main() -> Result<ExitCode, Error> {
        let (name, passkey, command) = parse()?;
        let session = Session::new().await?;
        let adapter = session.default_adapter().await?;
        adapter.set_powered(true).await?;
        let _agent = session.register_agent(agent(passkey)).await?;
        let device = find(&adapter, name.as_deref()).await?;
        let addr: Address = device.address();
        let result = run(&device, command).await;
        let _ = device.disconnect().await;
        // Forget the device so every run pairs afresh (the device keeps no bond).
        let _ = adapter.remove_device(addr).await;
        result
    }

    async fn run(device: &Device, command: Command) -> Result<ExitCode, Error> {
        connect(device).await?;
        pair(device).await?;
        let (req, resp) = characteristics(device).await?;
        println!(
            "MTU request={} response={}",
            req.mtu().await?,
            resp.mtu().await?
        );
        let mut notes: Pin<Box<dyn Stream<Item = Vec<u8>>>> = Box::pin(resp.notify().await?);
        let mut buf = [0u8; proto::MAX_FRAME];
        let (frame, wait) = match &command {
            Command::Info => (encode(&Request::GetInfo, &mut buf)?, 10),
            Command::Scan => (encode(&Request::Scan, &mut buf)?, 30),
            Command::Configure {
                ssid,
                passphrase,
                lat,
                lon,
                alt,
            } => {
                let c = Configure {
                    ssid,
                    passphrase,
                    lat_deg: *lat,
                    lon_deg: *lon,
                    alt_m: *alt,
                };
                println!(
                    "CONFIGURE ssid_len={} (ssid, passphrase and site not printed)",
                    ssid.len()
                );
                (encode(&Request::Configure(c), &mut buf)?, 90)
            }
            Command::Raw(bytes) => (bytes.clone(), 5),
        };
        let last = exchange(&req, &mut notes, &frame, Duration::from_secs(wait)).await?;
        let ok = match command {
            Command::Raw(_) => true,
            _ => last.is_some_and(|l| !l.starts_with("ERROR")),
        };
        Ok(if ok {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(3)
        })
    }

    fn encode(r: &Request, buf: &mut [u8]) -> Result<Vec<u8>, Error> {
        let n = proto::encode_request(r, buf).map_err(|_| "frame does not fit")?;
        Ok(buf[..n].to_vec())
    }
}
