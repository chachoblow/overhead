//! Host-preflight screen for one unicast SNTP reply (RFC 4330 §5, RFC 5905).
//! Pure and allocation-free: no socket, wireless stack or live time source.
//! It establishes request/reply association and *server-reported* bounds on
//! a benign network only — not authentication, server correctness or
//! oscillator drift (0063). Addresses/ports are checked by the future adapter.

use overhead_core::sgp4::chrono::{DateTime, NaiveDateTime};

/// Only a validated source adapter may construct an observation. The gate
/// cannot verify provenance; `uncertainty_us` must include the entire initial
/// UTC-to-monotonic error, not just packet round-trip time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenedObservation {
    pub midpoint_uptime_us: u64,
    pub utc_at_midpoint: NaiveDateTime,
    pub uncertainty_us: u64,
}

/// A client-mode (3), version-4 request whose transmit field carries `nonce`.
pub fn request(nonce: u64) -> [u8; HEADER_LEN] {
    let mut packet = [0u8; HEADER_LEN];
    packet[0] = (4 << 3) | 3;
    packet[40..48].copy_from_slice(&nonce.to_be_bytes());
    packet
}

/// End-to-end check between two accepted observations: `later` minus the
/// UTC predicted from `earlier` by monotonic elapsed time, and the bound it
/// must respect (both uncertainties plus `drift_ppm` over the interval).
/// `None` if `later` is not after `earlier` or arithmetic overflows.
pub fn consistency(
    earlier: ScreenedObservation,
    later: ScreenedObservation,
    drift_ppm: u64,
) -> Option<(i64, u64)> {
    let elapsed = later
        .midpoint_uptime_us
        .checked_sub(earlier.midpoint_uptime_us)?;
    let predicted = earlier
        .utc_at_midpoint
        .and_utc()
        .timestamp_micros()
        .checked_add(i64::try_from(elapsed).ok()?)?;
    let difference = later
        .utc_at_midpoint
        .and_utc()
        .timestamp_micros()
        .checked_sub(predicted)?;
    let drift =
        u64::try_from((u128::from(elapsed) * u128::from(drift_ppm)).div_ceil(1_000_000)).ok()?;
    let bound = earlier
        .uncertainty_us
        .checked_add(later.uncertainty_us)?
        .checked_add(drift)?;
    Some((difference, bound))
}

const HEADER_LEN: usize = 48;
/// Seconds from the NTP prime epoch (1900) to the Unix epoch.
const NTP_UNIX_OFFSET_S: i64 = 2_208_988_800;
const ERA_S: i64 = 1 << 32;
/// RFC 4330's "cozy" infinity for root delay/dispersion.
const ROOT_LIMIT_US: u64 = 1_000_000;
/// Floor conversion of each of T2/T3 and the half-µs client midpoint.
const CONVERSION_US: u64 = 2;

/// One outstanding request. `transmit` must be unique and nonzero (e.g. a
/// random nonce); it needs no prior UTC knowledge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SntpRequest {
    pub transmit: u64,
    pub sent_uptime_us: u64,
}

/// No-renewal outcomes: none is evidence against an older anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ignored {
    NoPendingRequest,
    Malformed,
    /// Origin does not echo the pending request; the request stays pending.
    Unmatched,
    BadVersionOrMode,
    ZeroTimestamp,
    /// Turnaround exceeds RTT, receive precedes send, or arithmetic overflow.
    ImpossibleTiming,
    RootOutOfRange,
    /// Two NTP eras map into the supported 1957–2100 calendar range.
    AmbiguousEra,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SntpOutcome {
    /// Feed to `UtcTrustPreflight::observe`; it may still be NoRenewal.
    Observation(ScreenedObservation),
    Ignored(Ignored),
    /// Stratum 0: honor rate/deny (adapter); no new time, no revocation.
    KissOfDeath([u8; 4]),
    /// LI=3 or stratum ≥ 16: `source_unsynchronized` (revoke).
    Unsynchronized,
    /// LI=1/2: `source_leap_pending` (revoke, 0063).
    LeapPending,
}

fn ts(b: &[u8], at: usize) -> u64 {
    u64::from_be_bytes(b[at..at + 8].try_into().unwrap())
}

fn short_us_ceil(b: &[u8], at: usize) -> Option<u64> {
    let raw = u32::from_be_bytes(b[at..at + 4].try_into().unwrap());
    if raw & 0x8000_0000 != 0 {
        return None; // negative signed 16.16
    }
    let us = (u64::from(raw) * 1_000_000).div_ceil(1 << 16);
    (us <= ROOT_LIMIT_US).then_some(us)
}

