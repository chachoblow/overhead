//! UTC input contract shared by ingestion, propagation, rotation, and hosts.
use sgp4::chrono::{Datelike, NaiveDateTime, Timelike};

/// A timestamp outside the engine's supported UTC representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeError {
    /// Calendar year outside 1957..=2100 (inclusive).
    UnsupportedYear(i32),
    /// Chrono's explicit leap-second representation is not supported.
    LeapSecond,
}

impl core::fmt::Display for TimeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedYear(year) => write!(f, "UTC year {year} is outside 1957..=2100"),
            Self::LeapSecond => f.write_str("explicit UTC leap seconds are not supported"),
        }
    }
}

/// Validates a naive datetime interpreted as UTC, never as local time.
///
/// Supports years 1957..=2100 and ordinary fractional seconds. Explicit leap
/// seconds are rejected; elapsed-time arithmetic does not insert leap seconds.
/// This is a representation/era check, not an element freshness or accuracy
/// guarantee. It does not correct upstream SGP4 epoch calendar approximations.
pub fn validate_utc_time(datetime: NaiveDateTime) -> Result<(), TimeError> {
    if !(1957..=2100).contains(&datetime.year()) {
        return Err(TimeError::UnsupportedYear(datetime.year()));
    }
    if datetime.nanosecond() >= 1_000_000_000 {
        return Err(TimeError::LeapSecond);
    }
    Ok(())
}
