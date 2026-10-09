# Overhead — Handoff

_Last updated: 2026-10-08. Next: representative detection-interval evaluation._

## State
M1/hardening, local catalogue ingestion, orbital pass search, and headless
catalogue reporting are complete. M2 still needs interval evaluation and measured
operating budgets. Render is a stub, sim static, firmware absent. Scope: [PLAN](PLAN.md).

## This session
Added alloc-enabled [catalogue aggregation](../core/src/catalogue_passes.rs) with
all satellite reports/partial records and uncertain earliest-arrival candidates
per [0016](decisions/0016-catalogue-pass-aggregation.md). Added `overhead-passes`
and extracted the unchanged loader into [tools/src/catalogue.rs](../tools/src/catalogue.rs);
CLI arguments, output, and exit codes are in [tools/README.md](../tools/README.md).
Added 17 tests across core ordering/orbital aggregation and executable reporting.
No detection default, numeric allowance, capacity, or scheduling policy was selected.

## Next step
Evaluate representative orbital detection intervals independently of crossing
tolerance: brief excursions/gaps, threshold grazes, and mixed orbit classes.
Distinguish same-model integration comparisons from independent timing accuracy
and detection coverage. Then measure propagation/prediction cost to justify
catalogue limits, cadence, scheduling, and explicit over-budget behavior.

## Risks / follow-ups
Completed searches can miss brief events; refinement does not prove unique
crossings. Aggregation allocates stored records and traverses NORAD IDs synchronously;
work limits are not memory limits or scheduling. Completion covers the accepted
catalogue, not records rejected at ingestion. Capacity/cadence remain unmeasured.
Confirm hardware for ESP32 benchmarks and minimal Sharp refresh checks; ask before
flashing or changing toolchains. No freshness cutoff or active-catalogue fallback
exists; unknown fetch time stays unknown. Initialization does not assure propagation.
Broaden independent references to resonant/GEO, negative times, and non-LEO observer
geometry. See 0007 for the sgp4 epoch-date issue after February 2100; use bound rotation per 0009.

## Verification
Workspace check and default/all-feature tests pass: 135 tests plus 6 doctests.
Standalone core feature checks, strict Clippy, fmt, and rustdoc checks pass.
No dependency/fixture/toolchain changes, simulator launch, or hardware operations.
