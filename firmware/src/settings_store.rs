//! Per-device settings (0084) in the `settings` partition (0083): a small
//! CRC-checked record in two alternating one-sector slots, so a power cut
//! during a save leaves the previous record. Host-tested; the target supplies
//! an [`element_store::Flash`].
//!
//! Fields: update channel, last build that failed its health check, the
//! build an install has activated but not yet confirmed (0085/0086), one
//! Wi-Fi network and the observer site. Images are built without personal
//! data; the developer-only `dev-settings` feature seeds an empty store (or
//! missing fields) from `.wifi.env`/`.site.env`. BLE bonding keys arrive with
//! the setup app (0087) as a later format.
//!
//! Every format is read; the newest is written. Record (little-endian, at the
//! start of a slot's sector): magic `OHST`, format u16, reserved u16,
//! sequence u32, channel u8 (0 stable, 1 beta), flags u8, reserved u16,
//! failed build u32, pending build u32, then
//! - format 1 (32 bytes): reserved 4 bytes, CRC-32 over bytes 0..28;
//! - format 2 (192 bytes): SSID length u8, passphrase length u8, reserved
//!   u16, SSID 32 bytes, passphrase 64 bytes, latitude f64 (deg), longitude
//!   f64 (deg), altitude f64 (m), reserved 40 bytes, CRC-32 over 0..188.
//!
//! Flags: bit 0 failed build, bit 1 pending build, bit 2 Wi-Fi, bit 3 site.
//! The valid slot with the higher sequence wins. A save erases the other
//! slot's sector and programs the record in one write: a cut leaves a bad CRC
//! there and the old slot in force.
use crate::element_store::{Flash, SECTOR, crc32};
use overhead_update::Channel;

const MAGIC: [u8; 4] = *b"OHST";
/// The format this firmware writes.
pub const FORMAT: u16 = 2;
pub const RECORD_LEN: usize = 192;
const V1_LEN: usize = 32;
pub const SSID_MAX: usize = 32;
pub const PASS_MAX: usize = 64;

const FAILED: u8 = 1;
const PENDING: u8 = 2;
const WIFI: u8 = 4;
const SITE: u8 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wifi {
    ssid: [u8; SSID_MAX],
    ssid_len: u8,
    pass: [u8; PASS_MAX],
    pass_len: u8,
}

impl Wifi {
    /// SSID 1..=32 bytes; passphrase 8..=63 characters or 64 hex digits
    /// (WPA2-Personal).
    pub fn new(ssid: &str, pass: &str) -> Option<Self> {
        let (s, p) = (ssid.as_bytes(), pass.as_bytes());
        let pass_ok =
            (8..=63).contains(&p.len()) || (p.len() == 64 && p.iter().all(u8::is_ascii_hexdigit));
        if s.is_empty() || s.len() > SSID_MAX || !pass_ok {
            return None;
        }
        let mut w = Self {
            ssid: [0; SSID_MAX],
            ssid_len: s.len() as u8,
            pass: [0; PASS_MAX],
            pass_len: p.len() as u8,
        };
        w.ssid[..s.len()].copy_from_slice(s);
        w.pass[..p.len()].copy_from_slice(p);
        Some(w)
    }

    pub fn ssid(&self) -> &str {
        core::str::from_utf8(&self.ssid[..usize::from(self.ssid_len)]).unwrap_or("")
    }

    pub fn passphrase(&self) -> &str {
        core::str::from_utf8(&self.pass[..usize::from(self.pass_len)]).unwrap_or("")
    }
}

/// Observer site in degrees and metres (checked like `site::parse`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Site {
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub alt_m: f64,
}

