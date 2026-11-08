#![no_std]
//! Signed update manifests ([0085](../../docs/decisions/0085-signed-updates-and-release-keys.md)),
//! shared by the device (verify, decide, hash the streamed image) and the host
//! `overhead-release` tool (sign). no_std + alloc; no I/O.
//!
//! The manifest is JSON for people and tools, but the signature covers
//! [`signed_bytes`]: a fixed-order line format rebuilt from the parsed fields,
//! so JSON spacing, key order or extra keys never change what was signed.

extern crate alloc;

use alloc::{string::String, vec::Vec};
use core::fmt::{self, Write as _};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};

/// The only manifest format this firmware reads. A later format is served at
/// a new address so devices on this one keep finding format 1.
pub const FORMAT: u32 = 1;
/// 0083: each app slot is 3 MiB; a longer image cannot be installed.
pub const MAX_IMAGE_LEN: u32 = 3 * 1024 * 1024;
const MAX_VERSION_LEN: usize = 32;
const MAX_IMAGE_PATH_LEN: usize = 64;
/// First line of the signed bytes; binds the signature to this format.
const DOMAIN: &str = "overhead-manifest-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Beta,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Beta => "beta",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "stable" => Some(Channel::Stable),
            "beta" => Some(Channel::Beta),
            _ => None,
        }
    }
}

/// The signed fields of one channel's manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub channel: Channel,
    /// Monotonic across all channels; a device installs only a larger one.
    pub build: u32,
    pub version: String,
    /// Path of the image relative to the `ota/` directory.
    pub image: String,
    pub length: u32,
    pub sha256: [u8; 32],
    /// Oldest settings-record format the new image reads (0084).
    pub min_settings: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Json,
    Format,
    Channel,
    Version,
    ImagePath,
    Length,
    Hex,
    /// No accepted key verifies the signature.
    Signature,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Error::Json => "json",
            Error::Format => "format",
            Error::Channel => "channel",
            Error::Version => "version",
            Error::ImagePath => "image-path",
            Error::Length => "length",
            Error::Hex => "hex",
            Error::Signature => "signature",
        })
    }
}

#[derive(Deserialize)]
struct Wire<'a> {
    format: u32,
    #[serde(borrow)]
    channel: &'a str,
    build: u32,
    #[serde(borrow)]
    version: &'a str,
    #[serde(borrow)]
    image: &'a str,
    length: u32,
    #[serde(borrow)]
    sha256: &'a str,
    min_settings: u32,
    #[serde(borrow)]
    signature: &'a str,
}

/// The exact bytes the release key signs.
pub fn signed_bytes(m: &Manifest) -> Vec<u8> {
    let mut s = String::new();
    // Writing to a String cannot fail.
    let _ = write!(
        s,
        "{DOMAIN}\nchannel={}\nbuild={}\nversion={}\nimage={}\nlength={}\nsha256=",
        m.channel.as_str(),
        m.build,
        m.version,
        m.image,
        m.length
    );
    push_hex(&mut s, &m.sha256);
    let _ = write!(s, "\nmin_settings={}\n", m.min_settings);
    s.into_bytes()
}

/// Parses `json`, checks every field, and verifies the signature against each
/// key in `keys` (0085's short accepted list). Returns the manifest and the
/// index of the key that verified it.
pub fn verify(json: &[u8], keys: &[[u8; 32]]) -> Result<(Manifest, usize), Error> {
    let (m, sig) = parse(json)?;
    let msg = signed_bytes(&m);
    let sig = Signature::from_bytes(&sig);
    for (i, k) in keys.iter().enumerate() {
        let Ok(key) = VerifyingKey::from_bytes(k) else {
            continue;
        };
        if key.verify_strict(&msg, &sig).is_ok() {
            return Ok((m, i));
        }
    }
    Err(Error::Signature)
}

