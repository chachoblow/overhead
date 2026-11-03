//! Naked-eye visibility of the selected object (decision 0078): whether it is
//! sunlit against a dark sky right now. A geometric true-state reading, not a
//! brightness, magnitude or weather claim.
use overhead_core::{
    EcefPosition, GeodeticPosition, ecef_to_look_angles, in_earth_shadow,
    sgp4::chrono::NaiveDateTime, sun_ecef,
};

/// The observer's sky counts as dark once the Sun is below this elevation
/// (civil twilight); above it the sky is too bright to see satellites.
pub const DARK_SKY_SUN_ELEVATION_DEG: f64 = -6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Sunlit while the observer's sky is dark: visible to the eye (if above
    /// the horizon and bright enough, which this does not judge).
    Visible,
    /// Dark sky, but the object is inside Earth's shadow.
    Eclipsed,
    /// The Sun is above the dark-sky threshold at the observer.
    Daylight,
}

/// `None` only if the Sun direction or its elevation cannot be computed
/// (unsupported time or observer), never as a fourth state.
pub fn classify(
    satellite: EcefPosition,
    observer: GeodeticPosition,
    time: NaiveDateTime,
) -> Option<Visibility> {
    let sun = sun_ecef(time).ok()?;
    let sun_elevation = ecef_to_look_angles(sun, observer).ok()?.elevation_rad;
    Some(
        if sun_elevation >= DARK_SKY_SUN_ELEVATION_DEG.to_radians() {
            Visibility::Daylight
        } else if in_earth_shadow(satellite, sun) {
            Visibility::Eclipsed
        } else {
            Visibility::Visible
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use overhead_core::sgp4::chrono::NaiveDate;

    fn utc(h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 3, 20)
            .unwrap()
            .and_hms_opt(h, m, 0)
            .unwrap()
    }

    fn site(lon_deg: f64) -> GeodeticPosition {
        GeodeticPosition {
            latitude_rad: 0.0,
            longitude_rad: lon_deg.to_radians(),
            altitude_km: 0.0,
        }
    }

    /// Point `km` above the observer's zenith.
    fn above(observer: GeodeticPosition, km: f64) -> EcefPosition {
        GeodeticPosition {
            altitude_km: km,
            ..observer
        }
        .to_ecef()
        .unwrap()
    }

    #[test]
    fn equinox_day_dusk_and_midnight() {
        // At 12:00 UTC on the equinox the subsolar longitude is ~-1.9°.
        let noon = site(0.0);
        assert_eq!(
            classify(above(noon, 400.0), noon, utc(12, 0)),
            Some(Visibility::Daylight)
        );
        // ~1 h after sunset (Sun ~-15°): a 400 km object overhead is still lit.
        let dusk = site(-1.9 + 105.0);
        assert_eq!(
            classify(above(dusk, 400.0), dusk, utc(12, 0)),
            Some(Visibility::Visible)
        );
        // Local midnight: the same object overhead is deep in shadow, while a
        // far higher one above the shadow cylinder's edge region stays lit.
        let midnight = site(-1.9 + 180.0);
        assert_eq!(
            classify(above(midnight, 400.0), midnight, utc(12, 0)),
            Some(Visibility::Eclipsed)
        );
        // Sun just above -6°: still daylight at the observer.
        let twilight = site(-1.9 + 95.0);
        assert_eq!(
            classify(above(twilight, 400.0), twilight, utc(12, 0)),
            Some(Visibility::Daylight)
        );
    }

    #[test]
    fn unsupported_time_is_unavailable_not_a_state() {
        let late = NaiveDate::from_ymd_opt(2101, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let s = site(0.0);
        assert_eq!(classify(above(s, 400.0), s, late), None);
    }
}
