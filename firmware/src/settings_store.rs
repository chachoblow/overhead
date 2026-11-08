//! Per-device settings (0084) in the `settings` partition (0083): a small
//! CRC-checked record in two alternating one-sector slots, so a power cut
//! during a save leaves the previous record. Host-tested; the target supplies
//! an [`element_store::Flash`].
//!
//! Format 1 holds what the update path needs (0085/0086): the update channel,
//! the last build that failed its health check, and the build an install has
//! just activated but not yet confirmed. Wi-Fi, site and bonding fields arrive
//! with M7 item 2 as format 2, which must still read format 1.
//!
//! Record (little-endian, 32 bytes at the start of a slot's sector): magic
//! `OHST`, format u16, reserved u16, sequence u32, channel u8 (0 stable,
//! 1 beta), flags u8 (bit 0 failed build present, bit 1 pending build
//! present), reserved u16, failed build u32, pending build u32, reserved
//! 4 bytes, CRC-32 u32 over the first 28 bytes. The valid slot with the
//! higher sequence wins. A save erases the other slot's sector and programs
//! the record in one write: a cut leaves a bad CRC there and the old slot.
use crate::element_store::{Flash, SECTOR, crc32};
use overhead_update::Channel;

pub const RECORD_LEN: usize = 32;
const MAGIC: [u8; 4] = *b"OHST";
/// The format this firmware writes.
pub const FORMAT: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub channel: Channel,
    pub failed_build: Option<u32>,
    pub pending_build: Option<u32>,
}

impl Settings {
    /// An empty store: gifts follow `stable` (0085).
    pub const fn defaults(channel: Channel) -> Self {
        Self {
            channel,
            failed_build: None,
            pending_build: None,
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
    b[13] = u8::from(s.failed_build.is_some()) | (u8::from(s.pending_build.is_some()) << 1);
    b[16..20].copy_from_slice(&s.failed_build.unwrap_or(0).to_le_bytes());
    b[20..24].copy_from_slice(&s.pending_build.unwrap_or(0).to_le_bytes());
    let crc = crc32(&b[..28]);
    b[28..32].copy_from_slice(&crc.to_le_bytes());
    b
}

pub fn decode(b: &[u8; RECORD_LEN]) -> Result<(Settings, u32), SlotError> {
    if b.iter().all(|&x| x == 0xFF) {
        return Err(SlotError::Erased);
    }
    if b[0..4] != MAGIC {
        return Err(SlotError::Magic);
    }
    let format = u16::from_le_bytes([b[4], b[5]]);
    if format != FORMAT {
        return Err(SlotError::Format(format));
    }
    let word = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    if word(28) != crc32(&b[..28]) {
        return Err(SlotError::Crc);
    }
    let channel = match b[12] {
        0 => Channel::Stable,
        1 => Channel::Beta,
        _ => return Err(SlotError::Field),
    };
    if b[13] & !0b11 != 0 {
        return Err(SlotError::Field);
    }
    let failed_build = (b[13] & 1 != 0).then(|| word(16));
    let pending_build = (b[13] & 2 != 0).then(|| word(20));
    Ok((
        Settings {
            channel,
            failed_build,
            pending_build,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
            let n = if self.fail_at == Some(self.ops) { 100 } else { SECTOR as usize };
            self.bytes[o..o + n].fill(0xFF);
            if self.fail_at == Some(self.ops) { Err(()) } else { Ok(()) }
        }
        fn write(&mut self, offset: u32, data: &[u8]) -> Result<(), ()> {
            self.ops += 1;
            let o = offset as usize;
            let n = if self.fail_at == Some(self.ops) { data.len() / 2 } else { data.len() };
            for (d, s) in self.bytes[o..o + n].iter_mut().zip(data) {
                *d &= *s;
            }
            if self.fail_at == Some(self.ops) { Err(()) } else { Ok(()) }
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
        }
    }

    #[test]
    fn round_trip() {
        for s in [Settings::defaults(Channel::Stable), sample(3), sample(4)] {
            assert_eq!(decode(&encode(&s, 9)), Ok((s, 9)));
        }
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
        let r = again.load(&mut f);
        // Five saves: slot 0 has seq 5, slot 1 seq 4.
        assert_eq!(r.slots, [Ok(5), Ok(4)]);
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
            assert_eq!(again.load(&mut f).loaded, Some(sample(2)), "fail_at={fail_at}");
        }
    }

    #[test]
    fn corrupt_and_foreign_records() {
        let mut b = encode(&sample(1), 1);
        b[16] ^= 1;
        assert_eq!(decode(&b), Err(SlotError::Crc));
        let mut b = encode(&sample(1), 1);
        b[4] = 2;
        assert_eq!(decode(&b), Err(SlotError::Format(2)));
        let mut b = encode(&sample(1), 1);
        b[0] = b'X';
        assert_eq!(decode(&b), Err(SlotError::Magic));
        let mut b = encode(&sample(1), 1);
        b[12] = 7;
        let crc = crc32(&b[..28]);
        b[28..32].copy_from_slice(&crc.to_le_bytes());
        assert_eq!(decode(&b), Err(SlotError::Field));
    }

    #[test]
    fn bad_layout() {
        assert_eq!(Store::new(0x100, 0x10000), None);
        assert_eq!(Store::new(0x1000, SECTOR), None);
    }
}
