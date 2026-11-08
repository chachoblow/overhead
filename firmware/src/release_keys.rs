//! 0085's accepted release keys: ed25519 public keys, never private ones.
//! The user generates each on the Mac (`overhead-release keygen`) and adds
//! the printed public key here. Empty until the first key exists, so until
//! then only `ota-dev` builds (trusting their own dev key) accept updates.
//! Rotation: a release signed by the old key ships a list with the new one.
pub const KEYS: [[u8; 32]; 0] = [];
