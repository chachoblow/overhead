#![no_std]
//! BLE setup protocol ([0087](../../docs/decisions/0087-ios-setup-app-over-ble.md)),
//! shared by the device and the host test client; the iOS app implements the
//! same bytes (the vectors in the tests are its reference). no_std, no alloc,
//! no I/O.
//!
//! **Transport.** One GATT service, [`SERVICE_UUID`]. The client writes
//! request frames to [`REQUEST_UUID`] (write with response); the device sends
//! response frames as notifications on [`RESPONSE_UUID`]. Both need an
//! authenticated (passkey-paired, LE Secure Connections) link. One ATT value
//! is one frame, so frames carry no length; a frame is at most [`MAX_FRAME`]
//! bytes, which needs an ATT MTU of at least `MAX_FRAME + 3`.
//!
//! **Frame.** `version u8, type u8, payload`. `version` is the sender's
//! protocol version ([`VERSION`]). Requests use types `0x01..=0x7F`,
//! responses `0x80..=0xFF`. Integers are little-endian, floats IEEE-754
//! little-endian `f64`, strings a `u8` byte length then UTF-8.
//!
//! **Compatibility.** Fields are only ever appended: a reader ignores bytes
//! after the fields it knows, and a newer version may add fields or types.
//! The device answers a request type it doesn't know with
//! [`Response::Error`] ([`ErrorCode::UnknownType`]); the client reads the
//! device's version from [`Info`] and offers only what that version has, and
//! ignores response types it doesn't know. An incompatible change gets a new
//! service UUID, never a reuse of these.
//!
//! **Exchange.** `GetInfo` → `Info`. `Scan` → zero or more `Network`, then
//! `ScanDone`. `Configure` → `Testing`, then `ConfigureResult`; on
//! [`ConfigureResult::Saved`] the device leaves setup and restarts on the new
//! settings. One request at a time: a request while another is running gets
//! [`ErrorCode::Busy`].

/// This protocol version.
pub const VERSION: u8 = 1;
/// Largest frame either side sends (iOS's usual ATT MTU is 185).
pub const MAX_FRAME: usize = 180;

pub const SERVICE_UUID: &str = "6f760010-8a3e-4c1d-9b7a-0b5e1c2d3a40";
/// Client → device: request frames.
pub const REQUEST_UUID: &str = "6f760011-8a3e-4c1d-9b7a-0b5e1c2d3a40";
/// Device → client: response frames (notify).
pub const RESPONSE_UUID: &str = "6f760012-8a3e-4c1d-9b7a-0b5e1c2d3a40";

/// Advertised name prefix; the device appends the last two MAC bytes in hex.
pub const NAME_PREFIX: &str = "OVERHEAD-";

pub const SSID_MAX: usize = 32;
pub const PASSPHRASE_MAX: usize = 64;
pub const VERSION_MAX: usize = 32;

mod ty {
    pub const GET_INFO: u8 = 0x01;
    pub const SCAN: u8 = 0x02;
    pub const CONFIGURE: u8 = 0x03;
    pub const INFO: u8 = 0x81;
    pub const NETWORK: u8 = 0x82;
    pub const SCAN_DONE: u8 = 0x83;
    pub const TESTING: u8 = 0x84;
    pub const CONFIGURE_RESULT: u8 = 0x85;
    pub const ERROR: u8 = 0x86;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Request<'a> {
    GetInfo,
    Scan,
    Configure(Configure<'a>),
}

impl Request<'_> {
    pub fn type_byte(&self) -> u8 {
        match self {
            Request::GetInfo => ty::GET_INFO,
            Request::Scan => ty::SCAN,
            Request::Configure(_) => ty::CONFIGURE,
        }
    }
}

/// The settings a client sends. Payload: SSID, passphrase, latitude (deg,
/// north positive), longitude (deg, east positive), altitude (m above the
/// WGS-84 ellipsoid).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Configure<'a> {
    pub ssid: &'a str,
    pub passphrase: &'a str,
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub alt_m: f64,
}

