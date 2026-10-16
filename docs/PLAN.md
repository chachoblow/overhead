# Overhead — Plan

**Current: M3 opt-in wide radar and per-satellite off-screen arrows implemented
and visually approved under 0033; square/diagnostic default retained. Marker
identification is next to scope; glyphs remain unimplemented. M2 operating-budget
and scheduling work remains open.**
Engine-first correctness remains the foundation
([0005](decisions/0005-engine-first-implementation.md)); visual iteration now informs
remaining product budgets under [0021](decisions/0021-radar-first-iteration-on-verified-engine.md).
Refine each slice before starting; later checkboxes are scope, not session-sized tasks.

## Complete
- **M0 — Setup:** workspace, simulator window, project workflow.
- **M1 — Verified tracking foundation:** OMM ingestion, timestamped SGP4,
  Earth-fixed/geodetic and observer geometry, reproducible headless runner.
- **Pre-M2 hardening:** shared UTC validation, timestamp-bound/frame-specific
  positions, finite propagation outputs, checked OMM metadata.

Conventions live in [decisions/](decisions/) (0006–0010);
verification provenance in the [fixture README](../core/tests/fixtures/README.md).

## Early hardware checks (alongside engine/UI work)
- [x] Benchmark propagation and prediction on confirmed S3 hardware; record board,
  build, mixed LEO/HEO/GNSS/GEO workloads, and timings
  ([first target baseline](evaluations/s3-costs.md)).
- [ ] Validate minimal Sharp output/refresh before full firmware integration.

Ask before flashing or changing toolchains. Simulator work need not wait for
hardware; the first S3 kernel baseline alone does not justify capacity or cadence.

