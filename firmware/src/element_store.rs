//! Fetched-element persistence (0079): the last successfully fetched response
//! body and its trusted fetch UTC, kept in a two-slot flash partition so a
//! reset can resume tracking without a CelesTrak request. Host-tested and
//! hardware-agnostic: the target supplies a [`Flash`] (esp-storage, through
//! an internal-RAM buffer); the tests use a NOR-semantics mock.
//!
//! Slot layout: sector 0 holds a 32-byte header, the body starts at sector 1.
//! A save erases the header sector first (so the slot is invalid at once),
//! then the body sectors, programs the body and writes the header last; the
//! header carries the body's length and CRC-32, so a write cut short at any
//! point leaves that slot invalid and the other slot (the previous copy)
//! loadable. Each [`Writer::step`] is one sector erase or one sector program,
//! so the caller can let the display run between steps.
//!
//! Header (little-endian): magic `OHES`, version u16, reserved u16, sequence
//! u32, fetch UTC as Unix seconds i64, body length u32, body CRC-32 u32,
//! header CRC-32 u32 over the first 28 bytes. The newest valid slot (highest
//! sequence whose header and body both check) is loaded.

pub const SECTOR: u32 = 4096;
pub const HEADER_LEN: usize = 32;
const MAGIC: [u8; 4] = *b"OHES";
/// Bump when the header or body meaning changes; other versions are ignored.
pub const VERSION: u16 = 1;

/// Word-aligned NOR flash access at absolute offsets.
pub trait Flash {
    type Error: core::fmt::Debug;
    /// Reads `buf.len()` bytes at `offset`; both multiples of 4.
    fn read(&mut self, offset: u32, buf: &mut [u8]) -> Result<(), Self::Error>;
    /// Erases the [`SECTOR`] at `offset` (sector-aligned) to `0xFF`.
    fn erase_sector(&mut self, offset: u32) -> Result<(), Self::Error>;
    /// Programs `data` (at most one [`SECTOR`]) at `offset`; both multiples
    /// of 4, the range already erased.
    fn write(&mut self, offset: u32, data: &[u8]) -> Result<(), Self::Error>;
}

/// CRC-32 (IEEE 802.3, reflected), bitwise: no table in flash or RAM.
#[derive(Clone, Copy, Debug)]
pub struct Crc32(u32);

impl Crc32 {
    pub const fn new() -> Self {
        Self(0xFFFF_FFFF)
    }

    pub fn update(&mut self, bytes: &[u8]) {
        let mut crc = self.0;
        for &b in bytes {
            crc ^= u32::from(b);
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
            }
        }
        self.0 = crc;
    }

    pub const fn finish(self) -> u32 {
        !self.0
    }
}

impl Default for Crc32 {
    fn default() -> Self {
        Self::new()
    }
}

pub fn crc32(bytes: &[u8]) -> u32 {
    let mut c = Crc32::new();
    c.update(bytes);
    c.finish()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutError {
    /// Partition offset or length is not sector-aligned.
    Unaligned,
    /// Too small for two slots of a header sector plus one body sector.
    TooSmall,
}

/// Two equal slots in one partition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    base: u32,
    slot_len: u32,
}

impl Layout {
    pub fn new(offset: u32, len: u32) -> Result<Self, LayoutError> {
        if !offset.is_multiple_of(SECTOR) || !len.is_multiple_of(SECTOR) {
            return Err(LayoutError::Unaligned);
        }
        let slot_len = (len / 2) / SECTOR * SECTOR;
        if slot_len < 2 * SECTOR {
            return Err(LayoutError::TooSmall);
        }
        Ok(Self {
            base: offset,
            slot_len,
        })
    }

    pub const fn slot_offset(&self, slot: usize) -> u32 {
        self.base + slot as u32 * self.slot_len
    }