/// Parses and checks the fields without checking the signature.
pub fn parse(json: &[u8]) -> Result<(Manifest, [u8; 64]), Error> {
    let w: Wire<'_> = serde_json::from_slice(json).map_err(|_| Error::Json)?;
    if w.format != FORMAT {
        return Err(Error::Format);
    }
    let channel = Channel::parse(w.channel).ok_or(Error::Channel)?;
    if !valid_version(w.version) {
        return Err(Error::Version);
    }
    if !valid_image_path(w.image) {
        return Err(Error::ImagePath);
    }
    if w.length == 0 || w.length > MAX_IMAGE_LEN {
        return Err(Error::Length);
    }
    let sha256 = parse_hex::<32>(w.sha256)?;
    let signature = parse_hex::<64>(w.signature)?;
    Ok((
        Manifest {
            channel,
            build: w.build,
            version: w.version.into(),
            image: w.image.into(),
            length: w.length,
            sha256,
            min_settings: w.min_settings,
        },
        signature,
    ))
}

/// The manifest JSON a device reads, with `signature` appended.
pub fn to_json(m: &Manifest, signature: &[u8; 64]) -> String {
    let mut s = String::new();
    let _ = write!(
        s,
        "{{\"format\":{FORMAT},\"channel\":\"{}\",\"build\":{},\"version\":\"{}\",\"image\":\"{}\",\"length\":{},\"sha256\":\"",
        m.channel.as_str(),
        m.build,
        m.version,
        m.image,
        m.length
    );
    push_hex(&mut s, &m.sha256);
    let _ = write!(s, "\",\"min_settings\":{},\"signature\":\"", m.min_settings);
    push_hex(&mut s, signature);
    s.push_str("\"}\n");
    s
}

/// Signs `m` with a raw 32-byte ed25519 secret key (host only).
#[cfg(feature = "sign")]
pub fn sign(m: &Manifest, secret: &[u8; 32]) -> [u8; 64] {
    use ed25519_dalek::{Signer, SigningKey};
    SigningKey::from_bytes(secret)
        .sign(&signed_bytes(m))
        .to_bytes()
}

/// The public key for a raw secret key (host only).
#[cfg(feature = "sign")]
pub fn public_key(secret: &[u8; 32]) -> [u8; 32] {
    ed25519_dalek::SigningKey::from_bytes(secret)
        .verifying_key()
        .to_bytes()
}

/// What the running device knows when it reads a verified manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Running {
    pub channel: Channel,
    pub build: u32,
    /// The last build that failed its health check (0084), if any.
    pub failed_build: Option<u32>,
    /// The settings-record format this device holds.
    pub settings_format: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Offer {
    Install,
    /// The manifest is for another channel (a misplaced file).
    OtherChannel,
    /// Not newer than the running build (also refuses a replayed old one).
    Current,
    /// This build already failed its health check here.
    Failed,
    /// The new image would not read this device's settings.
    SettingsTooOld,
}

