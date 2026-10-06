# 0009 — Shared UTC contract and bound, frame-specific positions

Date: 2026-10-06
Status: accepted

## Decision and rationale
Share UTC validation across ingestion, propagation, rotation, and host parsing.
Bind propagated state to its timestamp and label TEME/ECEF position types.
The review found inconsistent validation and API misuse hazards, not an
incorrect CLI calculation. Physical models and gates in 0006–0008 are unchanged.

## Contract
- `validate_utc_time(NaiveDateTime)` interprets UTC, supports 1957–2100 inclusive,
  and accepts ordinary fractional seconds. Reject explicit leap seconds
  (`nanosecond() >= 1_000_000_000`), including fractional leap seconds.
- Elapsed time is signed naive UTC subtraction, with no inserted leap seconds.
  This is neither a freshness policy nor an accuracy guarantee. The upstream
  sgp4 2100 epoch issue remains; rotation's Gregorian fix is separate (0007).
- `TemeState` is constructed only by propagation. All fields are read-only so
  neither timestamp nor position can be replaced. `to_ecef()` uses the bound
  timestamp with no second time argument.
- `TemePosition` / `EcefPosition` wrap `[f64; 3]`, with `from_km()` / `km()` and
  no implicit cross-frame conversion. They label frames, not numeric validity;
  geometry retains validation. Explicit construction supports synthetic inputs.
- Retain `teme_to_ecef(TemePosition, time)` for external/reference inputs; the
  caller owns timestamp correctness. Explicit array relabeling can likewise
  bypass frame protection.

## Consequences
Normal composition is `satellite.state_at(time)?` → `state.to_ecef()?` →
`ecef_to_geodetic(ecef)` / `ecef_to_look_angles(ecef, observer)`. Forward geodetic
conversion returns `EcefPosition` too.

This source-breaking cleanup precedes catalogue consumers. Domain errors wrap
shared time validation; compile-fail doctests protect frame/binding constraints.
No generic units framework, clock abstraction, velocity conversion, validated
geodetic constructor, dependencies, or changes to reports/fixtures are needed.
Default core remains no_std/allocation-free.

Reference: [Chrono leap-second representation](https://docs.rs/chrono/latest/chrono/trait.Timelike.html#tymethod.nanosecond).