    /// Largest body a slot holds.
    pub const fn capacity(&self) -> usize {
        (self.slot_len - SECTOR) as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub sequence: u32,
    pub fetched_unix_s: i64,
    pub body_len: u32,
    pub body_crc: u32,
}

/// Why a slot was not loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotError {
    /// Header bytes all `0xFF`: never written, or a save erased it.
    Erased,
    Magic,
    Version(u16),
    HeaderCrc,
    /// Body length zero or over the slot capacity.
    Length(u32),
    BodyCrc,
    /// The flash read failed.
    Read,
}

impl Header {
    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let mut b = [0u8; HEADER_LEN];
        b[0..4].copy_from_slice(&MAGIC);
        b[4..6].copy_from_slice(&VERSION.to_le_bytes());
        b[8..12].copy_from_slice(&self.sequence.to_le_bytes());
        b[12..20].copy_from_slice(&self.fetched_unix_s.to_le_bytes());
        b[20..24].copy_from_slice(&self.body_len.to_le_bytes());
        b[24..28].copy_from_slice(&self.body_crc.to_le_bytes());
        let crc = crc32(&b[..28]);
        b[28..32].copy_from_slice(&crc.to_le_bytes());
        b
    }

    pub fn decode(b: &[u8; HEADER_LEN], capacity: usize) -> Result<Self, SlotError> {
        if b.iter().all(|&x| x == 0xFF) {
            return Err(SlotError::Erased);
        }
        if b[0..4] != MAGIC {
            return Err(SlotError::Magic);
        }
        let version = u16::from_le_bytes([b[4], b[5]]);
        if version != VERSION {
            return Err(SlotError::Version(version));
        }
        let word = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
        if word(28) != crc32(&b[..28]) {
            return Err(SlotError::HeaderCrc);
        }
        let body_len = word(20);
        if body_len == 0 || body_len as usize > capacity {
            return Err(SlotError::Length(body_len));
        }
        let mut fetched = [0u8; 8];
        fetched.copy_from_slice(&b[12..20]);
        Ok(Self {
            sequence: word(8),
            fetched_unix_s: i64::from_le_bytes(fetched),
            body_len,
            body_crc: word(24),
        })
    }
}

/// The copy that was loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Loaded {
    pub slot: usize,
    pub header: Header,
}

/// What a boot-time scan found in each slot and which one it loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Report {
    pub slots: [Result<Header, SlotError>; 2],
    pub loaded: Option<Loaded>,
    /// Highest sequence of any decodable header, body checked or not.
    pub newest_sequence: u32,
}