impl Configure<'_> {
    /// The device's acceptance rules (also the settings record's, 0084):
    /// SSID 1..=32 bytes; WPA2-Personal passphrase of 8..=63 characters or
    /// 64 hex digits; latitude −90..=90, longitude −180..=180, altitude
    /// −500..=9000 m, all finite. A client should check before sending.
    pub fn check(&self) -> Result<(), ConfigureResult> {
        let (s, p) = (self.ssid.as_bytes(), self.passphrase.as_bytes());
        if s.is_empty() || s.len() > SSID_MAX {
            return Err(ConfigureResult::InvalidSsid);
        }
        let pass_ok =
            (8..=63).contains(&p.len()) || (p.len() == 64 && p.iter().all(u8::is_ascii_hexdigit));
        if !pass_ok {
            return Err(ConfigureResult::InvalidPassphrase);
        }
        let in_range = |v: f64, lo: f64, hi: f64| v.is_finite() && (lo..=hi).contains(&v);
        if !in_range(self.lat_deg, -90.0, 90.0)
            || !in_range(self.lon_deg, -180.0, 180.0)
            || !in_range(self.alt_m, -500.0, 9_000.0)
        {
            return Err(ConfigureResult::InvalidSite);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Response<'a> {
    Info(Info<'a>),
    Network(Network<'a>),
    /// After the last `Network`: how many were sent, and whether the scan
    /// itself worked (`false`: the list is empty because scanning failed).
    ScanDone {
        count: u8,
        ok: bool,
    },
    /// The device is trying the new network (it can take ~20 s).
    Testing,
    ConfigureResult(ConfigureResult),
    /// `request_type` is the type byte of the request it answers (0 when the
    /// frame was too short to have one).
    Error {
        code: ErrorCode,
        request_type: u8,
    },
}

/// Device status. Payload: build u32, version string, channel u8, MAC 6
/// bytes, flags u8, setup reason u8, stored SSID string (empty if none), last
/// successful element fetch (Unix seconds, 0 = none) i64, last build that
/// failed its health check (0 = none) u32.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Info<'a> {
    pub build: u32,
    pub version: &'a str,
    pub channel: Channel,
    pub mac: [u8; 6],
    /// Bit 0: Wi-Fi stored; bit 1: site stored. Other bits reserved (0).
    pub flags: u8,
    pub reason: SetupReason,
    pub ssid: &'a str,
    pub last_fetch_unix_s: i64,
    pub failed_build: u32,
}

impl Info<'_> {
    pub const WIFI_STORED: u8 = 1;
    pub const SITE_STORED: u8 = 2;
}

/// A network from the device's scan. Payload: RSSI i8 (dBm), security u8,
/// SSID string.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Network<'a> {
    pub rssi_dbm: i8,
    pub security: Security,
    pub ssid: &'a str,
}

macro_rules! byte_enum {
    ($(#[$m:meta])* $name:ident { $($(#[$vm:meta])* $v:ident = $n:literal),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $name { $($(#[$vm])* $v = $n),+ }
        impl $name {
            pub fn from_byte(b: u8) -> Option<Self> {
                match b { $($n => Some(Self::$v),)+ _ => None }
            }
            pub fn byte(self) -> u8 { self as u8 }
        }
    };
}

byte_enum!(
    /// Update channel (0085).
    Channel { Stable = 0, Beta = 1 }
);

byte_enum!(
    /// Why the setup window is open.
    SetupReason {
        /// No Wi-Fi or site stored.
        Unconfigured = 1,
        /// The stored network did not join within 5 minutes of boot.
        JoinTimeout = 2,
        /// The knob switch was held 3 s at boot.
        Button = 3,
        /// Factory reset (switch held 10 s at boot).
        FactoryReset = 4,
    }
);

byte_enum!(
    /// What a scanned network needs. Only `Personal` can be configured.
    Security {
        /// WPA/WPA2-Personal (or WPA2/WPA3 transition): supported.
        Personal = 0,
        /// No passphrase: not supported.
        Open = 1,
        /// WEP, WPA3-only, Enterprise, …: not supported.
        Unsupported = 2,
    }
);

byte_enum!(
    ConfigureResult {
        /// Joined, got an address, saved; the device restarts on it.
        Saved = 0,
        InvalidSsid = 1,
        InvalidPassphrase = 2,
        InvalidSite = 3,
        /// The network wasn't seen.
        NetworkNotFound = 4,
        /// The access point refused the passphrase.
        WrongPassphrase = 5,
        /// Joined but got no address (DHCP).
        NoAddress = 6,
        /// Any other join failure or timeout.
        JoinFailed = 7,
        /// The test passed but the settings could not be written.
        SaveFailed = 8,
    }
);

byte_enum!(
    ErrorCode {
        /// Truncated, bad UTF-8, a string too long, version 0, or a request
        /// type in the response range.
        Malformed = 1,
        /// A request type this device doesn't know (a newer client).
        UnknownType = 2,
        /// Another request is still running.
        Busy = 3,
    }
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// Fewer than two bytes, or a field cut short.
    Truncated,
    /// Version byte 0.
    Version,
    /// A type this reader doesn't know (or from the wrong direction).
    UnknownType(u8),
    /// A string that isn't UTF-8 or is longer than its limit.
    BadString,
    /// An enum byte outside its values.
    BadValue,
}

impl DecodeError {
    /// The error code a device answers with.
    pub fn code(self) -> ErrorCode {
        match self {
            DecodeError::UnknownType(t) if t < 0x80 => ErrorCode::UnknownType,
            _ => ErrorCode::Malformed,
        }
    }
}

/// The frame did not fit the buffer, or a string exceeds its limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncodeError;

/// A decoded frame: the sender's version and the message.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame<M> {
    pub version: u8,
    pub message: M,
}

