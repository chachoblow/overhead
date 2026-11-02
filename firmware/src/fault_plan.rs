//! Build-time fault plan for the opt-in `live-fetch-faults` diagnostic, which
//! shows 0075's failure handling on the board. Host-tested parsing only; the
//! image injects each fault at its real stage (DNS, TCP, TLS) so the existing
//! classification and schedule run unchanged.
//!
//! Nothing here alters 0075: the plan only chooses which attempt fails how,
//! and the freshness overrides shorten `OLD DATA`/expiry so they occur within
//! one capture. None of it exists in the ordinary `live-fetch-radar` image.

use crate::element_refresh::Outcome;

/// How one fetch attempt is made to fail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// No injection: the ordinary fetch.
    None,
    /// Resolve a `.invalid` name instead of the host: a real DNS failure.
    Dns,
    /// Connect to TEST-NET-1 (192.0.2.1) instead: a real TCP timeout.
    Tcp,
    /// Cut the connection after the server's first handshake bytes: a
    /// handshake failure with no certificate verdict.
    Handshake,
    /// Verify against a root not in the server's chain: a real
    /// certificate-verification failure, before any request.
    Verify,
}

impl Fault {
    /// The 0075 class this fault must produce.
    pub const fn expected(self) -> Outcome {
        match self {
            Fault::None => Outcome::Success,
            Fault::Dns | Fault::Tcp => Outcome::Transport,
            Fault::Handshake => Outcome::Handshake,
            Fault::Verify => Outcome::Rejected,
        }
    }

    fn parse(word: &str) -> Option<Self> {
        Some(match word {
            "ok" => Fault::None,
            "dns" => Fault::Dns,
            "tcp" => Fault::Tcp,
            "handshake" => Fault::Handshake,
            "verify" => Fault::Verify,
            _ => return None,
        })
    }
}

/// Longest plan accepted; later attempts are never injected.
pub const MAX_STEPS: usize = 16;

/// One fault per attempt, in order; attempts past the end are ordinary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    steps: [Fault; MAX_STEPS],
    len: usize,
}

impl Plan {
    /// Parses `FAULT_PLAN`, e.g. `tcp,ok,dns,handshake,verify`. Absent or
    /// empty means no injection; an unknown word or overlong plan is refused.
    pub fn parse(value: Option<&str>) -> Result<Self, &'static str> {
        let mut plan = Plan {
            steps: [Fault::None; MAX_STEPS],
            len: 0,
        };
        let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
            return Ok(plan);
        };
        for word in value.split(',') {
            if plan.len == MAX_STEPS {
                return Err("FAULT_PLAN has more than 16 steps");
            }
            plan.steps[plan.len] = Fault::parse(word.trim())
                .ok_or("FAULT_PLAN step is not ok/dns/tcp/handshake/verify")?;
            plan.len += 1;
        }
        Ok(plan)
    }

    /// Fault for a 1-based attempt number.
    pub fn for_attempt(&self, attempt: u32) -> Fault {
        match attempt.checked_sub(1).map(|i| i as usize) {
            Some(i) if i < self.len => self.steps[i],
            _ => Fault::None,
        }
    }

    pub fn steps(&self) -> &[Fault] {
        &self.steps[..self.len]
    }
}

/// Parses an optional positive whole number of minutes into seconds.
pub fn minutes(value: Option<&str>, name: &'static str) -> Result<Option<u64>, &'static str> {
    let Some(value) = value else {
        return Ok(None);
    };
    let minutes: u64 = value.trim().parse().map_err(|_| name)?;
    if minutes == 0 {
        return Err(name);
    }
    minutes.checked_mul(60).map(Some).ok_or(name)
}

/// Expiry threshold that makes a catalogue loaded at `fetched_unix_s`, whose
/// newest epoch is `newest_epoch_unix_s`, expire `after_load_s` after the
/// load. The app compares frame UTC with the newest epoch, so the threshold is
/// the epoch's age at load plus the wanted delay.
pub fn expire_after_s(fetched_unix_s: i64, newest_epoch_unix_s: i64, after_load_s: u64) -> i64 {
    let age = fetched_unix_s.saturating_sub(newest_epoch_unix_s).max(0);
    age.saturating_add(i64::try_from(after_load_s).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_or_empty_plan_injects_nothing() {
        for value in [None, Some(""), Some("  ")] {
            let plan = Plan::parse(value).unwrap();
            assert!(plan.steps().is_empty());
            assert_eq!(plan.for_attempt(1), Fault::None);
        }
    }

    #[test]
    fn plan_is_per_attempt_then_ordinary() {
        let plan = Plan::parse(Some("tcp, ok,dns,handshake,verify,ok")).unwrap();
        assert_eq!(
            plan.steps(),
            [
                Fault::Tcp,
                Fault::None,
                Fault::Dns,
                Fault::Handshake,
                Fault::Verify,
                Fault::None
            ]
        );
        assert_eq!(plan.for_attempt(0), Fault::None);
        assert_eq!(plan.for_attempt(1), Fault::Tcp);
        assert_eq!(plan.for_attempt(5), Fault::Verify);
        assert_eq!(plan.for_attempt(7), Fault::None);
        assert_eq!(plan.for_attempt(u32::MAX), Fault::None);
    }

    #[test]
    fn bad_plans_are_refused() {
        assert!(Plan::parse(Some("tcp,http")).is_err());
        assert!(Plan::parse(Some("tcp,,ok")).is_err());
        let long = "ok,".repeat(16) + "ok";
        assert!(Plan::parse(Some(&long)).is_err());
        let full = ["ok"; 16].join(",");
        assert_eq!(Plan::parse(Some(&full)).unwrap().steps().len(), 16);
    }

    #[test]
    fn each_fault_maps_to_its_0075_class() {
        assert_eq!(Fault::None.expected(), Outcome::Success);
        assert_eq!(Fault::Dns.expected(), Outcome::Transport);
        assert_eq!(Fault::Tcp.expected(), Outcome::Transport);
        assert_eq!(Fault::Handshake.expected(), Outcome::Handshake);
        assert_eq!(Fault::Verify.expected(), Outcome::Rejected);
    }

    #[test]
    fn minute_overrides() {
        assert_eq!(minutes(None, "X"), Ok(None));
        assert_eq!(minutes(Some(" 90 "), "X"), Ok(Some(5400)));
        assert_eq!(minutes(Some("0"), "X"), Err("X"));
        assert_eq!(minutes(Some("1.5"), "X"), Err("X"));
        assert_eq!(minutes(Some("99999999999999999999"), "X"), Err("X"));
    }

    #[test]
    fn expiry_is_epoch_age_plus_delay() {
        // Newest epoch 9 h before the fetch, expire 210 min after the load.
        assert_eq!(
            expire_after_s(100_000, 100_000 - 9 * 3600, 210 * 60),
            9 * 3600 + 12_600
        );
        // An epoch after the fetch time (clock skew) counts as age zero.
        assert_eq!(expire_after_s(100, 200, 60), 60);
        assert_eq!(expire_after_s(0, 0, u64::MAX), i64::MAX);
    }
}
