# 0009 — Shared UTC contract and timestamp-bound, frame-specific positions

Date: 2026-10-06
Status: accepted

## Decision
Use one UTC validator across ingestion, propagation, coordinate rotation, and
host input parsing. Bind propagated TEME state to its absolute timestamp and
use distinct TEME/ECEF position types at physical API boundaries.

This updates the caller-facing composition described in 0007–0008; the physical
models, units, numerical algorithms, and reference tolerances of 0006–0008
remain unchanged.

## Why
The walkthrough found inconsistent time validation and two API misuse hazards,
not demonstrated incorrect CLI calculations: propagation accepted unsupported
years and explicit leap seconds, ingestion did not reject leap-second epochs,
and callers could mix coordinate frames or supply the wrong rotation time.

- `validate_utc_time(NaiveDateTime)` interprets its input as UTC and supports
  years 1957..=2100 inclusive. Reject Chrono explicit leap-second values
  (`nanosecond() >= 1_000_000_000`), including fractional leap seconds.
  Ordinary fractional seconds are accepted. Elapsed time remains the signed
  naive UTC difference; no leap seconds are inserted between timestamps.
- This is an era/representation contract, not a freshness policy or an accuracy
  guarantee far from the element epoch. SGP4 divergence remains an error.
  The known upstream sgp4 2.4 epoch calendar issue after February 2100 is not
  fixed or hidden by accepting that year. Rotation retains its Gregorian fix.
- `TemeState` has private fields and read-only accessors: `position()`,
  `velocity()`, `minutes_since_epoch()`, and `datetime()`. Only propagation
  constructs it. `to_ecef()` uses its bound timestamp without a second time
  argument. Making only the timestamp private would still permit replacing
  the position and breaking the binding, so all fields are read-only.
- `TemePosition` and `EcefPosition` wrap `[f64; 3]`, expose `from_km()` and
  `km()`, and have no implicit cross-frame conversions. They label frames,
  not numeric validity; existing conversion/geometry checks still reject
  non-finite values. Explicit construction permits fixtures and synthetic
  geometry without an SGP4 dependency in those callers.
- Retain `teme_to_ecef(TemePosition, time)` for synthetic/reference cases.
  It is deliberately a low-level escape hatch whose caller must supply the
  correct timestamp. Removing it would make independent rotation tests and
  external TEME inputs unnecessarily awkward.
- Do not add a generic units framework, clock abstraction, velocity-frame
  conversion, or validated geodetic constructor. These are not required to
  address the identified hazards.

## Consequences
- Normal composition is `Satellite::state_at(time)` → `state.to_ecef()` →
  `ecef_to_geodetic(ecef)` / `ecef_to_look_angles(ecef, observer)`.
  Forward geodetic conversion also returns `EcefPosition`.
- This is a source-breaking API cleanup before catalogue consumers exist.
  State field reads become accessor calls; reporting extracts arrays with
  `.km()`. Existing CLI output format and reference tolerances are unchanged.
- The shared validator returns `TimeError::UnsupportedYear(year)` or
  `TimeError::LeapSecond`. Ingestion preserves `ImplausibleEpoch(year)` and
  adds `LeapSecondEpoch`; propagation adds `UnsupportedTime(TimeError)`;
  rotation preserves `CoordinateError::UnsupportedTime`. Domain-specific
  diagnostics do not duplicate validation rules.
- Boundary, signed/subsecond elapsed-time, and timestamp-binding tests cover
  the contract. Compile-fail doctests protect frame separation and prevent
  external retiming. Existing independent reference tests exercise the normal
  timestamp-bound conversion. No fixtures or tolerance gates were changed.
- Default core stays no_std/allocation-free. No dependencies, physical models,
  catalogue policy, or hardware assumptions are added.

Reference for the leap-second representation:
https://docs.rs/chrono/latest/chrono/trait.Timelike.html#tymethod.nanosecond
