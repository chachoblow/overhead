//! Certificate validity dates for the element fetch (0074), checked in Rust
//! because the prebuilt MbedTLS library has no wall clock (0073).
//!
//! MbedTLS still verifies chain, signatures and hostname. The caller records
//! the server's inbound bytes during the TLS 1.2 handshake, where the
//! Certificate message travels in clear; the authenticated Finished exchange
//! binds those bytes to the session, so after a successful handshake they are
//! exactly the certificates MbedTLS verified. This module reassembles the
//! handshake, extracts every served certificate and requires each to be within
//! `notBefore..=notAfter` at trusted UTC. Anything unexpected fails closed:
//! a TLS 1.3 server (encrypted certificates) yields `NoCertificate`.
//! Allocation-free and host-tested; no transport or clock lives here.

/// Recorded inbound bytes. CelesTrak's ServerHello plus three-certificate
/// chain measured ~5.1 KB on 2026-11-01.
pub const CAPTURE: usize = 8 * 1024;
/// Served certificates examined; CelesTrak sends three.
pub const MAX_CERTIFICATES: usize = 6;

const RECORD_HANDSHAKE: u8 = 22;
const RECORD_CHANGE_CIPHER_SPEC: u8 = 20;
const HANDSHAKE_CERTIFICATE: u8 = 11;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CertError {
    /// The capture ended before the Certificate message was complete.
    Truncated,
    /// A record other than handshake/ChangeCipherSpec preceded the certificates.
    UnexpectedRecord(u8),
    /// The handshake reached ChangeCipherSpec, or ended, with no certificates.
    NoCertificate,
    /// Inconsistent handshake or certificate-list framing.
    Malformed,
    /// An empty certificate list.
    Empty,
    TooMany,
    /// The certificate at this index (0 = leaf) has no readable validity.
    Der(usize),
    NotYetValid {
        index: usize,
        not_before: i64,
    },
    Expired {
        index: usize,
        not_after: i64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Validity {
    /// Unix seconds, inclusive.
    pub not_before: i64,
    pub not_after: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checked {
    pub certificates: usize,
    /// Leaf validity, for the report.
    pub leaf: Validity,
    /// Earliest `notAfter` across the served chain.
    pub earliest_not_after: i64,
}

/// Check every served certificate in `capture` (raw inbound TLS bytes from the
/// start of the connection) against `now_unix_s`. Reassembles in place.
pub fn check(capture: &mut [u8], now_unix_s: i64) -> Result<Checked, CertError> {
    let handshake = reassemble(capture)?;
    let mut list = [&[][..]; MAX_CERTIFICATES];
    let count = certificates(handshake, &mut list)?;
    let mut leaf = None;
    let mut earliest_not_after = i64::MAX;
    for (index, der) in list[..count].iter().enumerate() {
        let v = validity(der).ok_or(CertError::Der(index))?;
        if now_unix_s < v.not_before {
            return Err(CertError::NotYetValid {
                index,
                not_before: v.not_before,
            });
        }
        if now_unix_s > v.not_after {
            return Err(CertError::Expired {
                index,
                not_after: v.not_after,
            });
        }
        leaf.get_or_insert(v);
        earliest_not_after = earliest_not_after.min(v.not_after);
    }
    Ok(Checked {
        certificates: count,
        leaf: leaf.ok_or(CertError::Empty)?,
        earliest_not_after,
    })
}

/// Concatenate handshake-record payloads in place, stopping at
/// ChangeCipherSpec (later records are encrypted) or at a partial record.
fn reassemble(capture: &mut [u8]) -> Result<&[u8], CertError> {
    let (mut read, mut write) = (0, 0);
    while capture.len() - read >= 5 {
        let kind = capture[read];
        let len = usize::from(u16::from_be_bytes([capture[read + 3], capture[read + 4]]));
        match kind {
            RECORD_HANDSHAKE => {}
            RECORD_CHANGE_CIPHER_SPEC => break,
            other => return Err(CertError::UnexpectedRecord(other)),
        }
        let start = read + 5;
        let Some(end) = start.checked_add(len).filter(|&e| e <= capture.len()) else {
            // Keep the partial payload: the certificates may already be whole.
            let tail = capture.len() - start;
            capture.copy_within(start.., write);
            write += tail;
            break;
        };
        capture.copy_within(start..end, write);
        write += len;
        read = end;
    }
    Ok(&capture[..write])
}

/// Find the first Certificate message and split its list into `out`.
fn certificates<'a>(
    mut handshake: &'a [u8],
    out: &mut [&'a [u8]; MAX_CERTIFICATES],
) -> Result<usize, CertError> {
    loop {
        if handshake.is_empty() {
            return Err(CertError::NoCertificate);
        }
        let Some((kind, body, rest)) = handshake
            .split_first()
            .and_then(|(&kind, rest)| take24(rest).map(|(body, rest)| (kind, body, rest)))
        else {
            return Err(CertError::Truncated);
        };
        if kind != HANDSHAKE_CERTIFICATE {
            handshake = rest;
            continue;
        }
        let (mut list, trailing) = take24(body).ok_or(CertError::Malformed)?;
        if !trailing.is_empty() {
            return Err(CertError::Malformed);
        }
        let mut count = 0;
        while !list.is_empty() {
            let (der, rest) = take24(list).ok_or(CertError::Malformed)?;
            *out.get_mut(count).ok_or(CertError::TooMany)? = der;
            count += 1;
            list = rest;
        }
        return if count == 0 {
            Err(CertError::Empty)
        } else {
            Ok(count)
        };
    }
}

/// A 24-bit length-prefixed field and the bytes after it.
fn take24(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let (len, rest) = bytes.split_at_checked(3)?;
    let len = usize::from(len[0]) << 16 | usize::from(len[1]) << 8 | usize::from(len[2]);
    rest.split_at_checked(len)
}

/// `Certificate.tbsCertificate.validity` from DER (RFC 5280 §4.1).
pub fn validity(der: &[u8]) -> Option<Validity> {
    let (certificate, _) = tlv(der, 0x30)?;
    let (mut tbs, _) = tlv(certificate, 0x30)?;
    if tbs.first() == Some(&0xA0) {
        tbs = tlv(tbs, 0xA0)?.1; // explicit version
    }
    let tbs = tlv(tbs, 0x02)?.1; // serialNumber
    let tbs = tlv(tbs, 0x30)?.1; // signature AlgorithmIdentifier
    let tbs = tlv(tbs, 0x30)?.1; // issuer
    let (validity, _) = tlv(tbs, 0x30)?;
    let (not_before, rest) = time(validity)?;
    let (not_after, rest) = time(rest)?;
    (rest.is_empty() && not_before <= not_after).then_some(Validity {
        not_before,
        not_after,
    })
}

/// One DER element with the expected tag: (contents, following bytes).
fn tlv(bytes: &[u8], tag: u8) -> Option<(&[u8], &[u8])> {
    let (&found, rest) = bytes.split_first()?;
    if found != tag {
        return None;
    }
    let (&first, rest) = rest.split_first()?;
    let (len, rest) = if first < 0x80 {
        (usize::from(first), rest)
    } else {
        let n = usize::from(first & 0x7F);
        if n == 0 || n > 3 {
            return None;
        }
        let (digits, rest) = rest.split_at_checked(n)?;
        let len = digits.iter().fold(0, |acc, &d| acc << 8 | usize::from(d));
        (len, rest)
    };
    rest.split_at_checked(len)
}

/// UTCTime (`YYMMDDHHMMSSZ`, RFC 5280: 50–99 → 19xx) or GeneralizedTime
/// (`YYYYMMDDHHMMSSZ`) as Unix seconds.
fn time(bytes: &[u8]) -> Option<(i64, &[u8])> {
    let (contents, rest) = tlv(bytes, 0x17)
        .map(|(c, r)| (c, r, true))
        .or_else(|| tlv(bytes, 0x18).map(|(c, r)| (c, r, false)))
        .map(|(c, r, utc)| ((c, utc), r))?;
    let (text, utc) = contents;
    let (digits, zone) = text.split_last().map(|(z, d)| (d, *z))?;
    if zone != b'Z' || digits.len() != if utc { 12 } else { 14 } {
        return None;
    }
    let mut fields = digits.chunks(2).map(|pair| {
        let tens = pair[0].checked_sub(b'0').filter(|d| *d < 10)?;
        let ones = pair[1].checked_sub(b'0').filter(|d| *d < 10)?;
        Some(i64::from(tens * 10 + ones))
    });
    let mut next = || fields.next().flatten();
    let year = if utc {
        let yy = next()?;
        if yy < 50 { 2000 + yy } else { 1900 + yy }
    } else {
        next()? * 100 + next()?
    };
    let (month, day, hour, minute, second) = (next()?, next()?, next()?, next()?, next()?);
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    let days = days_from_civil(year, month, day);
    Some((days * 86_400 + hour * 3600 + minute * 60 + second, rest))
}

/// Days since 1970-01-01 in the proleptic Gregorian calendar (H. Hinnant).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    use overhead_core::sgp4::chrono::NaiveDate;

    // Served by celestrak.org on 2026-11-01 (public certificates).
    const LEAF: &[u8] = include_bytes!("../fixtures/celestrak-chain-20261101/leaf.der");
    const DV_R36: &[u8] = include_bytes!("../fixtures/celestrak-chain-20261101/dv-r36.der");
    const R46_CROSS: &[u8] =
        include_bytes!("../fixtures/celestrak-chain-20261101/r46-by-usertrust.der");
    const ROOTS_PEM: &str = include_str!("../certs/celestrak-roots.pem");

    fn unix(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> i64 {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, s)
            .unwrap()
            .and_utc()
            .timestamp()
    }

    fn handshake_message(kind: u8, body: &[u8]) -> Vec<u8> {
        let mut m = alloc::vec![kind];
        m.extend_from_slice(&(body.len() as u32).to_be_bytes()[1..]);
        m.extend_from_slice(body);
        m
    }

    fn certificate_message(chain: &[&[u8]]) -> Vec<u8> {
        let mut list = Vec::new();
        for der in chain {
            list.extend_from_slice(&(der.len() as u32).to_be_bytes()[1..]);
            list.extend_from_slice(der);
        }
        let mut body = (list.len() as u32).to_be_bytes()[1..].to_vec();
        body.extend_from_slice(&list);
        handshake_message(11, &body)
    }

    /// ServerHello, Certificate, ServerHelloDone in handshake records of at
    /// most `fragment` bytes, then ChangeCipherSpec and opaque encrypted bytes.
    fn capture(chain: &[&[u8]], fragment: usize) -> Vec<u8> {
        let mut handshake = handshake_message(2, &[0x03, 0x03, 7, 7, 7]);
        handshake.extend(certificate_message(chain));
        handshake.extend(handshake_message(14, &[]));
        let mut out = Vec::new();
        for chunk in handshake.chunks(fragment) {
            out.extend_from_slice(&[22, 3, 3]);
            out.extend_from_slice(&(chunk.len() as u16).to_be_bytes());
            out.extend_from_slice(chunk);
        }
        out.extend_from_slice(&[20, 3, 3, 0, 1, 1, 23, 3, 3, 0, 4, 0xde, 0xad, 0xbe, 0xef]);
        out
    }

    const CHAIN: [&[u8]; 3] = [LEAF, DV_R36, R46_CROSS];

    #[test]
    fn fixture_validity_matches_openssl() {
        // `openssl x509 -startdate -enddate` on the served chain, 2026-11-01.
        assert_eq!(
            validity(LEAF),
            Some(Validity {
                not_before: unix(2026, 5, 27, 0, 0, 0),
                not_after: unix(2026, 12, 11, 23, 59, 59),
            })
        );
        assert_eq!(
            validity(DV_R36),
            Some(Validity {
                not_before: unix(2021, 3, 22, 0, 0, 0),
                not_after: unix(2036, 3, 21, 23, 59, 59),
            })
        );
        assert_eq!(
            validity(R46_CROSS),
            Some(Validity {
                not_before: unix(2021, 3, 22, 0, 0, 0),
                not_after: unix(2038, 1, 18, 23, 59, 59),
            })
        );
    }

    #[test]
    fn served_chain_is_valid_today_whole_or_fragmented() {
        let now = unix(2026, 11, 1, 7, 0, 0);
        for fragment in [16_384, 1000, 1] {
            let mut bytes = capture(&CHAIN, fragment);
            let checked = check(&mut bytes, now).unwrap();
            assert_eq!(checked.certificates, 3);
            assert_eq!(checked.leaf, validity(LEAF).unwrap());
            assert_eq!(checked.earliest_not_after, unix(2026, 12, 11, 23, 59, 59));
        }
        // CelesTrak's real chain fits the capture with room to spare.
        assert!(capture(&CHAIN, 16_384).len() < CAPTURE * 3 / 4);
    }

    #[test]
    fn bounds_are_inclusive_and_each_certificate_counts() {
        let leaf = validity(LEAF).unwrap();
        for now in [leaf.not_before, leaf.not_after] {
            assert!(check(&mut capture(&CHAIN, 16_384), now).is_ok());
        }
        assert_eq!(
            check(&mut capture(&CHAIN, 16_384), leaf.not_after + 1),
            Err(CertError::Expired {
                index: 0,
                not_after: leaf.not_after
            })
        );
        assert_eq!(
            check(&mut capture(&CHAIN, 16_384), leaf.not_before - 1),
            Err(CertError::NotYetValid {
                index: 0,
                not_before: leaf.not_before
            })
        );
        // An expired intermediate fails even while the leaf is current.
        let now = unix(2026, 11, 1, 0, 0, 0);
        let mut old = DV_R36.to_vec();
        let at = old.windows(13).position(|w| w == b"360321235959Z").unwrap();
        old[at..at + 13].copy_from_slice(b"260101000000Z");
        assert_eq!(
            check(&mut capture(&[LEAF, &old, R46_CROSS], 16_384), now),
            Err(CertError::Expired {
                index: 1,
                not_after: unix(2026, 1, 1, 0, 0, 0)
            })
        );
    }

    #[test]
    fn missing_or_damaged_handshakes_fail_closed() {
        let now = unix(2026, 11, 1, 0, 0, 0);
        let full = capture(&CHAIN, 16_384);
        // Cut inside the Certificate message.
        assert_eq!(
            check(&mut full[..2000].to_vec(), now),
            Err(CertError::Truncated)
        );
        assert_eq!(check(&mut Vec::new(), now), Err(CertError::NoCertificate));
        // ChangeCipherSpec before any Certificate (e.g. resumption).
        let mut ccs = alloc::vec![22, 3, 3, 0, 9];
        ccs.extend(handshake_message(2, &[3, 3, 0, 0, 0]));
        ccs.extend_from_slice(&[20, 3, 3, 0, 1, 1]);
        ccs.extend(capture(&CHAIN, 16_384));
        assert_eq!(check(&mut ccs, now), Err(CertError::NoCertificate));
        // An alert instead of a handshake.
        assert_eq!(
            check(&mut alloc::vec![21, 3, 3, 0, 2, 2, 40], now),
            Err(CertError::UnexpectedRecord(21))
        );
        // TLS 1.3: everything after ServerHello is application_data (23).
        let mut tls13 = alloc::vec![22, 3, 3, 0, 9];
        tls13.extend(handshake_message(2, &[3, 3, 0, 0, 0]));
        tls13.extend_from_slice(&[23, 3, 3, 0, 1, 0]);
        assert_eq!(check(&mut tls13, now), Err(CertError::UnexpectedRecord(23)));
        assert_eq!(check(&mut capture(&[], 16_384), now), Err(CertError::Empty));
        assert_eq!(
            check(&mut capture(&[LEAF; MAX_CERTIFICATES + 1], 16_384), now),
            Err(CertError::TooMany)
        );
        let mut broken = LEAF.to_vec();
        broken[0] = 0x31;
        assert_eq!(
            check(&mut capture(&[LEAF, &broken], 16_384), now),
            Err(CertError::Der(1))
        );
    }

    #[test]
    fn time_forms_and_calendar() {
        let t = |s: &[u8], tag| {
            let mut der = alloc::vec![tag, s.len() as u8];
            der.extend_from_slice(s);
            time(&der).map(|(t, _)| t)
        };
        assert_eq!(t(b"700101000000Z", 0x17), Some(0));
        assert_eq!(
            t(b"491231235959Z", 0x17),
            Some(unix(2049, 12, 31, 23, 59, 59))
        );
        assert_eq!(t(b"500101000000Z", 0x17), Some(unix(1950, 1, 1, 0, 0, 0)));
        assert_eq!(t(b"20500101000000Z", 0x18), Some(unix(2050, 1, 1, 0, 0, 0)));
        assert_eq!(
            t(b"20280229120000Z", 0x18),
            Some(unix(2028, 2, 29, 12, 0, 0))
        );
        for bad in [
            &b"700101000000"[..],
            b"7001010000000Z",
            b"701301000000Z",
            b"700100000000Z",
            b"700101240000Z",
            b"70010100000aZ",
        ] {
            assert_eq!(t(bad, 0x17), None, "{bad:?}");
        }
    }

    /// The bundled roots (not served, not date-checked on the device) must
    /// outlive this prototype; a failing assertion means re-pinning roots.
    #[test]
    fn bundled_roots_are_long_lived() {
        let mut roots = 0;
        for block in ROOTS_PEM.split("-----BEGIN CERTIFICATE-----").skip(1) {
            let body = block.split("-----END CERTIFICATE-----").next().unwrap();
            let der = base64(body);
            let v = validity(&der).unwrap();
            assert!(v.not_before < unix(2026, 11, 1, 0, 0, 0));
            assert!(v.not_after > unix(2037, 1, 1, 0, 0, 0));
            roots += 1;
        }
        assert_eq!(roots, 2);
    }

    fn base64(text: &str) -> Vec<u8> {
        let value = |c: u8| match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => unreachable!(),
        };
        let digits: Vec<u8> = text
            .bytes()
            .filter(|c| !c.is_ascii_whitespace() && *c != b'=')
            .map(value)
            .collect();
        let mut out = Vec::new();
        for chunk in digits.chunks(4) {
            let n = chunk
                .iter()
                .enumerate()
                .fold(0u32, |acc, (i, &d)| acc | u32::from(d) << (18 - 6 * i));
            out.extend_from_slice(&n.to_be_bytes()[1..chunk.len()]);
        }
        out
    }
}