struct Writer<'b> {
    buf: &'b mut [u8],
    len: usize,
}

impl<'b> Writer<'b> {
    fn new(buf: &'b mut [u8], ty: u8) -> Result<Self, EncodeError> {
        let mut w = Self { buf, len: 0 };
        w.bytes(&[VERSION, ty])?;
        Ok(w)
    }
    fn bytes(&mut self, b: &[u8]) -> Result<(), EncodeError> {
        let end = self.len + b.len();
        if end > self.buf.len().min(MAX_FRAME) {
            return Err(EncodeError);
        }
        self.buf[self.len..end].copy_from_slice(b);
        self.len = end;
        Ok(())
    }
    fn u8(&mut self, v: u8) -> Result<(), EncodeError> {
        self.bytes(&[v])
    }
    fn str(&mut self, s: &str, max: usize) -> Result<(), EncodeError> {
        if s.len() > max {
            return Err(EncodeError);
        }
        self.u8(s.len() as u8)?;
        self.bytes(s.as_bytes())
    }
}

struct Reader<'a> {
    b: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if self.b.len() < n {
            return Err(DecodeError::Truncated);
        }
        let (head, rest) = self.b.split_at(n);
        self.b = rest;
        Ok(head)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let mut a = [0; N];
        a.copy_from_slice(self.take(N)?);
        Ok(a)
    }
    fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }
    fn str(&mut self, max: usize) -> Result<&'a str, DecodeError> {
        let n = usize::from(self.u8()?);
        let raw = self.take(n)?;
        if n > max {
            return Err(DecodeError::BadString);
        }
        core::str::from_utf8(raw).map_err(|_| DecodeError::BadString)
    }
    fn f64(&mut self) -> Result<f64, DecodeError> {
        Ok(f64::from_le_bytes(self.array()?))
    }
    fn enum_byte<T>(&mut self, f: fn(u8) -> Option<T>) -> Result<T, DecodeError> {
        f(self.u8()?).ok_or(DecodeError::BadValue)
    }
}

fn header(frame: &[u8]) -> Result<(u8, u8, Reader<'_>), DecodeError> {
    let [version, ty, rest @ ..] = frame else {
        return Err(DecodeError::Truncated);
    };
    if *version == 0 {
        return Err(DecodeError::Version);
    }
    Ok((*version, *ty, Reader { b: rest }))
}

/// The type byte of a frame, if it has one (for [`Response::Error`]).
pub fn type_of(frame: &[u8]) -> u8 {
    frame.get(1).copied().unwrap_or(0)
}

