# Overhead — Handoff

_Last updated: 2026-10-09. Next: confirmed S3 measurements and operating budgets._

## State
M1/hardening, catalogue ingestion/reporting, orbital pass search, interval
evaluation, and host kernel cost measurements are complete. M2 still needs
S3 measurements and operating policy. Render stub, sim static, firmware absent.
Scope: [PLAN](PLAN.md).

## This session
Added [overhead-benchmark](../tools/src/bin/overhead-benchmark.rs): 62 workloads
covering propagation/geometry, prediction interval/window/tolerance sweeps, and
4/16/64 repeated mixed-fixture slots. Shared embedded inputs via
[historical_orbits.rs](../tools/src/historical_orbits.rs); fixtures unchanged.
[Results/raw runs](evaluations/host-costs.md): M1 Pro 24h/60s searches ~0.7–0.9ms
per fixture; 64-slot batch ~48.6ms. These are kernel costs, not catalogue capacity,
memory bounds, or ESP32 timings. Four new tests; [usage](../tools/README.md).
No production interval, capacity, cadence, scheduling, or numeric allowance chosen.

## Next step
Confirm available board/display and permission before flashing or toolchain
changes. Measure equivalent propagation/prediction workloads on the S3; broaden
ages/orbits and measure distinct-catalogue storage/aggregation and peak memory.
Use target evidence to choose limits, cadence, scheduling, and over-budget behavior.

## Risks / follow-ups
Host samples are not worst-case/device bounds; repeated fixtures omit larger
working sets. ±12h does not bound older-element resonant costs. Synchronous search
is not background scheduling. [Detection limits](evaluations/detection-intervals.md)
remain: short excursions/gaps can be missed even at 5s; tolerance cannot fix this.
Broader natural grazes/phases and independent orbital references remain follow-ups.
Accepted-catalogue completion omits rejected inputs; no freshness cutoff/fallback.

## Verification
Workspace check/default/all-feature tests pass: 145 tests plus 6 doctests.
Standalone core feature checks, strict Clippy, fmt, rustdoc, and diff checks pass.
Two release runs have identical work counts, variable timings; raw samples saved.
No dependency/toolchain changes, simulator launch, flashing, or hardware operations.