## M2 — Catalogue, pass prediction, and operating budget
- [x] Checked local group configuration, catalogue merge/provenance and headless
  reporting under [0011](decisions/0011-catalogue-ingestion-and-provenance.md).
  [Tool usage](../tools/README.md#overhead-catalogue); live fetching/refresh remain M6.
- [x] Allocation-free physical-pass search and orbital integration, including
  boundaries, refinement, failures and evaluation budgets. Contracts:
  [semantics](decisions/0012-physical-pass-semantics.md),
  [detection limits](decisions/0013-pass-detection-resolution.md),
  [baseline](decisions/0014-pass-search-baseline.md),
  [API](decisions/0015-pass-search-api.md). Catalogue aggregation retains partial/
  unsearched work and [uncertain arrival ordering](decisions/0016-catalogue-pass-aggregation.md).
- [x] [Detection-interval study](evaluations/detection-intervals.md), separate from
  crossing tolerance; no detection default selected.
- [x] Kernel cost evidence on [host](evaluations/host-costs.md) and S3:
  [baseline](evaluations/s3-costs.md), [16-slot follow-up](evaluations/s3-scaling-16.md),
  [expanded age/search/scaling suites](evaluations/s3-expanded.md). Repeated slots
  measure compute, not distinct-catalogue capacity.
- [x] Distinct 4/8-object initialization, retained aggregation and heap experiments
  on [host](evaluations/catalogue-costs.md) and [S3](evaluations/s3-catalogue-memory.md),
  including partial/unsearched work and observed stack writes. Host heap is not
  target RAM; target observations are not whole-device peaks or safe-stack bounds.
- [x] Source-level DROM remedy and [offline image gate](../firmware/IMAGE_GATE.md)
  under [0018](decisions/0018-source-level-drom-remedy-and-image-gate.md), with
  [diagnosis and corrected catalogue-memory boots](evaluations/s3-drom-diagnostic.md).
  The separate RWX warning remains open.
- [x] Bounded preparation under [0019](decisions/0019-bounded-m2-evidence-preparation.md):
  [pinned-pool suitability](evaluations/m2-pool-suitability.md),
  [host ingestion/work/memory preflight](evaluations/m2-host-preflight.md), and
  [20-case manifest/numerical freeze](evaluations/m2-frozen-manifest.md) under
  [0020](decisions/0020-m2-capture-manifest-freeze.md).
- [x] [Frozen harness/image gates](../firmware/M2_CAPTURE.md) and approved target
  captures. All frozen gates pass; exact sample counts, numerical checks, memory/
  stack observations and limitations live in the [results](evaluations/s3-m2.md).
  Reproduce only at the [pinned revision](evaluations/s3-m2/README.md#offline-validation-and-table-regeneration),
  not the evolving workspace. No new matrix or production policy is implied.
- [ ] Use the visual workload and measured target costs to set catalogue/input/result
  limits, lookahead, calculation/prediction cadences and explicit over-budget behavior.
- [ ] Select scheduling and verify bounded latency separately. Production prediction
  scheduling does not block the initial M3 slice; M2 is not closed by starting it.

**Done when:** a curated catalogue and upcoming passes are testable headlessly,
with measured costs justifying supported size, cadence, and prediction accuracy.

## M3 — Minimal real-data radar (in progress)
[DESIGN](DESIGN.md#interface-intent) owns the visual brief and open panel questions;
[simulator usage](../sim/README.md) owns controls and comparison examples.

- [x] Shared clockless app state, 1-bit renderer and explicit-time/local-catalogue
  replay with real markers and basic identification ([0022](decisions/0022-minimal-ground-map-simulator.md)).
- [x] Replace the initial ground map with actual-3D fixed-camera perspective,
  lens zoom and a shared WGS-84 Earth outline under
  [0026](decisions/0026-fixed-camera-perspective-slice.md). Physical outputs,
  true observer-horizon eligibility and explicit failures remain independent.
- [x] Review motion, overview and viewport comparisons; implement
  [0027's higher camera](decisions/0027-higher-fixed-camera-overview.md) and
  [0028's borderless square](decisions/0028-borderless-square-viewport.md).
- [x] Agree normal/debug separation and routine field reduction under
  [0029](decisions/0029-normal-and-debug-presentation-direction.md), as direction
  only; the diagnostic panel is unchanged.
- [x] Agree [0032's wide-radar/glyph direction](decisions/0032-wide-radar-and-selected-satellite-glyphs.md):
  off-screen indicators, marker-adjacent ID and a right-hand category/data-glyph
  strip. [Retained reference](DESIGN.md#selected-satellite-presentation) is a
  starting point, not an implemented or fully specified UI.
- [x] Agree and implement [0033's bounded wide-radar comparison](decisions/0033-wide-radar-comparison-contract.md):
  opt-in 352×240, vertical FOV preserving pixel scale, one arrow per eligible
  off-screen satellite with overlaps allowed, no invalid-depth arrows, existing
  visible-only selection, and minimal explicit warning notices. Default square
  diagnostics retained; [usage/comparisons](../sim/README.md#wide-radar-comparison).
- [x] User reviewed and approved the wide comparison, retaining the current
  layout and overlaps-allowed arrows. No collision avoidance or default-mode
  change follows from that approval.
- [ ] Scope selected-marker identification next: bare NORAD ID and the ISS
  exception under 0032, with bounded edge placement and collision behavior
  agreed before implementation. Deferred to the next session at user request.
  Glyph mappings/transitions, final normal/debug access and refined empty/
  incomplete-state communication remain open. Neither the synthetic mock
  population nor the replay examples select production capacity.

No prediction integration, slowdown or live fetching is required for this slice.
Simulator throughput is not target capacity evidence.

**Done when:** satellites move through a zoomable view traceable to verified
engine outputs, with initial visual feedback on density and layout. Coastlines,
visual slow-down and pass countdowns are not prerequisites.

## M4 — Useful display behavior
- [ ] Refine secondary information with visual feedback; add stable automatic
  selection and the chosen true-state readouts. 0032 removes routine normal-view
  FOV/counts; time and data age are not requirements of its starting layout.
- [ ] Refine empty-sky behavior; evaluate next-pass countdown integration where
  useful, without sacrificing radar quality or responsiveness.
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
