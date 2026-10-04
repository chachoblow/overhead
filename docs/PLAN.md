# Overhead — Plan

Build a tested headless tracking engine, then a real-data UI, then the
standalone device. See decisions/0005-engine-first-implementation.md.

M1 is broken into intended session-sized tasks; split further if needed.
Later milestones are outcome-level scope, to refine before starting them.
An unchecked scope item in those milestones is not necessarily one session.

## M0 — Project setup (complete)
- [x] Cargo workspace with a working simulator window
- [x] Pi set up: web search, permission gate
- [x] Docs scaffold; AGENTS.md TODOs filled in (/bootstrap)
- [x] Open questions in DESIGN.md resolved enough to plan M1

## M1 — Verified satellite calculation foundation (current)

Build the physical calculation pipeline in `overhead-core`, exercised by a
headless host runner. Keep host file I/O and reporting outside the no_std
engine. No network fetching or display work required.

- [x] Research SGP4/OMM support and no_std compatibility; propose dependencies
  with rationale. Identify independent reference cases and document time,
  coordinate-frame, unit, and accuracy conventions before implementation.
  → decisions/0006-sgp4-crate-and-conventions.md
- [x] Add a small checked-in OMM fixture with provenance and fixed test
  timestamps; implement ingestion/validation into engine inputs, including
  invalid-data tests. Do not assume parser placement until compatibility is
  checked.
- [x] Implement propagation at an explicit timestamp; validate against trusted
  reference vectors with documented tolerances and error cases.
- [x] Implement Earth-relative coordinate transformations and geodetic
  position/altitude; test against documented reference cases and boundaries.
  → core/src/coordinates.rs; ERFA/pymap3d fixtures; decision 0007
- [x] Implement observer-relative range, azimuth, and elevation for an explicit
  location; add independent reference and geometric edge-case tests.
  → core/src/observer.rs; pymap3d/Skyfield fixtures; decision 0008
- [ ] Add a headless runner for fixture, time, and location inputs; print
  satellite measurements and exercise the complete pipeline reproducibly.

**Done when:** known elements, time, and location produce reproducible,
independently checked measurements without a UI. Tests cover each calculation
layer and the complete pipeline; workspace check and tests pass.

### Early hardware checks (alongside M1–M2)
- [ ] Once propagation works, benchmark it on available ESP32 hardware. Record
  board, build settings, workload, and timings; confirm on the S3 if the first
  board differs. Extend the benchmark to prediction workloads in M2.
- [ ] Run a minimal Sharp display/refresh test when hardware is available;
  establish the output path before full firmware integration.

Ask before flashing hardware or changing toolchains. These are limited risk
checks, not full firmware bring-up. Simulator work can proceed if hardware is
unavailable; the S3 operating budget remains provisional until measured.

## M2 — Catalogue, pass prediction, and operating budget
- [ ] Configure catalogue group selection; merge and deduplicate objects by
  catalogue ID. Use local datasets headlessly; live device fetching is M6.
- [ ] Predict physical passes with configurable elevation threshold and
  look-ahead. Define event semantics and accuracy; test ordinary passes,
  no-pass cases, and objects already above the threshold.
- [ ] Measure prediction cost as well as propagation cost on target hardware.
- [ ] Establish catalogue limits, update scheduling, and explicit behavior
  when the requested workload exceeds the supported budget.

**Done when:** a curated catalogue can be exercised headlessly, upcoming passes
can be inspected and tested, and measured workloads justify the supported
catalogue size and update cadence. Prediction accuracy and cost are evaluated
together, not in isolation.

## M3 — Minimal real-data radar
- [ ] Add shared application state and renderer entry point, simulator frame
  loop, and explicit clock handling.
- [ ] Implement and test the location-centered projection, clipping, and zoom
  from local scales to wider than Earth; settle spatial-view semantics here.
- [ ] Show real satellite markers and basic identification; support scroll
  wheel and up/down zoom controls.
- [ ] Provide a convenient simulator configuration path for tuning.

**Done when:** real satellites move through a zoomable view and displayed
locations can be traced to verified engine outputs. Coastlines and visual
slow-down are not prerequisites.

## M4 — Useful display behavior
- [ ] Add automatic satellite selection with stable switching behavior.
- [ ] Show true selected-satellite measurements, time, orbit-data age, and zoom.
- [ ] Connect next-pass prediction to a countdown and useful empty-sky behavior.
- [ ] Prepare coastline data offline and render it legibly across zoom levels.

**Done when:** the display is understandable and useful both during a pass and
when the local sky is empty.

## M5 — Motion and visual refinement
- [ ] Smooth marker motion between calculated updates.
- [ ] Implement configurable minimum on-screen traversal time, with explicit
  behavior for zoom and selection changes during slowed motion.
- [ ] Keep presentation state separate from physical state; verify readouts
  remain tied to true positions.
- [ ] Tune density, labels, transitions, and rendering performance.

**Done when:** fast passes are watchable, transitions are coherent, and
presentation state cannot contaminate physical calculations.

## M6 — Standalone device
- [ ] Run the shared engine and renderer on the S3 with Sharp display output
  and BOOT-button zoom presets.
- [ ] Add Wi-Fi credentials, HTTPS orbit-data retrieval, and SNTP.
- [ ] Persist orbital elements and fetch timestamp; schedule refreshes per
  decision 0002.
- [ ] Handle network failures, missing/invalid time, missing data, and stale
  data explicitly. Cached elements do not solve accurate offline cold boot.
- [ ] Verify end-to-end performance and extended operation on hardware.

**Done when:** the dev-board prototype boots, obtains valid time and orbital
data, tracks satellites, and continues operating between network connections.

## Configuration and boundaries

Keep catalogue groups, observer location, prediction thresholds, workload
limits, zoom, and motion parameters explicit and easy to tune. Configuration
need not be user-facing or runtime-editable on the device. Keep uncertain
presentation algorithms separate rather than building a generic plugin system.

Physical controls beyond BOOT-button presets, enclosure, battery integration,
RTC, and the optional Starlink/full-catalogue layer remain deferred.
