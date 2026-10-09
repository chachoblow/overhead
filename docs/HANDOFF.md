# Overhead — Handoff

_Last updated: 2026-10-08. Next: catalogue pass aggregation and reporting._

## State
M1/hardening and local catalogue ingestion are complete. The allocation-free
pass-search kernel now composes with orbital elevation; catalogue pass reporting
remains pending. Render is a stub, sim static, firmware absent. Scope: [PLAN](PLAN.md).

## This session
Added `search_satellite` and typed orbital evaluation errors in
[core/src/passes.rs](../core/src/passes.rs), preserving [0015](decisions/0015-pass-search-api.md).
[core/tests/orbital_passes.rs](../core/tests/orbital_passes.rs) covers existing
Skyfield elevation samples, ISS/deep-space brackets against same-model fine scans,
negative times, window boundaries, failures, and local/shared budget interruptions.
No new product policy or detection/allowance default was selected.

## Next step
Add catalogue aggregation and headless reporting using `search_satellite`.
Resolve overlapping arrival-bracket ordering without claiming unsupported
precision; preserve incomplete/unsearched satellite details. Then evaluate
representative orbital detection intervals independently of crossing tolerance
and measure operating costs. No detection default or numeric allowance is settled.

## Risks / follow-ups
Fine scans check integration, not independent pass-time accuracy or detection
coverage. Completed searches can miss brief events; refinement does not prove
unique crossings. Work limits do not provide scheduling. Capacity/cadence remain
unmeasured. Confirm hardware for ESP32 benchmarks and minimal Sharp refresh
checks; ask before flashing or changing toolchains. No freshness cutoff or
active-catalogue fallback exists; unknown fetch time stays unknown. Initialization
does not assure propagation. Broaden independent references to resonant/GEO,
negative times, and non-LEO observer geometry. See 0007 for the sgp4 epoch-date
issue after February 2100; use bound rotation per 0009.

## Verification
Workspace check and default/all-feature tests pass: 118 tests plus 6 doctests.
Standalone core feature checks, strict Clippy, fmt, and rustdoc checks pass.
No dependency/fixture/toolchain changes, simulator launch, or hardware operations.
