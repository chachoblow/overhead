# Overhead — Handoff

_Last updated: 2026-10-08. Next: orbital pass integration and catalogue reporting._

## State
M1/hardening and local catalogue ingestion are complete. The allocation-free
synthetic pass-search kernel is implemented and tested; orbital integration and
catalogue pass reporting remain pending. Render is a stub, sim static, firmware
absent. Scope: [PLAN](PLAN.md).

## This session
Accepted [0015](decisions/0015-pass-search-api.md). Added the public kernel in
[core/src/passes.rs](../core/src/passes.rs), exported from core/src/lib.rs.
[core/tests/passes.rs](../core/tests/passes.rs) owns the synthetic test matrix:
boundaries, detection limits, refinement, failures, and shared/local budgets.
Interrupted refinement retains established coarse brackets and partial records.

## Next step
Compose orbital elevation evaluation using state_at() → state.to_ecef() →
observer geometry; add orbital integration tests, then catalogue aggregation
and headless reporting. Resolve overlapping arrival-bracket ordering without
claiming unsupported precision; preserve incomplete/unsearched satellite details.
Evaluate representative orbital detection intervals independently of tolerance.
No detection default or numeric operating allowance is settled.

## Risks / follow-ups
Synthetic tests are not independent orbital pass references. Completed searches
can miss brief events; refinement does not prove unique crossings. Work limits
do not provide background scheduling. Capacity/cadence remain unmeasured.
Confirm hardware for ESP32 benchmarks and minimal Sharp refresh checks; ask
before flashing or changing toolchains. No freshness cutoff or active-catalogue
fallback exists; unknown fetch time stays unknown. Initialization does not assure
propagation. Broaden references to resonant/low-inclination GEO, applicable
half-day resonances, negative times, and non-LEO observer geometry. See 0007 for
the sgp4 epoch-date issue after February 2100; use bound rotation per 0009.

## Verification
Workspace check and default/all-feature tests pass: 108 tests plus 5 doctests.
Standalone core feature checks, strict Clippy, fmt, and rustdoc checks pass.
No dependency/fixture/toolchain changes, simulator launch, or hardware operations.