impl Site {
    pub fn valid(&self) -> bool {
        self.lat_deg.is_finite()
            && (-90.0..=90.0).contains(&self.lat_deg)
            && self.lon_deg.is_finite()
            && (-180.0..=180.0).contains(&self.lon_deg)
            && self.alt_m.is_finite()
            && (-500.0..=9_000.0).contains(&self.alt_m)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub channel: Channel,
    pub failed_build: Option<u32>,
    pub pending_build: Option<u32>,
    pub wifi: Option<Wifi>,
    pub site: Option<Site>,
}

impl Settings {
    /// An empty store: gifts follow `stable` (0085).
    pub const fn defaults(channel: Channel) -> Self {
        Self {
            channel,
            failed_build: None,
            pending_build: None,
            wifi: None,
            site: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotError {
    Erased,
    Magic,
    Format(u16),
    Crc,
    Field,
    Read,
}

pub fn encode(s: &Settings, sequence: u32) -> [u8; RECORD_LEN] {
    let mut b = [0u8; RECORD_LEN];
    b[0..4].copy_from_slice(&MAGIC);
    b[4..6].copy_from_slice(&FORMAT.to_le_bytes());
    b[8..12].copy_from_slice(&sequence.to_le_bytes());
    b[12] = match s.channel {
        Channel::Stable => 0,
        Channel::Beta => 1,
    };
    let mut flags = 0;
    if let Some(f) = s.failed_build {
        flags |= FAILED;
        b[16..20].copy_from_slice(&f.to_le_bytes());
    }
    if let Some(p) = s.pending_build {
        flags |= PENDING;
        b[20..24].copy_from_slice(&p.to_le_bytes());
    }
    if let Some(w) = s.wifi {
        flags |= WIFI;
        b[24] = w.ssid_len;
        b[25] = w.pass_len;
        b[28..60].copy_from_slice(&w.ssid);
        b[60..124].copy_from_slice(&w.pass);
    }
    if let Some(site) = s.site {
        flags |= SITE;
        b[124..132].copy_from_slice(&site.lat_deg.to_le_bytes());
        b[132..140].copy_from_slice(&site.lon_deg.to_le_bytes());
        b[140..148].copy_from_slice(&site.alt_m.to_le_bytes());
    }
    b[13] = flags;
    let crc = crc32(&b[..RECORD_LEN - 4]);
    b[RECORD_LEN - 4..].copy_from_slice(&crc.to_le_bytes());
    b
}

pub fn decode(b: &[u8; RECORD_LEN]) -> Result<(Settings, u32), SlotError> {
    if b[..V1_LEN].iter().all(|&x| x == 0xFF) {
        return Err(SlotError::Erased);
    }
    if b[0..4] != MAGIC {
        return Err(SlotError::Magic);
    }
    let format = u16::from_le_bytes([b[4], b[5]]);
    let (len, known) = match format {
        1 => (V1_LEN, FAILED | PENDING),
        2 => (RECORD_LEN, FAILED | PENDING | WIFI | SITE),
        f => return Err(SlotError::Format(f)),
    };
    let word = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    if word(len - 4) != crc32(&b[..len - 4]) {
        return Err(SlotError::Crc);
    }
    let channel = match b[12] {
        0 => Channel::Stable,
        1 => Channel::Beta,
        _ => return Err(SlotError::Field),
    };
    let flags = b[13];
    if flags & !known != 0 {
        return Err(SlotError::Field);
    }
    let f64_at = |i: usize| {
        let mut x = [0u8; 8];
        x.copy_from_slice(&b[i..i + 8]);
        f64::from_le_bytes(x)
    };
    let wifi = if flags & WIFI != 0 {
        let (sl, pl) = (usize::from(b[24]), usize::from(b[25]));
        if sl > SSID_MAX || pl > PASS_MAX {
            return Err(SlotError::Field);
        }
        let ssid = core::str::from_utf8(&b[28..28 + sl]).map_err(|_| SlotError::Field)?;
        let pass = core::str::from_utf8(&b[60..60 + pl]).map_err(|_| SlotError::Field)?;
        Some(Wifi::new(ssid, pass).ok_or(SlotError::Field)?)
    } else {
        None
    };
    let site = if flags & SITE != 0 {
        let site = Site {
            lat_deg: f64_at(124),
            lon_deg: f64_at(132),
            alt_m: f64_at(140),
        };
        if !site.valid() {
            return Err(SlotError::Field);
        }
        Some(site)
    } else {
        None
    };
    Ok((
        Settings {
            channel,
            failed_build: (flags & FAILED != 0).then(|| word(16)),
            pending_build: (flags & PENDING != 0).then(|| word(20)),
            wifi,
            site,
        },
        word(8),
    ))
}

/// The open partition: two slots one sector each, at `base` and `base+SECTOR`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Store {
    base: u32,
    /// Slot and sequence of the record in use, if any.
    current: Option<(usize, u32)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Report {
    pub slots: [Result<u32, SlotError>; 2],
    /// `None` when neither slot holds a valid record.
    pub loaded: Option<Settings>,
}

impl Store {
    /// Needs a sector-aligned partition of at least two sectors.
    pub fn new(offset: u32, len: u32) -> Option<Self> {
        (offset.is_multiple_of(SECTOR) && len >= 2 * SECTOR).then_some(Self {
            base: offset,
            current: None,
        })
    }

    fn slot_offset(&self, slot: usize) -> u32 {
        self.base + slot as u32 * SECTOR
    }

    pub fn load<F: Flash>(&mut self, flash: &mut F) -> Report {
        let mut slots = [Err(SlotError::Read), Err(SlotError::Read)];
        let mut best: Option<(usize, u32, Settings)> = None;
        for (slot, result) in slots.iter_mut().enumerate() {
            let mut b = [0u8; RECORD_LEN];
            if flash.read(self.slot_offset(slot), &mut b).is_err() {
                continue;
            }
            match decode(&b) {
                Ok((s, seq)) => {
                    *result = Ok(seq);
                    if best.is_none_or(|(_, best_seq, _)| seq > best_seq) {
                        best = Some((slot, seq, s));
                    }
                }
                Err(e) => *result = Err(e),
            }
        }
        self.current = best.map(|(slot, seq, _)| (slot, seq));
        Report {
            slots,
            loaded: best.map(|(_, _, s)| s),
        }
    }

    /// Writes `s` to the slot not in use; on success it becomes current.
    pub fn save<F: Flash>(&mut self, flash: &mut F, s: &Settings) -> Result<(), F::Error> {
        let (slot, seq) = match self.current {
            Some((slot, seq)) => (1 - slot, seq.wrapping_add(1)),
            None => (0, 1),
        };
        let at = self.slot_offset(slot);
        flash.erase_sector(at)?;
        flash.write(at, &encode(s, seq))?;
        self.current = Some((slot, seq));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    /// NOR mock: erase sets 0xFF; programming only clears bits. `fail_at`
    /// cuts the n-th operation (counting erases and writes) after it changed
    /// part of the target, like a power cut.
    struct Nor {
        bytes: Vec<u8>,
        ops: usize,
        fail_at: Option<usize>,
    }

    impl Flash for Nor {
        type Error = ();
        fn read(&mut self, offset: u32, buf: &mut [u8]) -> Result<(), ()> {
            let o = offset as usize;
            buf.copy_from_slice(&self.bytes[o..o + buf.len()]);
            Ok(())
        }
        fn erase_sector(&mut self, offset: u32) -> Result<(), ()> {
            self.ops += 1;
            let o = offset as usize;
            let n = if self.fail_at == Some(self.ops) {
                100
            } else {
                SECTOR as usize
            };
            self.bytes[o..o + n].fill(0xFF);
            if self.fail_at == Some(self.ops) {
                Err(())
            } else {
                Ok(())
            }
        }
        fn write(&mut self, offset: u32, data: &[u8]) -> Result<(), ()> {
            self.ops += 1;
            let o = offset as usize;
            let n = if self.fail_at == Some(self.ops) {
                data.len() / 2
            } else {
                data.len()
            };
            for (d, s) in self.bytes[o..o + n].iter_mut().zip(data) {
                *d &= *s;
            }
            if self.fail_at == Some(self.ops) {
                Err(())
            } else {
                Ok(())
            }
        }
    }

    const BASE: u32 = 0x2000;

    fn nor() -> Nor {
        Nor {
            bytes: vec![0xFF; 0x10000],
            ops: 0,
            fail_at: None,
        }
    }

    fn sample(n: u32) -> Settings {
        Settings {
            channel: Channel::Beta,
            failed_build: Some(n),
            pending_build: (n % 2 == 0).then_some(n + 1),
            wifi: Wifi::new("home net", "correct horse"),
            site: (n % 2 == 1).then_some(Site {
                lat_deg: 10.5,
                lon_deg: -20.25,
                alt_m: 100.0,
            }),
        }
    }

    #[test]
    fn round_trip() {
        for s in [Settings::defaults(Channel::Stable), sample(3), sample(4)] {
            assert_eq!(decode(&encode(&s, 9)), Ok((s, 9)));
        }
    }

    #[test]
    fn reads_format_1() {
        // A format-1 record as the 2026-11-08 board wrote it.
        let mut b = [0xFFu8; RECORD_LEN];
        let mut v1 = [0u8; V1_LEN];
        v1[0..4].copy_from_slice(b"OHST");
        v1[4..6].copy_from_slice(&1u16.to_le_bytes());
        v1[8..12].copy_from_slice(&7u32.to_le_bytes());
        v1[12] = 1;
        v1[13] = FAILED;
        v1[16..20].copy_from_slice(&5u32.to_le_bytes());
        let crc = crc32(&v1[..28]);
        v1[28..32].copy_from_slice(&crc.to_le_bytes());
        b[..V1_LEN].copy_from_slice(&v1);
        let (s, seq) = decode(&b).unwrap();
        assert_eq!(seq, 7);
        assert_eq!(
            s,
            Settings {
                failed_build: Some(5),
                ..Settings::defaults(Channel::Beta)
            }
        );
    }

    #[test]
    fn wifi_limits() {
        assert!(Wifi::new("", "password").is_none());
        assert!(Wifi::new("x", "short").is_none());
        assert!(Wifi::new(&"s".repeat(33), "password").is_none());
        assert!(Wifi::new("x", &"p".repeat(64)).is_none());
        assert!(Wifi::new("x", &"a".repeat(64)).is_some());
        let w = Wifi::new(&"s".repeat(32), &"p".repeat(63)).unwrap();
        assert_eq!((w.ssid().len(), w.passphrase().len()), (32, 63));
    }

    #[test]
    fn empty_then_saves_alternate() {
        let mut f = nor();
        let mut st = Store::new(BASE, 0x10000 - BASE).unwrap();
        let r = st.load(&mut f);
        assert_eq!(r.loaded, None);
        assert_eq!(r.slots, [Err(SlotError::Erased), Err(SlotError::Erased)]);
        for n in 1..=5 {
            st.save(&mut f, &sample(n)).unwrap();
            let mut again = Store::new(BASE, 0x10000 - BASE).unwrap();
            assert_eq!(again.load(&mut f).loaded, Some(sample(n)));
        }
        let mut again = Store::new(BASE, 0x10000 - BASE).unwrap();
        assert_eq!(again.load(&mut f).slots, [Ok(5), Ok(4)]);
    }

    #[test]
    fn a_cut_save_keeps_the_previous_record() {
        for fail_at in 1..=2 {
            let mut f = nor();
            let mut st = Store::new(BASE, 0x4000).unwrap();
            st.load(&mut f);
            st.save(&mut f, &sample(1)).unwrap();
            st.save(&mut f, &sample(2)).unwrap();
            f.ops = 0;
            f.fail_at = Some(fail_at);
            assert!(st.save(&mut f, &sample(3)).is_err());
            let mut again = Store::new(BASE, 0x4000).unwrap();
            assert_eq!(
                again.load(&mut f).loaded,
                Some(sample(2)),
                "fail_at={fail_at}"
            );
        }
    }

    #[test]
    fn corrupt_and_foreign_records() {
        let fix_crc = |b: &mut [u8; RECORD_LEN]| {
            let crc = crc32(&b[..RECORD_LEN - 4]);
            b[RECORD_LEN - 4..].copy_from_slice(&crc.to_le_bytes());
        };
        let mut b = encode(&sample(1), 1);
        b[16] ^= 1;
        assert_eq!(decode(&b), Err(SlotError::Crc));
        let mut b = encode(&sample(1), 1);
        b[4] = 3;
        assert_eq!(decode(&b), Err(SlotError::Format(3)));
        let mut b = encode(&sample(1), 1);
        b[0] = b'X';
        assert_eq!(decode(&b), Err(SlotError::Magic));
        let mut b = encode(&sample(1), 1);
        b[12] = 7;
        fix_crc(&mut b);
        assert_eq!(decode(&b), Err(SlotError::Field));
        let mut b = encode(&sample(1), 1);
        b[124..132].copy_from_slice(&95.0f64.to_le_bytes());
        fix_crc(&mut b);
        assert_eq!(decode(&b), Err(SlotError::Field));
        let mut b = encode(&sample(1), 1);
        b[25] = 3; // passphrase too short
        fix_crc(&mut b);
        assert_eq!(decode(&b), Err(SlotError::Field));
    }

    #[test]
    fn bad_layout() {
        assert_eq!(Store::new(0x100, 0x10000), None);
        assert_eq!(Store::new(0x1000, SECTOR), None);
    }
}
