//! Clockless label presentation policy, independent of selection and tracking.
use crate::projection::{MAX_FOV_DEG, MIN_FOV_DEG, View};

pub const DEFAULT_LABEL_SHOW_DEG: f64 = 60.0;
pub const DEFAULT_LABEL_HIDE_DEG: f64 = 70.0;

/// Wide-view label zoom thresholds. Smaller vertical FOV means zoomed in.
/// The open interval between thresholds retains the previous visibility.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabelZoom {
    show_half_span: f64,
    hide_half_span: f64,
}

impl LabelZoom {
    /// Require finite thresholds within the lens range, with show < hide.
    pub fn new(show_deg: f64, hide_deg: f64) -> Result<Self, &'static str> {
        if !show_deg.is_finite()
            || !hide_deg.is_finite()
            || !(MIN_FOV_DEG..=MAX_FOV_DEG).contains(&show_deg)
            || !(MIN_FOV_DEG..=MAX_FOV_DEG).contains(&hide_deg)
            || show_deg >= hide_deg
        {
            return Err("label thresholds must satisfy 0.1 <= show < hide <= 100 degrees");
        }
        // Compare in the lens's native coordinates, avoiding tan/atan roundoff
        // at exact inclusive thresholds, including configurable ones.
        Ok(Self {
            show_half_span: View::new(show_deg)?.vertical_half_span(),
            hide_half_span: View::new(hide_deg)?.vertical_half_span(),
        })
    }

    /// At launch there is no zoom history: show below the hide threshold.
    pub(crate) fn initially_visible(self, view: View) -> bool {
        view.vertical_half_span() < self.hide_half_span
    }

    pub(crate) fn after_zoom(self, view: View, was_visible: bool) -> bool {
        let span = view.vertical_half_span();
        if span >= self.hide_half_span {
            false
        } else if span <= self.show_half_span {
            true
        } else {
            was_visible
        }
    }
}

impl Default for LabelZoom {
    fn default() -> Self {
        Self::new(DEFAULT_LABEL_SHOW_DEG, DEFAULT_LABEL_HIDE_DEG).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inclusive_thresholds_and_dead_band_keep_zoom_history() {
        let policy = LabelZoom::default();
        let mut visible = policy.initially_visible(View::new(20.0).unwrap());
        for (fov, expected) in [
            (60.0, true),
            (65.0, true),
            (69.999, true),
            (70.0, false),
            (100.0, false),
            (65.0, false),
            (69.0, false),
            (60.001, false),
            (60.0, true),
            (65.0, true),
            (0.1, true),
        ] {
            visible = policy.after_zoom(View::new(fov).unwrap(), visible);
            assert_eq!(visible, expected, "fov={fov}");
        }
    }

    #[test]
    fn startup_and_configurable_boundaries_are_deterministic() {
        for (show, hide) in [(60.0, 70.0), (17.3, 42.7), (0.1, 100.0)] {
            let policy = LabelZoom::new(show, hide).unwrap();
            assert!(policy.initially_visible(View::new(show).unwrap()));
            assert!(policy.initially_visible(View::new((show + hide) / 2.0).unwrap()));
            assert!(!policy.initially_visible(View::new(hide).unwrap()));
            assert!(policy.after_zoom(View::new(show).unwrap(), false));
            assert!(!policy.after_zoom(View::new(hide).unwrap(), true));
        }
    }

    #[test]
    fn invalid_thresholds_are_rejected() {
        for (show, hide) in [
            (f64::NAN, 70.0),
            (60.0, f64::NAN),
            (f64::INFINITY, 70.0),
            (60.0, f64::INFINITY),
            (0.0, 70.0),
            (60.0, 100.1),
            (70.0, 70.0),
            (80.0, 70.0),
        ] {
            assert!(LabelZoom::new(show, hide).is_err());
        }
    }
}