impl Report {
    /// Where the next save goes: never over the loaded copy, and with a
    /// sequence above every header seen, valid body or not.
    pub fn next_target(&self) -> Target {
        let slot = match self.loaded {
            Some(l) => 1 - l.slot,
            None => 0,
        };
        Target {
            slot,
            sequence: self.newest_sequence.wrapping_add(1),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    pub slot: usize,
    pub sequence: u32,
}

impl Target {
    /// After a completed save to this target, the following one.
    pub const fn after(self) -> Self {
        Self {
            slot: 1 - self.slot,
            sequence: self.sequence.wrapping_add(1),
        }
    }
}

const fn round4(n: usize) -> usize {
    (n + 3) & !3
}

/// Reads both headers, then tries the valid ones newest first; the first
/// body whose CRC matches is left in `buf[..body_len]`. A body longer than
/// `buf` (or the slot) counts as a [`SlotError::Length`].
pub fn load<F: Flash>(flash: &mut F, layout: &Layout, buf: &mut [u8]) -> Report {
    let capacity = layout.capacity().min(buf.len() & !3);
    let mut slots = [Err(SlotError::Read); 2];
    for (i, slot) in slots.iter_mut().enumerate() {
        let mut h = [0u8; HEADER_LEN];
        if flash.read(layout.slot_offset(i), &mut h).is_ok() {
            *slot = Header::decode(&h, capacity);
        }
    }
    let newest_sequence = slots
        .iter()
        .filter_map(|s| s.as_ref().ok())
        .map(|h| h.sequence)
        .max()
        .unwrap_or(0);
    let mut order = [0usize, 1];
    if let (Ok(a), Ok(b)) = (&slots[0], &slots[1])
        && b.sequence > a.sequence
    {
        order = [1, 0];
    }
    for i in order {
        let Ok(header) = slots[i] else { continue };
        match read_body(flash, layout, i, &header, buf) {
            Ok(()) => {
                return Report {
                    slots,
                    loaded: Some(Loaded { slot: i, header }),
                    newest_sequence,
                };
            }
            Err(e) => slots[i] = Err(e),
        }
    }
    Report {
        slots,
        loaded: None,
        newest_sequence,
    }
}

fn read_body<F: Flash>(
    flash: &mut F,
    layout: &Layout,
    slot: usize,
    header: &Header,
    buf: &mut [u8],
) -> Result<(), SlotError> {
    let len = header.body_len as usize;
    let start = layout.slot_offset(slot) + SECTOR;
    let mut crc = Crc32::new();
    let mut done = 0;
    while done < len {
        let n = (len - done).min(SECTOR as usize);
        let chunk = &mut buf[done..done + round4(n)];
        flash
            .read(start + done as u32, chunk)
            .map_err(|_| SlotError::Read)?;
        crc.update(&chunk[..n]);
        done += n;
    }
    if crc.finish() == header.body_crc {
        Ok(())
    } else {
        Err(SlotError::BodyCrc)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteError {
    /// Empty body or larger than a slot.
    Length(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    EraseHeader,
    EraseBody(u32),
    WriteBody(u32),
    WriteHeader,
    Done,
}

/// What the last [`Writer::step`] did, for timing it by kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Did {
    Erase,
    Write,
}

/// One save, a sector at a time. The body must stay unchanged until done.
#[derive(Clone, Copy, Debug)]
pub struct Writer {
    base: u32,
    header: Header,
    sectors: u32,
    step: Step,
}

impl Writer {
    pub fn new(
        layout: &Layout,
        target: Target,
        body: &[u8],
        fetched_unix_s: i64,
    ) -> Result<Self, WriteError> {
        if body.is_empty() || body.len() > layout.capacity() {
            return Err(WriteError::Length(body.len()));
        }
        Ok(Self {
            base: layout.slot_offset(target.slot),
            header: Header {
                sequence: target.sequence,
                fetched_unix_s,
                body_len: body.len() as u32,
                body_crc: crc32(body),
            },
            sectors: body.len().div_ceil(SECTOR as usize) as u32,
            step: Step::EraseHeader,
        })
    }

    pub const fn header(&self) -> Header {
        self.header
    }

    pub fn done(&self) -> bool {
        self.step == Step::Done
    }

    /// Total steps a save takes (header erase, body erases and programs,
    /// header program).
    pub const fn steps(&self) -> u32 {
        2 + 2 * self.sectors
    }

    /// Performs the next step. A failed step can be retried or abandoned;
    /// either way the slot stays invalid until the header is written.
    pub fn step<F: Flash>(&mut self, flash: &mut F, body: &[u8]) -> Result<Did, F::Error> {
        let body_at = |i: u32| self.base + SECTOR * (1 + i);
        let (did, next) = match self.step {
            Step::EraseHeader => {
                flash.erase_sector(self.base)?;
                (Did::Erase, Step::EraseBody(0))
            }
            Step::EraseBody(i) => {
                flash.erase_sector(body_at(i))?;
                let next = if i + 1 < self.sectors {
                    Step::EraseBody(i + 1)
                } else {
                    Step::WriteBody(0)
                };
                (Did::Erase, next)
            }
            Step::WriteBody(i) => {
                let from = (i * SECTOR) as usize;
                let chunk = &body[from..body.len().min(from + SECTOR as usize)];
                let whole = chunk.len() & !3;
                if whole > 0 {
                    flash.write(body_at(i), &chunk[..whole])?;
                }
                if whole < chunk.len() {
                    // Pad the last partial word; erased flash is 0xFF anyway.
                    let mut word = [0xFFu8; 4];
                    word[..chunk.len() - whole].copy_from_slice(&chunk[whole..]);
                    flash.write(body_at(i) + whole as u32, &word)?;
                }
                let next = if i + 1 < self.sectors {
                    Step::WriteBody(i + 1)
                } else {
                    Step::WriteHeader
                };
                (Did::Write, next)
            }
            Step::WriteHeader => {
                flash.write(self.base, &self.header.encode())?;
                (Did::Write, Step::Done)
            }
            Step::Done => return Ok(Did::Write),
        };
        self.step = next;
        Ok(did)
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::{vec, vec::Vec};

    const PART: u32 = 0x10_000;
    const PART_LEN: u32 = 0x50_000;

    /// NOR semantics: erase sets a sector to 0xFF, a program can only clear
    /// bits. Offsets are absolute; the mock covers the partition only.
    struct Nor {
        mem: Vec<u8>,
        fail_after: Option<usize>,
        ops: usize,
    }

    #[derive(Debug)]
    struct Fault;

    impl Nor {
        fn new() -> Self {
            Self {
                mem: vec![0xFF; PART_LEN as usize],
                fail_after: None,
                ops: 0,
            }
        }
        fn at(&self, offset: u32) -> usize {
            (offset - PART) as usize
        }
        fn tick(&mut self) -> Result<(), Fault> {
            self.ops += 1;
            match self.fail_after {
                Some(n) if self.ops > n => Err(Fault),
                _ => Ok(()),
            }
        }
    }

    impl Flash for Nor {
        type Error = Fault;
        fn read(&mut self, offset: u32, buf: &mut [u8]) -> Result<(), Fault> {
            assert!(offset.is_multiple_of(4) && buf.len().is_multiple_of(4));
            let a = self.at(offset);
            buf.copy_from_slice(&self.mem[a..a + buf.len()]);
            Ok(())
        }
        fn erase_sector(&mut self, offset: u32) -> Result<(), Fault> {
            assert!(offset.is_multiple_of(SECTOR));
            self.tick()?;
            let a = self.at(offset);
            self.mem[a..a + SECTOR as usize].fill(0xFF);
            Ok(())
        }
        fn write(&mut self, offset: u32, data: &[u8]) -> Result<(), Fault> {
            assert!(offset.is_multiple_of(4) && data.len().is_multiple_of(4));
            assert!(data.len() <= SECTOR as usize);
            self.tick()?;
            let a = self.at(offset);
            for (m, d) in self.mem[a..a + data.len()].iter_mut().zip(data) {
                *m &= d;
            }
            Ok(())
        }
    }

    fn layout() -> Layout {
        Layout::new(PART, PART_LEN).unwrap()
    }

    fn body(len: usize, seed: u8) -> Vec<u8> {
        (0..len)
            .map(|i| (i as u8).wrapping_mul(31) ^ seed)
            .collect()
    }

    fn buf() -> Vec<u8> {
        vec![0; round4(layout().capacity())]
    }

    fn save(flash: &mut Nor, target: Target, body: &[u8], at: i64) {
        let mut w = Writer::new(&layout(), target, body, at).unwrap();
        let mut steps = 0;
        while !w.done() {
            w.step(flash, body).unwrap();
            steps += 1;
        }
        assert_eq!(steps, w.steps());
    }

    #[test]
    fn crc32_matches_the_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        let mut c = Crc32::new();
        c.update(b"1234");
        c.update(b"56789");
        assert_eq!(c.finish(), 0xCBF4_3926);
    }

    #[test]
    fn layout_splits_the_partition_into_sector_aligned_slots() {
        let l = layout();
        assert_eq!(l.slot_offset(0), PART);
        assert_eq!(l.slot_offset(1), PART + 0x28_000);
        assert_eq!(l.capacity(), 0x27_000);
        assert_eq!(Layout::new(PART + 1, PART_LEN), Err(LayoutError::Unaligned));
        assert_eq!(Layout::new(PART, 3 * SECTOR), Err(LayoutError::TooSmall));
        assert!(Layout::new(PART, 4 * SECTOR).is_ok());
    }

    #[test]
    fn header_round_trips_and_rejects_damage() {
        let h = Header {
            sequence: 7,
            fetched_unix_s: 1_793_000_000,
            body_len: 1234,
            body_crc: 0xDEAD_BEEF,
        };
        let cap = layout().capacity();
        let bytes = h.encode();
        assert_eq!(Header::decode(&bytes, cap), Ok(h));
        assert_eq!(
            Header::decode(&[0xFF; HEADER_LEN], cap),
            Err(SlotError::Erased)
        );
        let mut b = bytes;
        b[0] = b'X';
        assert_eq!(Header::decode(&b, cap), Err(SlotError::Magic));
        let mut b = bytes;
        b[4] = 2;
        assert_eq!(Header::decode(&b, cap), Err(SlotError::Version(2)));
        let mut b = bytes;
        b[13] ^= 1;
        assert_eq!(Header::decode(&b, cap), Err(SlotError::HeaderCrc));
        for len in [0, cap as u32 + 1] {
            let b = Header { body_len: len, ..h }.encode();
            assert_eq!(Header::decode(&b, cap), Err(SlotError::Length(len)));
        }
    }

    #[test]
    fn erased_partition_loads_nothing_and_targets_slot_zero() {
        let mut f = Nor::new();
        let r = load(&mut f, &layout(), &mut buf());
        assert_eq!(r.loaded, None);
        assert_eq!(r.slots, [Err(SlotError::Erased); 2]);
        assert_eq!(
            r.next_target(),
            Target {
                slot: 0,
                sequence: 1
            }
        );
    }

    #[test]
    fn saves_alternate_slots_and_the_newest_loads() {
        let mut f = Nor::new();
        let mut target = load(&mut f, &layout(), &mut buf()).next_target();
        // Unaligned, multi-sector, exactly-one-sector and full-capacity bodies.
        let cap = layout().capacity();
        for (n, len) in [5usize, 9_001, 4096, cap].into_iter().enumerate() {
            let b = body(len, n as u8);
            save(&mut f, target, &b, 1000 + n as i64);
            let mut out = buf();
            let r = load(&mut f, &layout(), &mut out);
            let l = r.loaded.unwrap();
            assert_eq!(l.slot, target.slot);
            assert_eq!(l.header.sequence, target.sequence);
            assert_eq!(l.header.fetched_unix_s, 1000 + n as i64);
            assert_eq!(&out[..len], &b[..]);
            assert_eq!(r.next_target(), target.after());
            target = target.after();
        }
    }

    #[test]
    fn a_body_longer_than_the_load_buffer_is_not_read() {
        let mut f = Nor::new();
        save(
            &mut f,
            Target {
                slot: 0,
                sequence: 1,
            },
            &body(9_000, 11),
            5,
        );
        let mut small = vec![0; 8_999];
        let r = load(&mut f, &layout(), &mut small);
        assert_eq!(r.slots[0], Err(SlotError::Length(9_000)));
        assert_eq!(r.loaded, None);
        let mut exact = vec![0; 9_000];
        assert!(load(&mut f, &layout(), &mut exact).loaded.is_some());
    }

    #[test]
    fn oversized_or_empty_bodies_are_refused() {
        let l = layout();
        let t = Target {
            slot: 0,
            sequence: 1,
        };
        assert_eq!(
            Writer::new(&l, t, &[], 0).err(),
            Some(WriteError::Length(0))
        );
        let big = vec![0; l.capacity() + 1];
        assert_eq!(
            Writer::new(&l, t, &big, 0).err(),
            Some(WriteError::Length(big.len()))
        );
    }

    #[test]
    fn an_interrupted_save_leaves_the_previous_copy_loadable() {
        let old = body(10_000, 1);
        let new = body(9_000, 2);
        let mut base = Nor::new();
        let first = Target {
            slot: 0,
            sequence: 1,
        };
        save(&mut base, first, &old, 100);
        let steps = Writer::new(&layout(), first.after(), &new, 200)
            .unwrap()
            .steps();
        for cut in 0..steps {
            let mut f = Nor {
                mem: base.mem.clone(),
                fail_after: None,
                ops: 0,
            };
            let mut w = Writer::new(&layout(), first.after(), &new, 200).unwrap();
            for _ in 0..cut {
                w.step(&mut f, &new).unwrap();
            }
            let mut out = buf();
            let r = load(&mut f, &layout(), &mut out);
            let l = r.loaded.expect("previous copy");
            assert_eq!((l.slot, l.header.fetched_unix_s), (0, 100), "cut {cut}");
            assert_eq!(&out[..old.len()], &old[..]);
            // The retry goes to the same, still-invalid slot.
            assert_eq!(r.next_target(), first.after(), "cut {cut}");
        }
    }

    #[test]
    fn a_failed_flash_operation_stops_the_save_without_damage() {
        let old = body(6_000, 3);
        let new = body(6_000, 4);
        let mut f = Nor::new();
        let first = Target {
            slot: 0,
            sequence: 1,
        };
        save(&mut f, first, &old, 1);
        f.ops = 0;
        f.fail_after = Some(3);
        let mut w = Writer::new(&layout(), first.after(), &new, 2).unwrap();
        let mut result = Ok(Did::Erase);
        while !w.done() && result.is_ok() {
            result = w.step(&mut f, &new);
        }
        assert!(result.is_err());
        let r = load(&mut f, &layout(), &mut buf());
        assert_eq!(r.loaded.unwrap().header.fetched_unix_s, 1);
    }

    #[test]
    fn a_corrupt_newest_body_falls_back_to_the_older_slot() {
        let mut f = Nor::new();
        let first = Target {
            slot: 0,
            sequence: 1,
        };
        save(&mut f, first, &body(5_000, 5), 10);
        save(&mut f, first.after(), &body(5_000, 6), 20);
        // Flip one body byte in slot 1 (as a program could: clear a bit).
        let at = f.at(layout().slot_offset(1) + SECTOR + 4_321);
        f.mem[at] ^= 0x10;
        let r = load(&mut f, &layout(), &mut buf());
        assert_eq!(r.slots[1], Err(SlotError::BodyCrc));
        let l = r.loaded.unwrap();
        assert_eq!((l.slot, l.header.fetched_unix_s), (0, 10));
        // Next save overwrites the corrupt slot, above its sequence.
        assert_eq!(
            r.next_target(),
            Target {
                slot: 1,
                sequence: 3
            }
        );
    }

    #[test]
    fn a_wrong_version_or_damaged_header_is_skipped() {
        let mut f = Nor::new();
        let first = Target {
            slot: 0,
            sequence: 1,
        };
        save(&mut f, first, &body(100, 7), 10);
        save(&mut f, first.after(), &body(100, 8), 20);
        let at = f.at(layout().slot_offset(1) + 4);
        f.mem[at] = 0; // version 1 → 0 (and header CRC no longer matches)
        let r = load(&mut f, &layout(), &mut buf());
        assert!(matches!(r.slots[1], Err(SlotError::Version(_))));
        assert_eq!(r.loaded.unwrap().slot, 0);
        // Both damaged: nothing loads, a fetch is needed.
        let at = f.at(layout().slot_offset(0) + 9);
        f.mem[at] ^= 1;
        let r = load(&mut f, &layout(), &mut buf());
        assert_eq!(r.slots[0], Err(SlotError::HeaderCrc));
        assert_eq!(r.loaded, None);
        assert_eq!(r.next_target().slot, 0);
    }

    #[test]
    fn higher_sequence_wins_regardless_of_slot() {
        let mut f = Nor::new();
        save(
            &mut f,
            Target {
                slot: 1,
                sequence: 4,
            },
            &body(50, 9),
            40,
        );
        save(
            &mut f,
            Target {
                slot: 0,
                sequence: 5,
            },
            &body(50, 10),
            50,
        );
        let r = load(&mut f, &layout(), &mut buf());
        assert_eq!(r.loaded.unwrap().header.fetched_unix_s, 50);
    }
}