pub fn encode_request(r: &Request, buf: &mut [u8]) -> Result<usize, EncodeError> {
    let mut w = Writer::new(buf, r.type_byte())?;
    if let Request::Configure(c) = r {
        w.str(c.ssid, SSID_MAX)?;
        w.str(c.passphrase, PASSPHRASE_MAX)?;
        w.bytes(&c.lat_deg.to_le_bytes())?;
        w.bytes(&c.lon_deg.to_le_bytes())?;
        w.bytes(&c.alt_m.to_le_bytes())?;
    }
    Ok(w.len)
}

pub fn decode_request(frame: &[u8]) -> Result<Frame<Request<'_>>, DecodeError> {
    let (version, ty, mut r) = header(frame)?;
    let message = match ty {
        ty::GET_INFO => Request::GetInfo,
        ty::SCAN => Request::Scan,
        ty::CONFIGURE => Request::Configure(Configure {
            ssid: r.str(SSID_MAX)?,
            passphrase: r.str(PASSPHRASE_MAX)?,
            lat_deg: r.f64()?,
            lon_deg: r.f64()?,
            alt_m: r.f64()?,
        }),
        t => return Err(DecodeError::UnknownType(t)),
    };
    Ok(Frame { version, message })
}

pub fn encode_response(r: &Response, buf: &mut [u8]) -> Result<usize, EncodeError> {
    let ty = match r {
        Response::Info(_) => ty::INFO,
        Response::Network(_) => ty::NETWORK,
        Response::ScanDone { .. } => ty::SCAN_DONE,
        Response::Testing => ty::TESTING,
        Response::ConfigureResult(_) => ty::CONFIGURE_RESULT,
        Response::Error { .. } => ty::ERROR,
    };
    let mut w = Writer::new(buf, ty)?;
    match r {
        Response::Info(i) => {
            w.bytes(&i.build.to_le_bytes())?;
            w.str(i.version, VERSION_MAX)?;
            w.u8(i.channel.byte())?;
            w.bytes(&i.mac)?;
            w.u8(i.flags)?;
            w.u8(i.reason.byte())?;
            w.str(i.ssid, SSID_MAX)?;
            w.bytes(&i.last_fetch_unix_s.to_le_bytes())?;
            w.bytes(&i.failed_build.to_le_bytes())?;
        }
        Response::Network(n) => {
            w.bytes(&n.rssi_dbm.to_le_bytes())?;
            w.u8(n.security.byte())?;
            w.str(n.ssid, SSID_MAX)?;
        }
        Response::ScanDone { count, ok } => {
            w.u8(*count)?;
            w.u8(u8::from(*ok))?;
        }
        Response::Testing => {}
        Response::ConfigureResult(c) => w.u8(c.byte())?,
        Response::Error { code, request_type } => {
            w.u8(code.byte())?;
            w.u8(*request_type)?;
        }
    }
    Ok(w.len)
}

pub fn decode_response(frame: &[u8]) -> Result<Frame<Response<'_>>, DecodeError> {
    let (version, ty, mut r) = header(frame)?;
    let message = match ty {
        ty::INFO => Response::Info(Info {
            build: u32::from_le_bytes(r.array()?),
            version: r.str(VERSION_MAX)?,
            channel: r.enum_byte(Channel::from_byte)?,
            mac: r.array()?,
            flags: r.u8()?,
            reason: r.enum_byte(SetupReason::from_byte)?,
            ssid: r.str(SSID_MAX)?,
            last_fetch_unix_s: i64::from_le_bytes(r.array()?),
            failed_build: u32::from_le_bytes(r.array()?),
        }),
        ty::NETWORK => Response::Network(Network {
            rssi_dbm: i8::from_le_bytes(r.array()?),
            security: r.enum_byte(Security::from_byte)?,
            ssid: r.str(SSID_MAX)?,
        }),
        ty::SCAN_DONE => Response::ScanDone {
            count: r.u8()?,
            ok: r.u8()? != 0,
        },
        ty::TESTING => Response::Testing,
        ty::CONFIGURE_RESULT => Response::ConfigureResult(r.enum_byte(ConfigureResult::from_byte)?),
        ty::ERROR => Response::Error {
            code: r.enum_byte(ErrorCode::from_byte)?,
            request_type: r.u8()?,
        },
        t => return Err(DecodeError::UnknownType(t)),
    };
    Ok(Frame { version, message })
}

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod tests;
