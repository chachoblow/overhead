//! Host-testable rotary-encoder detent decoder; no GPIO, PCNT or app ownership.
//! The caller reads the PCNT unit's raw quadrature count whenever convenient
//! (the hardware keeps counting through render/SPI) and gets back whole
//! detents moved since the last read. Measured on the board (knob-test,
//! 2026-11-03): 4 counts per detent, rest at a multiple of 4, clockwise negative.

/// Counts per mechanical detent for the measured encoder.
pub const COUNTS_PER_DETENT: i32 = 4;
/// PCNT high/low limit the target configures (±LIMIT). On reaching either the
/// unit resets to 0, so raw values are positions modulo LIMIT.
pub const LIMIT: i16 = 20_000;

#[derive(Clone, Copy, Debug)]
pub struct Detents {
    last_raw: i16,
    position: i64,
    detent: i64,
    /// +1 when clockwise should yield positive detents (clockwise counts negative).
    sign: i64,
}

impl Detents {
    /// Start from the counter's current raw value, treated as a rest position.
    /// `clockwise_positive` maps clockwise (negative raw counts) to positive
    /// detents, i.e. zoom in.
    pub const fn new(raw: i16, clockwise_positive: bool) -> Self {
        Self {
            last_raw: raw,
            position: 0,
            detent: 0,
            sign: if clockwise_positive { -1 } else { 1 },
        }
    }

    /// Detents moved since the previous call. Assumes fewer than LIMIT/2
    /// counts between reads (≈2,500 detents), far beyond any hand turn.
    /// Positions round to the nearest detent, so jitter of ±1 count around a
    /// rest position never emits a step.
    pub fn update(&mut self, raw: i16) -> i32 {
        let limit = i64::from(LIMIT);
        let mut delta = i64::from(raw) - i64::from(self.last_raw);
        if delta > limit / 2 {
            delta -= limit;
        } else if delta < -limit / 2 {
            delta += limit;
        }
        self.last_raw = raw;
        self.position += delta * self.sign;
        let per = i64::from(COUNTS_PER_DETENT);
        let detent = (self.position + per / 2).div_euclid(per);
        let moved = detent - self.detent;
        self.detent = detent;
        moved as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_counts_make_one_detent_with_clockwise_positive() {
        let mut d = Detents::new(0, true);
        let mut total = 0;
        for raw in [-1, -2, -3, -4] {
            total += d.update(raw);
        }
        assert_eq!(total, 1);
        for raw in [-3, -2, -1, 0] {
            total += d.update(raw);
        }
        assert_eq!(total, 0);
    }

    #[test]
    fn ten_clicks_each_way_like_the_board_capture() {
        let mut d = Detents::new(0, true);
        assert_eq!(d.update(-40), 10);
        assert_eq!(d.update(0), -10);
        assert_eq!(d.update(-44), 11);
    }

    #[test]
    fn jitter_at_rest_emits_nothing() {
        let mut d = Detents::new(8, false);
        for raw in [9, 8, 7, 8, 9, 8] {
            assert_eq!(d.update(raw), 0);
        }
    }

    #[test]
    fn counter_reset_at_either_limit_is_unwrapped() {
        let mut d = Detents::new(LIMIT - 2, false);
        // +2 reaches LIMIT and resets to 0, then +6 more.
        assert_eq!(d.update(6), 2);
        let mut d = Detents::new(-(LIMIT - 4), false);
        assert_eq!(d.update(-4), -2);
    }

    #[test]
    fn large_gap_between_reads_keeps_every_detent() {
        let mut d = Detents::new(0, true);
        assert_eq!(d.update(-4_000), 1_000);
    }
}