/// 0085's device checks, after the signature.
pub fn offer(m: &Manifest, r: &Running) -> Offer {
    if m.channel != r.channel {
        Offer::OtherChannel
    } else if m.build <= r.build {
        Offer::Current
    } else if r.failed_build == Some(m.build) {
        Offer::Failed
    } else if r.settings_format < m.min_settings {
        Offer::SettingsTooOld
    } else {
        Offer::Install
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageError {
    TooLong,
    Short,
    Hash,
}

/// Hashes a streamed image and checks it against the manifest; the image is
/// never held whole.
pub struct ImageCheck {
    hasher: Sha256,
    seen: u32,
    length: u32,
    sha256: [u8; 32],
}

impl ImageCheck {
    pub fn new(m: &Manifest) -> Self {
        Self {
            hasher: Sha256::new(),
            seen: 0,
            length: m.length,
            sha256: m.sha256,
        }
    }

    pub fn seen(&self) -> u32 {
        self.seen
    }

    pub fn remaining(&self) -> u32 {
        self.length - self.seen
    }

    /// Accepts the next chunk; refuses one that would pass the length.
    pub fn feed(&mut self, chunk: &[u8]) -> Result<(), ImageError> {
        let n = u32::try_from(chunk.len()).map_err(|_| ImageError::TooLong)?;
        if n > self.remaining() {
            return Err(ImageError::TooLong);
        }
        self.hasher.update(chunk);
        self.seen += n;
        Ok(())
    }

    pub fn finish(self) -> Result<(), ImageError> {
        if self.seen != self.length {
            return Err(ImageError::Short);
        }
        let got: [u8; 32] = self.hasher.finalize().into();
        if got != self.sha256 {
            return Err(ImageError::Hash);
        }
        Ok(())
    }
}

/// SHA-256 of a whole buffer (host tools and tests).
pub fn sha256(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

fn valid_version(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= MAX_VERSION_LEN
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+'))
}

fn valid_image_path(p: &str) -> bool {
    !p.is_empty()
        && p.len() <= MAX_IMAGE_PATH_LEN
        && !p.starts_with('/')
        && p.split('/')
            .all(|seg| !seg.is_empty() && seg != "." && seg != "..")
        && p.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'/'))
}

fn push_hex(s: &mut String, bytes: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for b in bytes {
        s.push(HEX[usize::from(b >> 4)] as char);
        s.push(HEX[usize::from(b & 15)] as char);
    }
}

/// Lowercase hex of exactly `N` bytes.
pub fn parse_hex<const N: usize>(s: &str) -> Result<[u8; N], Error> {
    let b = s.as_bytes();
    if b.len() != 2 * N {
        return Err(Error::Hex);
    }
    let nib = |c: u8| match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        _ => Err(Error::Hex),
    };
    let mut out = [0u8; N];
    for (i, o) in out.iter_mut().enumerate() {
        *o = (nib(b[2 * i])? << 4) | nib(b[2 * i + 1])?;
    }
    Ok(out)
}

