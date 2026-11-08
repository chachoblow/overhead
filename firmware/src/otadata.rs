//! ESP-IDF `otadata` entries (0083/0086), read and written directly rather
//! than through `esp-bootloader-esp-idf` 0.6.0's `Ota`: that crate's slot
//! choice after an erased `otadata` (every USB flash) computes sequence 0
//! for `ota_1` and can pick the running slot as "next". Host-tested.
//!
//! The 8 KiB partition holds two 4 KiB sectors, each starting with one
//! 32-byte entry: `ota_seq` u32, a 20-byte label (unused, 0xFF), `ota_state`
//! u32, and a CRC-32 of `ota_seq` (ROM `crc32_le(u32::MAX, ..)`). The
//! bootloader boots app slot `(seq - 1) % 2` of the valid entry with the
//! highest sequence. A sector is erased before its entry is programmed.

pub const SECTOR: u32 = 4096;
pub const ENTRY_LEN: usize = 32;

/// `esp_ota_img_states_t`.
pub mod state {
    pub const NEW: u32 = 0;
    pub const PENDING_VERIFY: u32 = 1;
    pub const VALID: u32 = 2;
    pub const INVALID: u32 = 3;
    pub const ABORTED: u32 = 4;
    pub const UNDEFINED: u32 = u32::MAX;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub seq: u32,
    pub state: u32,
}

/// ROM `crc32_le(u32::MAX, ..)`: reflected 0xEDB88320, register starts at
/// 0, result inverted (not the standard CRC-32 initial value).
pub fn crc(bytes: &[u8]) -> u32 {
    let mut c: u32 = 0;
    for &b in bytes {
        c ^= u32::from(b);
        for _ in 0..8 {
            let mask = (c & 1).wrapping_neg();
            c = (c >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !c
}

impl Entry {
    pub fn encode(&self) -> [u8; ENTRY_LEN] {
        let mut b = [0xFFu8; ENTRY_LEN];
        b[0..4].copy_from_slice(&self.seq.to_le_bytes());
        b[24..28].copy_from_slice(&self.state.to_le_bytes());
        b[28..32].copy_from_slice(&crc(&self.seq.to_le_bytes()).to_le_bytes());
        b
    }

    /// `None` unless the bootloader would accept it (sequence set, CRC ok).
    pub fn decode(b: &[u8; ENTRY_LEN]) -> Option<Self> {
        let word = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
        let seq = word(0);
        (seq != u32::MAX && word(28) == crc(&b[0..4])).then_some(Self {
            seq,
            state: word(24),
        })
    }

    /// The app slot (0 or 1) this entry selects.
    pub fn slot(&self) -> usize {
        (self.seq.wrapping_sub(1) % 2) as usize
    }
}

/// The entry the bootloader acts on: highest valid sequence, and its sector.
pub fn selected(entries: [Option<Entry>; 2]) -> Option<(usize, Entry)> {
    match entries {
        [Some(a), Some(b)] => Some(if b.seq > a.seq { (1, b) } else { (0, a) }),
        [Some(a), None] => Some((0, a)),
        [None, Some(b)] => Some((1, b)),
        [None, None] => None,
    }
}

/// The write that makes app `slot` boot next as `NEW`: the sector not holding
/// the selected entry, with the next sequence that maps to `slot`. A cut
/// write leaves that sector invalid and the current choice in force.
pub fn activate(entries: [Option<Entry>; 2], slot: usize) -> (usize, Entry) {
    let (sector, max) = match selected(entries) {
        Some((s, e)) => (1 - s, e.seq),
        None => (0, 0),
    };
    let mut seq = max + 1;
    if ((seq - 1) % 2) as usize != slot {
        seq += 1;
    }
    (
        sector,
        Entry {
            seq,
            state: state::NEW,
        },
    )
}

/// The write that changes the selected entry's state in place (`Valid` or
/// `Invalid`). `None` after an erased `otadata`: nothing to confirm.
pub fn set_state(entries: [Option<Entry>; 2], new_state: u32) -> Option<(usize, Entry)> {
    selected(entries).map(|(sector, e)| {
        (
            sector,
            Entry {
                seq: e.seq,
                state: new_state,
            },
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_rom_crc32_le() {
        // IDF writes `9a 98 43 47` after ota_seq 1; zlib.crc32(seq, !0)
        // agrees for both values.
        assert_eq!(crc(&1u32.to_le_bytes()), 0x4743_989a);
        assert_eq!(crc(&2u32.to_le_bytes()), 0x55f6_3774);
        assert_eq!(Entry::decode(&Entry { seq: 1, state: 0 }.encode()).map(|e| e.seq), Some(1));
    }

    #[test]
    fn erased_and_corrupt_entries_are_invalid() {
        assert_eq!(Entry::decode(&[0xFF; ENTRY_LEN]), None);
        let mut b = Entry { seq: 3, state: state::VALID }.encode();
        b[0] ^= 1;
        assert_eq!(Entry::decode(&b), None);
    }

    #[test]
    fn selection_follows_highest_sequence() {
        let e = |seq| Some(Entry { seq, state: state::VALID });
        assert_eq!(selected([None, None]), None);
        assert_eq!(selected([e(1), None]).map(|(s, x)| (s, x.slot())), Some((0, 0)));
        assert_eq!(selected([e(1), e(2)]).map(|(s, x)| (s, x.slot())), Some((1, 1)));
        assert_eq!(selected([e(5), e(4)]).map(|(s, x)| (s, x.slot())), Some((0, 0)));
    }

    #[test]
    fn activation_after_usb_flash_targets_ota_1() {
        // USB flash: otadata erased, bootloader runs ota_0.
        let (sector, e) = activate([None, None], 1);
        assert_eq!((sector, e.seq, e.slot(), e.state), (0, 2, 1, state::NEW));
        let (sector, e) = activate([None, None], 0);
        assert_eq!((sector, e.seq, e.slot()), (0, 1, 0));
    }

    #[test]
    fn activation_alternates_sectors_and_slots() {
        let mut entries = [None, None];
        let mut slot = 1;
        for _ in 0..6 {
            let (sector, e) = activate(entries, slot);
            assert_eq!(e.slot(), slot);
            assert_ne!(Some(sector), selected(entries).map(|(s, _)| s));
            entries[sector] = Some(e);
            assert_eq!(selected(entries).map(|(_, e)| e.slot()), Some(slot));
            slot = 1 - slot;
        }
    }

    #[test]
    fn a_cut_activation_keeps_the_running_slot() {
        let running = [Some(Entry { seq: 1, state: state::VALID }), None];
        let (sector, _) = activate(running, 1);
        let mut after = running;
        after[sector] = None; // erased, never programmed
        assert_eq!(selected(after).map(|(_, e)| e.slot()), Some(0));
    }

    #[test]
    fn state_rewrites_the_selected_entry() {
        let entries = [
            Some(Entry { seq: 1, state: state::VALID }),
            Some(Entry { seq: 2, state: state::PENDING_VERIFY }),
        ];
        assert_eq!(
            set_state(entries, state::VALID),
            Some((1, Entry { seq: 2, state: state::VALID }))
        );
        assert_eq!(set_state([None, None], state::VALID), None);
    }
}
