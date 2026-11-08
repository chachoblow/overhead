//! 0085's accepted release keys: ed25519 public keys, never private ones.
//! The user generates each on the Mac (`overhead-release keygen`) and adds
//! the printed public key here. `ota-dev` drill builds also trust their own dev key.
//! Rotation: a release signed by the old key ships a list with the new one.
pub const KEYS: [[u8; 32]; 1] = [
    // Key 1, generated on the user's Mac 2026-11-08:
    // 961ae46efa99916dd4354e1b1fb28a133e8a1ed5afa8b00543ed595564692496
    [
        0x96, 0x1a, 0xe4, 0x6e, 0xfa, 0x99, 0x91, 0x6d,
        0xd4, 0x35, 0x4e, 0x1b, 0x1f, 0xb2, 0x8a, 0x13,
        0x3e, 0x8a, 0x1e, 0xd5, 0xaf, 0xa8, 0xb0, 0x05,
        0x43, 0xed, 0x59, 0x55, 0x64, 0x69, 0x24, 0x96,
    ],
];