/// Lowercase hex of `bytes`.
pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::new();
    push_hex(&mut s, bytes);
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    const SECRET: [u8; 32] = [7; 32];
    const OTHER: [u8; 32] = [9; 32];

    fn manifest() -> Manifest {
        Manifest {
            channel: Channel::Beta,
            build: 5,
            version: "0.5.0-beta".into(),
            image: "images/5.bin".into(),
            length: 11,
            sha256: sha256(b"hello image"),
            min_settings: 1,
        }
    }

    fn signed(m: &Manifest) -> String {
        to_json(m, &sign(m, &SECRET))
    }

    #[test]
    fn signed_bytes_are_fixed() {
        let s = String::from_utf8(signed_bytes(&manifest())).unwrap();
        assert_eq!(
            s,
            "overhead-manifest-v1\nchannel=beta\nbuild=5\nversion=0.5.0-beta\nimage=images/5.bin\nlength=11\nsha256=".to_string()
                + &hex(&sha256(b"hello image"))
                + "\nmin_settings=1\n"
        );
    }

    #[test]
    fn round_trip_verifies_with_listed_key() {
        let m = manifest();
        let json = signed(&m);
        let keys = [public_key(&OTHER), public_key(&SECRET)];
        assert_eq!(verify(json.as_bytes(), &keys), Ok((m, 1)));
    }

    #[test]
    fn unknown_key_is_refused() {
        let json = signed(&manifest());
        assert_eq!(
            verify(json.as_bytes(), &[public_key(&OTHER)]),
            Err(Error::Signature)
        );
    }

    #[test]
    fn any_changed_field_breaks_the_signature() {
        let m = manifest();
        let sig = sign(&m, &SECRET);
        let keys = [public_key(&SECRET)];
        let mut changed = [
            m.clone(),
            m.clone(),
            m.clone(),
            m.clone(),
            m.clone(),
            m.clone(),
        ];
        changed[0].build = 6;
        changed[1].channel = Channel::Stable;
        changed[2].length = 12;
        changed[3].sha256[0] ^= 1;
        changed[4].image = "images/6.bin".into();
        changed[5].min_settings = 0;
        for c in changed {
            assert_eq!(
                verify(to_json(&c, &sig).as_bytes(), &keys),
                Err(Error::Signature)
            );
        }
    }

    #[test]
    fn json_spacing_and_order_do_not_matter() {
        let m = manifest();
        let sig = hex(&sign(&m, &SECRET));
        let sha = hex(&m.sha256);
        let json = alloc::format!(
            "{{ \"signature\": \"{sig}\", \"min_settings\": 1, \"sha256\": \"{sha}\",\n \"length\": 11, \"image\": \"images/5.bin\", \"version\": \"0.5.0-beta\", \"build\": 5, \"channel\": \"beta\", \"format\": 1, \"note\": \"ignored\" }}"
        );
        assert_eq!(verify(json.as_bytes(), &[public_key(&SECRET)]), Ok((m, 0)));
    }

    #[test]
    fn field_checks() {
        let base = signed(&manifest());
        let cases = [
            ("\"format\":1", "\"format\":2", Error::Format),
            ("\"beta\"", "\"alpha\"", Error::Channel),
            ("0.5.0-beta", "0.5 beta", Error::Version),
            ("images/5.bin", "../5.bin", Error::ImagePath),
            ("images/5.bin", "/images/5.bin", Error::ImagePath),
            ("images/5.bin", "images//5.bin", Error::ImagePath),
            ("\"length\":11", "\"length\":0", Error::Length),
            ("\"length\":11", "\"length\":3145729", Error::Length),
            ("\"build\":5", "\"build\":-1", Error::Json),
        ];
        for (from, to, err) in cases {
            let j = base.replacen(from, to, 1);
            assert_eq!(parse(j.as_bytes()).map(|_| ()), Err(err), "{to}");
        }
        let upper = base.replacen("\"sha256\":\"", "\"sha256\":\"A", 1);
        assert_eq!(parse(upper.as_bytes()).map(|_| ()), Err(Error::Hex));
        assert_eq!(parse(b"not json").map(|_| ()), Err(Error::Json));
    }

    #[test]
    fn offer_rules() {
        let m = manifest();
        let r = Running {
            channel: Channel::Beta,
            build: 4,
            failed_build: None,
            settings_format: 1,
        };
        assert_eq!(offer(&m, &r), Offer::Install);
        assert_eq!(
            offer(
                &m,
                &Running {
                    channel: Channel::Stable,
                    ..r
                }
            ),
            Offer::OtherChannel
        );
        assert_eq!(offer(&m, &Running { build: 5, ..r }), Offer::Current);
        assert_eq!(offer(&m, &Running { build: 9, ..r }), Offer::Current);
        assert_eq!(
            offer(
                &m,
                &Running {
                    failed_build: Some(5),
                    ..r
                }
            ),
            Offer::Failed
        );
        assert_eq!(
            offer(
                &m,
                &Running {
                    failed_build: Some(3),
                    ..r
                }
            ),
            Offer::Install
        );
        assert_eq!(
            offer(
                &m,
                &Running {
                    settings_format: 0,
                    ..r
                }
            ),
            Offer::SettingsTooOld
        );
    }

    #[test]
    fn image_check_streams() {
        let m = manifest();
        let mut c = ImageCheck::new(&m);
        c.feed(b"hello").unwrap();
        c.feed(b" image").unwrap();
        assert_eq!(c.finish(), Ok(()));

        let mut short = ImageCheck::new(&m);
        short.feed(b"hello").unwrap();
        assert_eq!(short.finish(), Err(ImageError::Short));

        let mut long = ImageCheck::new(&m);
        long.feed(b"hello image").unwrap();
        assert_eq!(long.feed(b"!"), Err(ImageError::TooLong));

        let mut wrong = ImageCheck::new(&m);
        wrong.feed(b"hello imagE").unwrap();
        assert_eq!(wrong.finish(), Err(ImageError::Hash));
    }
}
