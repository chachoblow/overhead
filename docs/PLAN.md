# Overhead — Plan

**Current: M2; next, catalogue pass aggregation and reporting.**
Build the headless engine, then the real-data UI, then the device
([rationale](decisions/0005-engine-first-implementation.md)). Refine each
slice before starting; later checkboxes are scope, not session-sized tasks.

## Complete
- **M0 — Setup:** workspace, simulator window, project workflow.
- **M1 — Verified tracking foundation:** OMM ingestion, timestamped SGP4,
  Earth-fixed/geodetic and observer geometry, reproducible headless runner.
- **Pre-M2 hardening:** shared UTC validation, timestamp-bound/frame-specific
  positions, finite propagation outputs, checked OMM metadata.

Conventions live in [decisions/](decisions/) (0006–0010);
verification provenance in the [fixture README](../core/tests/fixtures/README.md).

## Early hardware checks (alongside M2)
- [ ] Benchmark propagation on available ESP32 hardware; record board, build,
  mixed LEO/GNSS/GEO workload, and timings. Confirm on the S3 if using another
  board; include prediction cost once implemented.
- [ ] Validate minimal Sharp output/refresh before full firmware integration.

Ask before flashing or changing toolchains. Simulator work need not wait for
hardware, but capacity and cadence remain provisional until measured on the S3.

## M2 — Catalogue, pass prediction, and operating budget
- [x] Settle catalogue conflicts, invalid records, provenance, and nonempty
  acceptance ([0011](decisions/0011-catalogue-ingestion-and-provenance.md)).
- [x] Implement/test local group configuration and checked NORAD-ID merge in
  shared core plus the [headless catalogue tool](../tools/README.md).
  Live fetching and active-catalogue refresh remain M6.
- [x] Settle physical-pass product semantics and configurable defaults
  ([0012](decisions/0012-physical-pass-semantics.md), detection limits clarified
  by [0013](decisions/0013-pass-detection-resolution.md)); keep passes independent
  of screen traversal.
- [x] Settle the bounded search baseline, boundary handling, and partial-result
  behavior ([0014](decisions/0014-pass-search-baseline.md)).
- [x] Finalize the allocation-free shared-core API, configuration validation,
  and synthetic test matrix ([0015](decisions/0015-pass-search-api.md)).
- [x] Implement/test the synthetic elevation search kernel, including boundaries,
  short-event detection limits, refinement, failures, and evaluation budgets.
- [x] Integrate orbital elevation evaluation with orbital pipeline, pass-bracket,
  failure, and budget tests.
- [ ] Add headless catalogue pass aggregation/reporting per 0012–0015,
  preserving partial results and unsearched satellite details.
- [ ] Evaluate representative orbital detection intervals separately from
  crossing tolerance.
- [ ] Measure propagation/prediction cost; set catalogue limits, scheduling,
  and explicit over-budget behavior.

**Done when:** a curated catalogue and upcoming passes are testable headlessly,
with measured costs justifying supported size, cadence, and prediction accuracy.

## M3 — Minimal real-data radar
- [ ] Add shared app state, renderer entry point, simulator loop, explicit clock,
  and convenient simulator configuration.
- [ ] Implement/test location-centered projection, clipping, and zoom from
  local scales to wider than Earth; settle spatial-view semantics.
- [ ] Show real markers and basic identification; wire scroll wheel/up/down zoom.

**Done when:** satellites move through a zoomable view traceable to verified
engine outputs. Coastlines and visual slow-down are not prerequisites.

## M4 — Useful display behavior
- [ ] Add stable automatic selection and true satellite/time/data-age/zoom readouts.
- [ ] Connect next-pass countdown and empty-sky behavior.
- [ ] Prepare coastlines offline and render them legibly across zoom levels.

**Done when:** the view is useful and understandable during passes and empty sky.

## M5 — Motion and visual refinement
- [ ] Smooth motion between calculation updates; add configurable minimum screen
  traversal time with explicit zoom/selection-change behavior.
- [ ] Keep presentation state separate; verify readouts use true positions.
- [ ] Tune density, labels, transitions, and rendering performance.

**Done when:** fast passes are watchable without contaminating physical state.

## M6 — Standalone device
- [ ] Run shared engine/renderer on S3 + Sharp, with BOOT-button zoom presets.
- [ ] Add Wi-Fi credentials, HTTPS elements, SNTP, persistence, and refreshes
  per [0002](decisions/0002-data-and-time-over-wifi.md).
- [ ] Handle network failure, missing/invalid time or data, and stale elements.
- [ ] Verify end-to-end performance and extended hardware operation.

**Done when:** the prototype boots with valid time/data and tracks between
connections. Cached elements alone do not enable accurate offline cold boot.

## Boundaries
Keep tuning inputs explicit; they need not be device settings. Defer uncertain
presentation policies rather than building a plugin system. Knob, enclosure,
battery integration, RTC, real location source, and Starlink/full catalogue
remain outside this roadmap.
