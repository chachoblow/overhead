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

/// One asked server's answer within a poll.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    NoReply,
    Kiss([u8; 4]),
    /// A matched reply that gave no in-budget observation: unsynchronized,
    /// leap pending, screened out or over the budget (0090).
    Unusable,
    /// A matched reply with an in-budget observation.
    Usable,
}

/// What the poll does after an [`Answer`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Ask the next due server, if any.
    NextServer,
    /// Kissed: the next due server, after logging how this one backs off.
    Kissed {
        kiss: Kiss,
        code: [u8; 4],
        wait_s: u64,
    },
    /// The poll has its observation.
    Done,
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

    /// Records server `i`'s answer at `now_s` (0080) and says whether the
    /// poll goes on. Only a usable reply ends it; no reply, a KoD or an
    /// unusable matched reply falls through to the next due server (0090).
    pub fn answer(&mut self, i: usize, now_s: u64, answer: Answer) -> Step {
        match answer {
            Answer::NoReply => {
                self.no_reply(i, now_s);
                Step::NextServer
            }
            Answer::Kiss(code) => {
                let (kiss, wait_s) = self.kiss(i, now_s, code);
                Step::Kissed { kiss, code, wait_s }
            }
            Answer::Unusable => {
                self.replied(i, now_s);
                Step::NextServer
            }
            Answer::Usable => {
                self.replied(i, now_s);
                Step::Done
            }
        }
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

    /// One poll as the device runs it: due servers in order until `Done`.
    /// Returns which servers were asked.
    fn poll(p: &mut Policy<2>, now: u64, answers: [Answer; 2]) -> [bool; 2] {
        let mut asked = [false; 2];
        for (i, a) in answers.into_iter().enumerate() {
            if !p.due(i, now) {
                continue;
            }
            asked[i] = true;
            if p.answer(i, now, a) == Step::Done {
                break;
            }
        }
        asked
    }

    #[test]
    fn only_a_usable_reply_ends_the_poll() {
        use Answer::*;
        for first in [NoReply, Kiss(*b"RATE"), Kiss(*b"DENY"), Unusable] {
            let mut p = Policy::<2>::new(DENY_BLOCK_S);
            assert_eq!(poll(&mut p, 0, [first, Usable]), [true, true], "{first:?}");
        }
        let mut p = Policy::<2>::new(DENY_BLOCK_S);
        assert_eq!(poll(&mut p, 0, [Usable, Usable]), [true, false]);
        // Each server keeps its own rate: neither is asked before it is due.
        let mut p = Policy::<2>::new(DENY_BLOCK_S);
        assert_eq!(poll(&mut p, 0, [Unusable, Usable]), [true, true]);
        assert_eq!(poll(&mut p, 30, [Unusable, Usable]), [false, false]);
        assert_eq!(poll(&mut p, POLL_S, [Unusable, Usable]), [true, true]);
    }

    #[test]
    fn an_unusable_reply_counts_as_a_reply_for_backoff() {
        let mut p = Policy::<2>::new(DENY_BLOCK_S);
        p.kiss(0, 0, *b"RATE");
        assert_eq!(p.answer(0, 128, Answer::Unusable), Step::NextServer);
        assert_eq!(p.interval_s(0), POLL_S);
        assert_eq!(
            p.answer(1, 0, Answer::Kiss(*b"RATE")),
            Step::Kissed {
                kiss: Kiss::Rate,
                code: *b"RATE",
                wait_s: 128
            }
        );
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
