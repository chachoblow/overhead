# Overhead — Handoff

_Last updated: 2026-10-08. Next: M2 pass search strategy, API, and tests._

## State
M1/hardening and M2's local catalogue slice are complete. Physical-pass product
semantics are now accepted in [0012](decisions/0012-physical-pass-semantics.md),
but prediction is not implemented. Render remains a stub, sim static, firmware
absent. M2 prediction and operating budgets remain open in [PLAN](PLAN.md).

## This session
Recorded pass semantics, configurable defaults, incomplete/window-limited
results, and separate horizon-based radar eligibility in 0012. Updated
[DESIGN](DESIGN.md) for local-sky framing and “next pass” wording; checked off
product semantics in PLAN. Documentation only; no algorithm or API settled.

## Next step
Read 0012, then design search strategy, shared-core API, and tests before coding.
Address short-pass detection separately from crossing-time refinement. Settle
threshold/window boundaries, tangencies, configuration validation, and failure
reporting without silently weakening the agreed semantics. Keep physical
prediction independent of future projection, selection, and rendering.

Confirm available hardware for propagation/prediction benchmarks and minimal
Sharp refresh checks. Ask before flashing or changing toolchains. Capacity,
budgets, scheduling, and search cost remain unmeasured; define over-budget behavior.

## Risks / follow-ups
- A precise refined crossing does not prove a coarse search found every pass.
- No catalogue freshness cutoff or active-catalogue fallback is implemented;
  unknown fetch time stays unknown. Initialization does not assure propagation.
- Broaden independent references to resonant/low-inclination GEO, applicable
  half-day resonances, negative times, and a non-LEO observer pipeline.
- sgp4's epoch helper mishandles dates after February 2100; see 0007.
  Prefer timestamp-bound `state.to_ecef()` per 0009.

## Verification
Documentation links and diff whitespace checked. No builds/tests rerun this
session; prior implementation verification is recorded in [PROGRESS](PROGRESS.md).
No simulator/hardware run, flashing, toolchain, dependency, or code changes.
