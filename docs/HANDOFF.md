# Overhead — Handoff

_Last updated: 2026-10-09. Next: measured propagation/prediction operating budgets._

## State
M1/hardening, catalogue ingestion/reporting, orbital pass search, and representative
interval evaluation are complete. M2 still needs measured operating budgets.
Render is a stub, sim static, firmware absent. Scope: [PLAN](PLAN.md).

## This session
Added [overhead-evaluate-intervals](../tools/src/bin/overhead-evaluate-intervals.rs):
15 cases / 630 searches over ISS, resonant HEO, GNSS and GEO, varying detection
interval, sampling phase, and crossing tolerance independently. Added historical
[TLE inputs/provenance](../tools/fixtures/README.md) and six tests, including
[executable coverage](../tools/tests/intervals.rs). Existing ISS fixture unchanged.
[Results/limits](evaluations/detection-intervals.md): ordinary sampled cases match
same-model dense scans, while constructed ~4s excursions/gaps can be missed even
at 5s spacing. Tighter crossing tolerance does not recover those misses.
No detection default, numeric operating allowance, capacity, or policy selected.

## Next step
Measure propagation/prediction cost by orbit class, then on confirmed ESP32-S3
hardware; use evidence to select capacity, cadence, scheduling, and over-budget
behavior. Confirm available board/display before hardware operations. Tool usage:
[tools/README.md](../tools/README.md). Ask before flashing/changing toolchains.

## Risks / follow-ups
Same-model grids are not independent timing accuracy or a completeness proof;
constructed cases do not estimate miss frequency. Broaden natural threshold
grazes/phase coverage when selecting an interval, and independent resonant/GEO,
negative-time and non-LEO geometry references. Synchronous catalogue aggregation
allocates records; evaluation allowances do not bound memory or provide scheduling.
Completion covers accepted records only; no freshness cutoff/active fallback exists.
Initialization does not assure propagation. Existing numerical caveats: 0007/0009.

## Verification
Workspace check/default/all-feature tests pass: 141 tests plus 6 doctests.
Standalone core feature checks, strict Clippy, fmt, rustdoc, and diff checks pass.
Two release evaluations are byte-identical on this host. No dependency/toolchain
changes, simulator launch, flashing, or hardware operations.
