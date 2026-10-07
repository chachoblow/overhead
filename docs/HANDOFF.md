# Overhead — Handoff

_Last updated: 2026-10-07. Next: refine M2 physical pass prediction._

## State
M1/hardening and M2's first local catalogue slice are complete. Shared core now
merges checked OMM records; the offline tool loads configured groups and reports
selected provenance. Render remains a stub, sim static, and firmware absent.
M2 is not complete: prediction and operating budgets remain open in [PLAN](PLAN.md).

## This session
Settled catalogue policy in [0011](decisions/0011-catalogue-ingestion-and-provenance.md).
Implemented opt-in `core/src/catalogue.rs`, `tools/src/bin/overhead-catalogue.rs`,
a historical example manifest, and 20 tests. Updated crate features, AGENTS,
PLAN, and [tools usage](../tools/README.md). No new crate/version or fixture changes;
Serde is now direct in tools and serde_json's raw_value feature preserves keys.

## Next step
Refine pass-event semantics before coding: elevation threshold, look-ahead,
already-above-threshold behavior, no pass, event accuracy, and search cost.
Keep physical passes independent of screen traversal. Catalogue APIs/tests own
merge details; firmware refresh/cache/logging are not implemented.

Confirm available hardware for propagation benchmarks and minimal Sharp refresh
checks. Ask before flashing or changing toolchains. Capacity, allocation and
diagnostic budgets, scheduling, and prediction cost remain unmeasured.

## Risks / follow-ups
- No freshness cutoff or active-catalogue fallback is implemented; unknown fetch
  time stays unknown. SGP4 initialization does not guarantee propagation success.
- Broaden independent references to resonant/low-inclination GEO, applicable
  half-day resonances, negative times, and a non-LEO observer pipeline.
- sgp4's epoch helper mishandles dates after February 2100; only our rotation
  avoids it ([0007](decisions/0007-coordinate-implementation-conventions.md)).
- Prefer timestamp-bound `state.to_ecef()` ([0009](decisions/0009-core-time-and-frame-api.md)).

## Verification
Workspace check, default/all-feature tests (78 + 3 compile-fail doctests), core
checks with no features/OMM/catalogue, strict Clippy, fmt, and the offline example
passed. No simulator launch, hardware run, flashing, or toolchain changes.
