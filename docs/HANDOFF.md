# Overhead — Handoff

_Last updated: 2026-10-08. Next: M2 pass-search API and tests._

## State
M1/hardening and M2's local catalogue slice are complete. Prediction remains
unimplemented; render is a stub, sim static, firmware absent. Scope: [PLAN](PLAN.md).

## This session
Accepted the bounded search baseline in
[0014](decisions/0014-pass-search-baseline.md): regular elevation sampling,
crossing bisection, explicit boundary uncertainty, and preserved partial results.
Linked it from 0012/0013 and updated PLAN. Physical-pass semantics and detection
limits remain in 0012/0013. Prediction supports next-pass information, not entry
into the zoomed radar view.

## Next step
Read 0012–0014, then finalize the allocation-free shared-core API, validation,
and test matrix before coding. Then implement synthetic search tests and the
predictor before orbital integration and headless catalogue reporting.
Detection interval must initially be explicit; no numeric work budget or
detection default is settled.

## Risks / follow-ups
Completed numerical searches can miss brief events. Refinement does not prove
unique crossings. Work limits do not provide background scheduling.
Confirm available hardware for benchmarks and minimal Sharp refresh checks;
ask before flashing or changing toolchains. Capacity and cadence remain unmeasured.
No freshness cutoff or active-catalogue fallback exists; unknown fetch time stays
unknown. Initialization does not assure propagation. Broaden references to
resonant/low-inclination GEO, applicable half-day resonances, negative times,
and a non-LEO observer pipeline. See 0007 for the sgp4 epoch-date issue after
February 2100; use `state.to_ecef()` per 0009.

## Verification
Documentation links and diff whitespace checked. No code changes, builds/tests,
dependencies, toolchain changes, simulator runs, or hardware operations.
