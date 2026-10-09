# Overhead — Handoff

_Last updated: 2026-10-08. Next: M2 pass search strategy, API, and tests._

## State
M1/hardening and M2's local catalogue slice are complete. Physical-pass semantics
are accepted in [0012](decisions/0012-physical-pass-semantics.md), with practical
detection limits clarified by [0013](decisions/0013-pass-detection-resolution.md).
Prediction is not implemented. Render remains a stub, sim static, firmware absent.
M2 prediction and operating budgets remain open in [PLAN](PLAN.md).

## This session
Accepted 0013: bounded search may miss brief threshold excursions/gaps; detected
short passes are not deliberately filtered. Prediction does not gate radar
markers or reverse visual-slowing intent. Linked the clarification from 0012
and PLAN. No algorithm, API, detection default, or work budget was settled.

## Next step
Read 0012/0013, then design the shared-core search, API, and tests before coding.
Fixed-step sampling plus crossing bisection was discussed as a headless baseline,
not accepted as the algorithm. The suggested 10-second detection limit is not a
default. Choose/test detection resolution separately from crossing tolerance;
settle threshold/window boundaries, tangencies, validation, and failure reporting.
Keep physical prediction independent of projection, selection, and rendering.

Confirm available hardware for propagation/prediction benchmarks and minimal
Sharp refresh checks. Ask before flashing or changing toolchains. Capacity,
budgets, scheduling, and search cost remain unmeasured; define over-budget behavior.

## Risks / follow-ups
- Completed numerical searches do not prove absence of arbitrarily short events.
- No freshness cutoff or active-catalogue fallback is implemented; unknown fetch
  time stays unknown. Initialization does not assure propagation.
- Broaden references to resonant/low-inclination GEO, applicable half-day
  resonances, negative times, and a non-LEO observer pipeline.
- sgp4 epoch dates after February 2100: see 0007. Use `state.to_ecef()` per 0009.

## Verification
Documentation links and diff whitespace checked. No builds/tests rerun;
no code, dependency, toolchain, simulator, hardware, or flashing changes.