/// Unique-era unfolding: reject if both era 0 and era 1 are supported.
/// Returns microseconds since the Unix epoch.
fn ntp_to_unix_us(raw: u64) -> Result<i64, Ignored> {
    let seconds = (raw >> 32) as i64;
    let micros = ((raw & 0xffff_ffff) * 1_000_000 >> 32) as i64;
    let mut found = None;
    for era in 0..=1 {
        let unix_s = seconds + era * ERA_S - NTP_UNIX_OFFSET_S;
        let unix_us = unix_s * 1_000_000 + micros;
        let supported = DateTime::from_timestamp_micros(unix_us)
            .is_some_and(|t| overhead_core::validate_utc_time(t.naive_utc()).is_ok());
        if supported {
            if found.is_some() {
                return Err(Ignored::AmbiguousEra);
            }
            found = Some(unix_us);
        }
    }
    found.ok_or(Ignored::AmbiguousEra)
}

/// `C` for *bracketing* captures: send uptime read before the packet is handed
/// to the stack, receive uptime read after the stack returns it. Stack/radio
/// queueing then only widens the measured RTT (already in the bound), leaving
/// 1 µs floor resolution plus local drift over the RTT. `drift_ppm` is the
/// same evidenced bound `D` as the trust gate; this helper does not supply it.
pub fn bracketing_capture_bound_us(rtt_us: u64, drift_ppm: u64) -> Option<u64> {
    let drift = u128::from(rtt_us) * u128::from(drift_ppm);
    u64::try_from(drift.div_ceil(1_000_000))
        .ok()?
        .checked_add(1)
}

