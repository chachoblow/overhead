//! Source-screen to trust-gate boundary shared by the host preflight and the
//! opt-in live-UTC radar image. Peer validation and KoD backoff are the socket
//! adapter's responsibility; never accept a packet directly here.
use crate::{
    sntp_screen::SntpOutcome,
    utc_trust::{ObservationOutcome, UtcTrustPreflight},
};
use overhead_app::RadarApp;

/// Apply a screened, peer-matched outcome at processing uptime. Ignored/KoD
/// cannot renew; the caller must also honor any KoD rate/deny instruction.
/// `None` means no new anchor and no explicit revocation (existing holdover
/// remains subject to its original expiry).
pub fn apply(
    gate: &mut UtcTrustPreflight,
    app: &mut RadarApp,
    now_us: u64,
    outcome: SntpOutcome,
) -> Option<ObservationOutcome> {
    match outcome {
        SntpOutcome::Observation(o) => Some(gate.observe(app, now_us, o)),
        SntpOutcome::Unsynchronized => {
            gate.source_unsynchronized(app);
            Some(ObservationOutcome::Revoked)
        }
        SntpOutcome::LeapPending => {
            gate.source_leap_pending(app);
            Some(ObservationOutcome::Revoked)
        }
        SntpOutcome::Ignored(_) | SntpOutcome::KissOfDeath(_) => {
            gate.no_renewal(app, now_us);
            None
        }
    }
}
