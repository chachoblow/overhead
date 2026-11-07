//! 0080 per-server SNTP poll policy: Kiss-o'-Death backs off one server for
//! a bounded time instead of stopping all queries until reboot. Host-tested
//! and clock-agnostic: times are monotonic seconds supplied by the caller.
//!
//! RFC 4330 §8: on `DENY`/`RSTR` the client must stop sending to that
//! server; on `RATE` it must reduce its polling rate to it. Other kiss codes
//! are treated as `RATE` (backing off is the safe reading).

/// The ordinary poll interval.
pub const POLL_S: u64 = 64;
/// Longest interval `RATE` doubling reaches.
pub const RATE_MAX_S: u64 = 3600;
/// How long a `DENY`/`RSTR` server is left alone.
pub const DENY_BLOCK_S: u64 = 24 * 3600;

/// What a kiss code asks of the client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kiss {
    /// Poll this server less often.
    Rate,
    /// Stop asking this server (bounded here by the block time).
    Deny,
}

impl Kiss {
    pub fn classify(code: [u8; 4]) -> Self {
        match &code {
            b"DENY" | b"RSTR" => Kiss::Deny,
            _ => Kiss::Rate,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Server {
    interval_s: u64,
    next_s: u64,
}

/// One entry per server, in fallback order. Every server starts due.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Policy<const N: usize> {
    servers: [Server; N],
    block_s: u64,
}

impl<const N: usize> Policy<N> {
    /// `block_s` is [`DENY_BLOCK_S`] on the device; the fault build may
    /// shorten it so a block ends within one capture.
    pub const fn new(block_s: u64) -> Self {
        Self {
            servers: [Server {
                interval_s: POLL_S,
                next_s: 0,
            }; N],
            block_s,
        }
    }

    /// Whether server `i` may be asked at `now_s`.
    pub const fn due(&self, i: usize, now_s: u64) -> bool {
        now_s >= self.servers[i].next_s
    }

    pub const fn interval_s(&self, i: usize) -> u64 {
        self.servers[i].interval_s
    }

    /// Seconds from `now_s` until server `i` may be asked again.
    pub const fn next_in_s(&self, i: usize, now_s: u64) -> u64 {
        self.servers[i].next_s.saturating_sub(now_s)
    }

    /// Server `i`, asked at `now_s`, sent no matched reply: ask it again
    /// after its current interval.
    pub fn no_reply(&mut self, i: usize, now_s: u64) {
        let s = &mut self.servers[i];
        s.next_s = now_s.saturating_add(s.interval_s);
    }

    /// Server `i`, asked at `now_s`, sent a matched non-KoD reply: halve a
    /// backed-off interval towards [`POLL_S`].
    pub fn replied(&mut self, i: usize, now_s: u64) {
        let s = &mut self.servers[i];
        s.interval_s = (s.interval_s / 2).max(POLL_S);
        s.next_s = now_s.saturating_add(s.interval_s);
    }

    /// Server `i`, asked at `now_s`, sent kiss `code`. Returns how it was
    /// classified and the seconds until the server is asked again.
    pub fn kiss(&mut self, i: usize, now_s: u64, code: [u8; 4]) -> (Kiss, u64) {
        let kiss = Kiss::classify(code);
        let s = &mut self.servers[i];
        let wait = match kiss {
            Kiss::Rate => {
                s.interval_s = s.interval_s.saturating_mul(2).min(RATE_MAX_S);
                s.interval_s
            }
            // The interval is kept: after the block the server is asked at
            // whatever rate it last accepted.
            Kiss::Deny => self.block_s,
        };
        s.next_s = now_s.saturating_add(wait);
        (kiss, wait)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_classify() {
        assert_eq!(Kiss::classify(*b"DENY"), Kiss::Deny);
        assert_eq!(Kiss::classify(*b"RSTR"), Kiss::Deny);
        assert_eq!(Kiss::classify(*b"RATE"), Kiss::Rate);
        assert_eq!(Kiss::classify(*b"INIT"), Kiss::Rate);
        assert_eq!(Kiss::classify([0; 4]), Kiss::Rate);
    }

    #[test]
    fn all_servers_start_due_at_the_poll_interval() {
        let p = Policy::<2>::new(DENY_BLOCK_S);
        assert!(p.due(0, 0) && p.due(1, 0));
        assert_eq!(p.interval_s(0), POLL_S);
    }

    #[test]
    fn ordinary_polling_is_unchanged() {
        let mut p = Policy::<2>::new(DENY_BLOCK_S);
        p.replied(0, 1000);
        assert!(!p.due(0, 1063));
        assert!(p.due(0, 1064));
        p.no_reply(1, 1000);
        assert_eq!(p.next_in_s(1, 1000), POLL_S);
        assert_eq!(p.interval_s(0), POLL_S);
    }

    #[test]
    fn rate_doubles_to_the_cap_and_replies_halve_it() {
        let mut p = Policy::<2>::new(DENY_BLOCK_S);
        let mut now = 0;
        for expected in [128, 256, 512, 1024, 2048, 3600, 3600] {
            assert!(p.due(0, now));
            assert_eq!(p.kiss(0, now, *b"RATE"), (Kiss::Rate, expected));
            assert!(!p.due(0, now + expected - 1));
            now += expected;
        }
        // The other server is unaffected.
        assert!(p.due(1, 0));
        for expected in [1800, 900, 450, 225, 112, 64, 64] {
            p.replied(0, now);
            assert_eq!(p.interval_s(0), expected);
            now += expected;
        }
    }

    #[test]
    fn deny_blocks_one_server_for_the_block_then_recovers() {
        let mut p = Policy::<2>::new(DENY_BLOCK_S);
        assert_eq!(p.kiss(1, 500, *b"DENY"), (Kiss::Deny, DENY_BLOCK_S));
        assert!(!p.due(1, 500 + DENY_BLOCK_S - 1));
        assert!(p.due(1, 500 + DENY_BLOCK_S));
        assert!(p.due(0, 500));
        // Interval kept through the block.
        assert_eq!(p.interval_s(1), POLL_S);
        let mut short = Policy::<2>::new(40 * 60);
        assert_eq!(short.kiss(0, 0, *b"RSTR"), (Kiss::Deny, 2400));
    }

    #[test]
    fn rate_backoff_survives_a_deny() {
        let mut p = Policy::<1>::new(100);
        p.kiss(0, 0, *b"RATE");
        p.kiss(0, 128, *b"DENY");
        assert_eq!(p.interval_s(0), 128);
        p.replied(0, 228);
        assert_eq!(p.interval_s(0), POLL_S);
    }

    #[test]
    fn far_future_times_saturate() {
        let mut p = Policy::<1>::new(u64::MAX);
        p.kiss(0, u64::MAX - 1, *b"DENY");
        assert!(!p.due(0, u64::MAX - 1));
        p.no_reply(0, u64::MAX);
        assert_eq!(p.next_in_s(0, u64::MAX), 0);
    }
}
