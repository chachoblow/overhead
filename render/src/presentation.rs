//! Explicit-time, bounded selected-data presentation. Never modifies app truth.
use core::time::Duration;
use overhead_app::RadarApp;

use crate::glyphs::Params;

const DURATION: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy)]
struct Motion {
    from: f64,
    target: f64,
    start: Duration,
    end: Duration,
}

impl Motion {
    fn at(self, now: Duration) -> f64 {
        if now >= self.end {
            return self.target;
        }
        let elapsed = now.saturating_sub(self.start).as_secs_f64();
        let span = self.end.saturating_sub(self.start).as_secs_f64();
        let t = if span == 0.0 {
            1.0
        } else {
            (elapsed / span).clamp(0.0, 1.0)
        };
        let q = t * t * (3.0 - 2.0 * t);
        self.from + (self.target - self.from) * q
    }
}

/// One selected-ID history and four independent drawing parameters. Caller
/// supplies monotonic presentation time; decreasing time is clamped. A new
/// instance (including headless snapshots) starts at the static true state.
#[derive(Debug, Default)]
pub struct Presentation {
    id: Option<u64>,
    last_time: Option<Duration>,
    displayed: [Option<f64>; 4],
    motions: [Option<Motion>; 4],
}

impl Presentation {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn values(&mut self, app: &RadarApp, time: Duration) -> Params {
        self.advance(app.selected_id(), Params::selected(app), time)
    }

    fn advance(&mut self, next_id: Option<u64>, Params(target): Params, time: Duration) -> Params {
        let now = self.last_time.map_or(time, |last| time.max(last));
        self.last_time = Some(now);
        for (value, motion) in self.displayed.iter_mut().zip(&mut self.motions) {
            if let Some(m) = *motion {
                *value = Some(m.at(now));
                if now >= m.end {
                    *motion = None;
                }
            }
        }
        if next_id.is_none() {
            self.id = None;
            self.displayed = [None; 4];
            self.motions = [None; 4];
        } else if self.id != next_id {
            // A first/recovered selection has no previous endpoint. A -> B -> C
            // instead starts at the currently displayed (possibly moving) shape.
            for (i, &to) in target.iter().enumerate() {
                self.motions[i] = match (self.id, self.displayed[i], to) {
                    (Some(_), Some(from), Some(to)) if from != to => Some(Motion {
                        from,
                        target: to,
                        start: now,
                        end: now.saturating_add(DURATION),
                    }),
                    _ => None,
                };
                self.displayed[i] = self.motions[i].map_or(to, |m| Some(m.from));
            }
            self.id = next_id;
        } else {
            for (i, &to) in target.iter().enumerate() {
                self.motions[i] = match (self.motions[i], self.displayed[i], to) {
                    (Some(m), Some(from), Some(to)) if m.target != to => Some(Motion {
                        from,
                        target: to,
                        start: now,
                        end: m.end, // New frame, same ID: keep original deadline.
                    }),
                    (Some(m), Some(_), Some(_)) => Some(m),
                    _ => None, // Invalid/absent field or stable selected ID.
                };
                if self.motions[i].is_none() {
                    self.displayed[i] = to;
                }
            }
        }
        Params(self.displayed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }
    fn p(values: [Option<f64>; 4]) -> Params {
        Params(values)
    }
    const A: [Option<f64>; 4] = [Some(0.0), Some(9.0), Some(0.0), Some(2.5)];
    const B: [Option<f64>; 4] = [Some(1.0), Some(2.0), Some(12.0), Some(1.0)];
    const C: [Option<f64>; 4] = [Some(0.5), Some(8.0), Some(6.0), Some(2.0)];

    #[test]
    fn first_selection_and_stable_frames_are_immediate() {
        let mut state = Presentation::new();
        assert_eq!(state.advance(None, p([None; 4]), ms(0)), p([None; 4]));
        assert_eq!(state.advance(Some(1), p(A), ms(10)), p(A));
        assert_eq!(state.advance(Some(1), p(B), ms(20)), p(B));
        assert_eq!(state.advance(Some(1), p(C), ms(21)), p(C));
    }

    #[test]
    fn quarter_half_deadline_and_backwards_time_use_smoothstep() {
        let mut state = Presentation::new();
        state.advance(Some(1), p(A), ms(0));
        assert_eq!(state.advance(Some(2), p(B), ms(100)), p(A));
        assert!((state.advance(Some(2), p(B), ms(125)).0[0].unwrap() - 0.028).abs() < 1e-12);
        let half = state.advance(Some(2), p(B), ms(225));
        assert_eq!(half.0[0], Some(0.5));
        assert_eq!(half.0[1], Some(5.5));
        assert_eq!(state.advance(Some(2), p(B), ms(100)), half);
        assert_eq!(state.advance(Some(2), p(B), ms(350)), p(B));
        assert_eq!(state.advance(Some(2), p(B), ms(500)), p(B));
    }

    #[test]
    fn rapid_reselection_starts_at_visible_shape_without_queueing() {
        let mut state = Presentation::new();
        state.advance(Some(1), p(A), ms(0));
        state.advance(Some(2), p(B), ms(0));
        let halfway = state.advance(Some(2), p(B), ms(125));
        assert_eq!(state.advance(Some(3), p(C), ms(125)), halfway);
        let mid = state.advance(Some(3), p(C), ms(250));
        assert_eq!(mid.0[0], Some(0.5));
        assert_eq!(mid.0[1], Some(6.75));
        assert_eq!(state.advance(Some(3), p(C), ms(375)), p(C));
    }

    #[test]
    fn same_id_frame_retargets_without_extending_deadline() {
        let mut state = Presentation::new();
        state.advance(Some(1), p(A), ms(0));
        state.advance(Some(2), p(B), ms(0));
        let old = state.advance(Some(2), p(B), ms(125));
        assert_eq!(state.advance(Some(2), p(C), ms(125)), old);
        assert_eq!(state.advance(Some(2), p(C), ms(187)).0[0], Some(0.5)); // already at target
        assert_eq!(state.advance(Some(2), p(C), ms(250)), p(C));
        assert_eq!(state.advance(Some(2), p(B), ms(251)), p(B)); // stable frame
    }

    #[test]
    fn field_invalidity_and_selection_loss_discard_only_applicable_history() {
        let mut state = Presentation::new();
        state.advance(Some(1), p(A), ms(0));
        state.advance(Some(2), p(B), ms(0));
        let mut invalid = B;
        invalid[1] = None;
        let frame = state.advance(Some(2), p(invalid), ms(100));
        assert_eq!(frame.0[1], None);
        assert!(frame.0[0].unwrap() > 0.0 && frame.0[0].unwrap() < 1.0);
        invalid[1] = Some(4.0);
        assert_eq!(state.advance(Some(2), p(invalid), ms(101)).0[1], Some(4.0));
        assert_eq!(state.advance(None, p([None; 4]), ms(102)), p([None; 4]));
        assert_eq!(state.advance(Some(3), p(C), ms(103)), p(C));
    }
}
