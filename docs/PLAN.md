# Overhead — Plan

**Current: M2; DROM source remedy/image gate pass offline; approved corrected boot and bounded evidence preparation next.**
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
- [x] Benchmark propagation and prediction on confirmed S3 hardware; record board,
  build, mixed LEO/HEO/GNSS/GEO workloads, and timings
  ([first target baseline](evaluations/s3-costs.md)).
- [ ] Validate minimal Sharp output/refresh before full firmware integration.

Ask before flashing or changing toolchains. Simulator work need not wait for
hardware; the first S3 kernel baseline alone does not justify capacity or cadence.

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
- [x] Add headless catalogue pass aggregation/reporting per 0012–0015,
  preserving partial results and unsearched satellite details; uncertain
  arrival ordering follows [0016](decisions/0016-catalogue-pass-aggregation.md).
- [x] Evaluate representative orbital detection intervals separately from
  crossing tolerance ([results and limits](evaluations/detection-intervals.md));
  no detection default selected.
- [x] Add reproducible host propagation/geometry/prediction benchmarks by orbit
  class and repeated mixed workload size ([results](evaluations/host-costs.md));
  kernel throughput only, not catalogue capacity or device limits.
- [x] Establish the first S3 propagation/prediction baseline: 14 workloads with
  matching host work counts; no display/PSRAM dependency ([results](evaluations/s3-costs.md)).
- [x] Prepare separately runnable age/search/scaling suites and capture the first
  16-slot S3 workload ([results](evaluations/s3-scaling-16.md)); repeated slots
  remain compute evidence, not catalogue capacity.
- [x] Capture V2 baseline, 64-slot scaling, and all prepared signed-age/search-setting
  suites: 120 workloads × three samples, matching host counts
  ([results and evidence](evaluations/s3-expanded.md)).
- [x] Measure 4/8 distinct historical catalogues at common UTC on host: checked
  initialization, retained aggregation, and requested heap peaks, including
  partial/unsearched work ([results](evaluations/catalogue-costs.md)); not target RAM.
- [x] Prepare a separate bounded S3 catalogue/heap/written-stack experiment with
  host work expectations and strict capture validation
  ([method](../firmware/CATALOGUE_MEMORY.md)).
- [x] Capture first 4/8-object S3 initialization/aggregation heap costs and observed
  stack writes: two runs × eight cases × three samples, matching host counts
  ([results and limits](evaluations/s3-catalogue-memory.md)); not whole-device peak RAM.
- [x] Investigate the multiple-DROM boot diagnostic offline: section-gap cause
  reproduced and this image's mapping explained ([findings](evaluations/s3-drom-diagnostic.md));
  source remedy below; no corrected target capture yet.
- [x] Prepare a source-level DROM remedy/image gate: both firmware binaries,
  64 KiB stress builds, four alignment fixtures and a NOBITS negative control
  pass offline ([method](../firmware/IMAGE_GATE.md), [0018](decisions/0018-source-level-drom-remedy-and-image-gate.md)).
- [ ] Verify a corrected normal-build boot/capture with approval; the separate
  RWX warning remains.
- [ ] Refine/freeze the [bounded next-evidence proposal](evaluations/m2-next-evidence.md),
  then broaden distinct orbital/pass-density and collection/input-memory samples,
  strengthen stack evidence, and add independent target numeric checks. Set catalogue
  limits, cadence, scheduling, and explicit over-budget behavior.

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