/// Screen `reply` received at `received_uptime_us` against `pending`.
/// A reply echoing the pending origin consumes it (no duplicate can match).
/// `capture_bound_us` must bound send/receive capture and stack latency
/// asymmetry not inside the measured interval; it is caller evidence.
pub fn screen(
    pending: &mut Option<SntpRequest>,
    reply: &[u8],
    received_uptime_us: u64,
    capture_bound_us: u64,
) -> SntpOutcome {
    use SntpOutcome::Ignored as I;
    let Some(request) = *pending else {
        return I(Ignored::NoPendingRequest);
    };
    if reply.len() < HEADER_LEN {
        return I(Ignored::Malformed);
    }
    if request.transmit == 0 || ts(reply, 24) != request.transmit {
        return I(Ignored::Unmatched);
    }
    *pending = None;
    let (li, vn, mode, stratum) = (reply[0] >> 6, (reply[0] >> 3) & 7, reply[0] & 7, reply[1]);
    if vn != 4 || mode != 4 {
        return I(Ignored::BadVersionOrMode);
    }
    if stratum == 0 {
        return SntpOutcome::KissOfDeath(reply[12..16].try_into().unwrap());
    }
    if li == 3 || stratum >= 16 {
        return SntpOutcome::Unsynchronized;
    }
    if li != 0 {
        return SntpOutcome::LeapPending;
    }
    let (t2, t3) = (ts(reply, 32), ts(reply, 40));
    if t2 == 0 || t3 == 0 {
        return I(Ignored::ZeroTimestamp);
    }
    let (Some(root_delay), Some(root_disp)) = (short_us_ceil(reply, 4), short_us_ceil(reply, 8))
    else {
        return I(Ignored::RootOutOfRange);
    };
    let (t2_us, t3_us) = match (ntp_to_unix_us(t2), ntp_to_unix_us(t3)) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => return I(e),
    };
    let Some(rtt) = received_uptime_us.checked_sub(request.sent_uptime_us) else {
        return I(Ignored::ImpossibleTiming);
    };
    // Server turnaround τ lies inside the client interval, so the server
    // midpoint (T2+T3)/2 is within (RTT−τ)/2 of the client midpoint without
    // assuming symmetric paths.
    let Some(turnaround) = t3_us.checked_sub(t2_us).and_then(|d| u64::try_from(d).ok()) else {
        return I(Ignored::ImpossibleTiming);
    };
    let Some(path) = rtt.checked_sub(turnaround) else {
        return I(Ignored::ImpossibleTiming);
    };
    let server_mid_us = t2_us + (turnaround / 2) as i64;
    let uncertainty = [
        path.div_ceil(2),
        root_delay.div_ceil(2),
        root_disp,
        CONVERSION_US,
        capture_bound_us,
    ]
    .into_iter()
    .try_fold(0u64, u64::checked_add);
    let (Some(uncertainty_us), Some(utc)) = (
        uncertainty,
        DateTime::from_timestamp_micros(server_mid_us).map(|t| t.naive_utc()),
    ) else {
        return I(Ignored::ImpossibleTiming);
    };
    SntpOutcome::Observation(ScreenedObservation {
        midpoint_uptime_us: request.sent_uptime_us + rtt / 2,
        utc_at_midpoint: utc,
        uncertainty_us,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use overhead_core::sgp4::chrono::NaiveDate;

    const NONCE: u64 = 0x0123_4567_89ab_cdef;

    fn ntp(unix_us: i64) -> u64 {
        let s = (unix_us.div_euclid(1_000_000) + NTP_UNIX_OFFSET_S) as u64 & 0xffff_ffff;
        let frac = ((unix_us.rem_euclid(1_000_000) as u64) << 32).div_ceil(1_000_000);
        (s << 32) | frac
    }

    fn base_us() -> i64 {
        NaiveDate::from_ymd_opt(2026, 10, 26)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_micros()
    }

    fn reply(li: u8, stratum: u8, t2: u64, t3: u64) -> [u8; 48] {
        let mut p = [0u8; 48];
        p[0] = (li << 6) | (4 << 3) | 4;
        p[1] = stratum;
        p[4..8].copy_from_slice(&0x0000_0290u32.to_be_bytes()); // ~10 ms delay
        p[8..12].copy_from_slice(&0x0000_0083u32.to_be_bytes()); // ~2 ms disp
        p[12..16].copy_from_slice(b"GPS\0");
        p[24..32].copy_from_slice(&NONCE.to_be_bytes());
        p[32..40].copy_from_slice(&t2.to_be_bytes());
        p[40..48].copy_from_slice(&t3.to_be_bytes());
        p
    }

    fn pending() -> Option<SntpRequest> {
        Some(SntpRequest {
            transmit: NONCE,
            sent_uptime_us: 1_000_000,
        })
    }

    #[test]
    fn matched_reply_pairs_server_midpoint_with_client_midpoint() {
        let mut req = pending();
        let p = reply(0, 2, ntp(base_us()), ntp(base_us() + 4_000));
        let SntpOutcome::Observation(o) = screen(&mut req, &p, 1_040_000, 500) else {
            panic!("observation");
        };
        assert_eq!(req, None, "consumed");
        assert_eq!(o.midpoint_uptime_us, 1_020_000);
        assert_eq!(
            o.utc_at_midpoint.and_utc().timestamp_micros(),
            base_us() + 2_000
        );
        // (40−4)/2 ms + ceil(10.01/2) ms + 1.999 ms + 2 µs + 500 µs, rounded up.
        assert_eq!(o.uncertainty_us, 18_000 + 5_005 + 1_999 + 2 + 500);
        // A duplicate cannot match a consumed request.
        assert_eq!(
            screen(&mut req, &p, 1_041_000, 0),
            SntpOutcome::Ignored(Ignored::NoPendingRequest)
        );
    }

    #[test]
    fn unmatched_and_malformed_keep_request_pending() {
        let mut req = pending();
        let mut p = reply(0, 2, ntp(base_us()), ntp(base_us()));
        assert_eq!(
            screen(&mut req, &p[..47], 1_001_000, 0),
            SntpOutcome::Ignored(Ignored::Malformed)
        );
        p[31] ^= 1;
        assert_eq!(
            screen(&mut req, &p, 1_001_000, 0),
            SntpOutcome::Ignored(Ignored::Unmatched)
        );
        assert_eq!(req, pending());
        let mut zero = Some(SntpRequest {
            transmit: 0,
            sent_uptime_us: 0,
        });
        let mut z = reply(0, 2, 1, 1);
        z[24..32].fill(0);
        assert_eq!(
            screen(&mut zero, &z, 1, 0),
            SntpOutcome::Ignored(Ignored::Unmatched)
        );
    }

    #[test]
    fn status_fields_map_to_kod_unsynchronized_and_leap_revocation() {
        let (t2, t3) = (ntp(base_us()), ntp(base_us()));
        let at = 1_010_000;
        assert_eq!(
            screen(&mut pending(), &reply(0, 0, t2, t3), at, 0),
            SntpOutcome::KissOfDeath(*b"GPS\0")
        );
        assert_eq!(
            screen(&mut pending(), &reply(3, 2, t2, t3), at, 0),
            SntpOutcome::Unsynchronized
        );
        assert_eq!(
            screen(&mut pending(), &reply(0, 16, t2, t3), at, 0),
            SntpOutcome::Unsynchronized
        );
        assert_eq!(
            screen(&mut pending(), &reply(1, 2, t2, t3), at, 0),
            SntpOutcome::LeapPending
        );
        assert_eq!(
            screen(&mut pending(), &reply(2, 2, t2, t3), at, 0),
            SntpOutcome::LeapPending
        );
        let mut v3 = reply(0, 2, t2, t3);
        v3[0] = (3 << 3) | 4;
        assert_eq!(
            screen(&mut pending(), &v3, at, 0),
            SntpOutcome::Ignored(Ignored::BadVersionOrMode)
        );
        let mut broadcast = reply(0, 2, t2, t3);
        broadcast[0] = (4 << 3) | 5;
        assert_eq!(
            screen(&mut pending(), &broadcast, at, 0),
            SntpOutcome::Ignored(Ignored::BadVersionOrMode)
        );
    }

    #[test]
    fn impossible_timing_root_fields_and_ambiguous_era_are_ignored() {
        let at = 1_010_000;
        let ig = |p: &[u8], at| match screen(&mut pending(), p, at, 0) {
            SntpOutcome::Ignored(i) => i,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            ig(&reply(0, 2, 0, ntp(base_us())), at),
            Ignored::ZeroTimestamp
        );
        assert_eq!(
            ig(&reply(0, 2, ntp(base_us() + 1), ntp(base_us())), at),
            Ignored::ImpossibleTiming
        );
        // Turnaround (20 ms) longer than measured RTT (10 ms).
        assert_eq!(
            ig(&reply(0, 2, ntp(base_us()), ntp(base_us() + 20_000)), at),
            Ignored::ImpossibleTiming
        );
        assert_eq!(
            ig(&reply(0, 2, ntp(base_us()), ntp(base_us())), 999_999),
            Ignored::ImpossibleTiming
        );
        let mut neg = reply(0, 2, ntp(base_us()), ntp(base_us()));
        neg[4] = 0x80;
        assert_eq!(ig(&neg, at), Ignored::RootOutOfRange);
        let mut huge = reply(0, 2, ntp(base_us()), ntp(base_us()));
        huge[8..12].copy_from_slice(&0x0001_0001u32.to_be_bytes()); // > 1 s
        assert_eq!(ig(&huge, at), Ignored::RootOutOfRange);
        // 1960 (era 0) and 2096 (era 1) share NTP seconds; both supported.
        let y1960 = NaiveDate::from_ymd_opt(1960, 6, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let t = ntp(y1960.and_utc().timestamp_micros());
        assert_eq!(ig(&reply(0, 2, t, t), at), Ignored::AmbiguousEra);
    }

    #[test]
    fn bracketing_bound_is_resolution_plus_rounded_up_drift() {
        assert_eq!(bracketing_capture_bound_us(0, 50), Some(1));
        assert_eq!(bracketing_capture_bound_us(40_000, 10), Some(2)); // 0.4 µs up
        assert_eq!(bracketing_capture_bound_us(1_000_000, 25), Some(26));
        assert_eq!(bracketing_capture_bound_us(u64::MAX, u64::MAX), None);
    }

    #[test]
    fn request_echo_round_trips_through_screen() {
        let req = request(NONCE);
        assert_eq!(req[0], 0x23);
        let mut pending = pending();
        let mut p = reply(0, 2, ntp(base_us()), ntp(base_us()));
        p[24..32].copy_from_slice(&req[40..48]); // server copies transmit -> origin
        assert!(matches!(
            screen(&mut pending, &p, 1_010_000, 1),
            SntpOutcome::Observation(_)
        ));
    }

    #[test]
    fn consistency_reports_offset_against_combined_bound() {
        let at = |mid, utc_offset_us: i64, u| ScreenedObservation {
            midpoint_uptime_us: mid,
            utc_at_midpoint: DateTime::from_timestamp_micros(base_us() + utc_offset_us)
                .unwrap()
                .naive_utc(),
            uncertainty_us: u,
        };
        // 64 s later, server says 3 ms more than predicted; bound 20+25+1,280.
        let (diff, bound) =
            consistency(at(0, 0, 20_000), at(64_000_000, 64_003_000, 25_000), 20).unwrap();
        assert_eq!((diff, bound), (3_000, 46_280));
        assert!(consistency(at(5, 0, 1), at(4, 0, 1), 20).is_none());
    }

    #[test]
    fn post_2036_era_unfolds_uniquely() {
        let y2040 = NaiveDate::from_ymd_opt(2040, 3, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let us = y2040.and_utc().timestamp_micros();
        let SntpOutcome::Observation(o) =
            screen(&mut pending(), &reply(0, 1, ntp(us), ntp(us)), 1_000_000, 0)
        else {
            panic!("observation");
        };
        assert_eq!(o.utc_at_midpoint, y2040);
    }
}
