# Overhead — Plan

**Current: M0, M1, M2 and M3 complete (M2 closed under
[0077](decisions/0077-m2-operating-budget-visual-group.md) on 2026-11-03); M4
done for now (user, 2026-10-24); M6 standalone device remains open; M6 element
fetch runs under [0071](decisions/0071-https-element-fetch-first-slice.md),
[0074](decisions/0074-live-utc-radar-on-fetched-elements.md) and
[0075](decisions/0075-fetched-element-refresh-and-staleness.md); the device
app build is [0076](decisions/0076-device-app-build.md).**
Engine-first correctness remains the foundation
([0005](decisions/0005-engine-first-implementation.md)); visual iteration now informs
remaining product budgets under [0021](decisions/0021-radar-first-iteration-on-verified-engine.md).
Checkboxes are scope; work is taken on as outcome-sized work items under
[0072](decisions/0072-outcome-sized-work-items.md), tracked in `docs/work/` while active.
Unless noted, each interface slice below was contract-approved, separately
authorized, implemented, and then accepted by the user on fresh executable
output, separately from mocks and host checks.

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
- [x] Validate minimal Sharp output/refresh before full firmware integration:
  the opt-in [wiring/refresh test](../firmware/README.md#isolated-sharp-wiringrefresh-test)
  booted on the S3 and the user confirmed the alternating pattern on the physical
  #4694 panel. This is not radar integration or whole-device cadence evidence.

Flashing needs no approval ([0069](decisions/0069-flash-without-per-flash-approval.md));
ask before changing toolchains. Simulator work need not wait for
hardware; the first S3 kernel baseline alone does not justify capacity or cadence.

## M2 — Catalogue, pass prediction, and operating budget (complete)
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
- [x] Set catalogue/input limits, cadence and over-budget behavior under
  [0077](decisions/0077-m2-operating-budget-visual-group.md): CelesTrak's `visual`
  group, 128 KiB bodies in PSRAM, at most 256 objects (role then NORAD ID, dropped
  count on debug only), the selected object every frame and the rest within 1 s.
  Device pass prediction (lookahead, prediction cadence) is deferred, not set.
- [x] Verify the staggered schedule's bounded update cost and heap on the board
  ([evidence](../firmware/README.md#visual-group-under-the-m2-budget-0077)).

**Done when:** a curated catalogue is testable headlessly, with measured costs
justifying supported size and cadence. Amended by 0077: device pass prediction is
deferred, so upcoming passes remain host-tested (0012–0016) rather than a device
budget.

## M3 — Minimal real-data radar (complete)
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
  strip; a starting point, not a fully specified UI.
- [x] [0033's bounded wide-radar comparison](decisions/0033-wide-radar-comparison-contract.md):
  opt-in 352×240, vertical FOV preserving pixel scale, one arrow per eligible
  off-screen satellite with overlaps allowed, no invalid-depth arrows, visible-only
  selection and minimal explicit warnings. The user approved the layout and
  overlaps-allowed arrows; no collision avoidance or default-mode change follows.
  Default square diagnostics retained; [usage](../sim/README.md#wide-radar-comparison).
- [x] [0034's selected-marker identification](decisions/0034-selected-marker-identification.md)
  (bare NORAD ID / ISS, right-side with left fallback, vertical clamping,
  overlaps allowed), refined by [0035](decisions/0035-label-zoom-visibility.md)'s
  configurable hide at ≥70° / restore at ≤60° vertical FOV with the selected
  diamond retained and no dithering. Accepted for now.
- [x] Glyph direction history: [0036's capsule](decisions/0036-shared-glyph-capsule.md)
  (direction only) and [0037's elevation-only capsule](decisions/0037-static-elevation-glyph-contract.md)
  (implemented, accepted) were superseded by [0038's labelled borderless direction](decisions/0038-labelled-borderless-glyph-direction.md):
  labelled stack, padded separator, circular elevation cutout, concentric
  altitude circles, speed arc, period spiral, navigation cross and station
  body-and-wings icon.
- [x] [0039](decisions/0039-bottom-left-condition-driven-notices.md)/[0040](decisions/0040-white-backed-bottom-left-notice-contract.md)
  bottom-left condition-driven notices with a small borderless white backing
  drawn last; they clear only when conditions resolve, never on a timeout, and
  may cover crossing graphics without changing eligibility or selection.
- [x] Five static fields, each preserving existing radar pixels, notices and
  square output, with exact raster/lifecycle and independent real-data snapshot
  tests: [0041](decisions/0041-circular-elevation-only-contract.md) circular
  elevation and separator; [0042](decisions/0042-static-altitude-glyph-contract.md)
  fixed compressed WGS-84 altitude; [0043](decisions/0043-static-speed-glyph-contract.md)
  TEME speed on a linear 0–12 km/s arc; [0044](decisions/0044-static-period-glyph-contract.md)
  nominal period, logarithmic 90-minute/2½-turn to 24-hour/one-turn (the user
  chose 2½ over three turns); [0045](decisions/0045-static-group-category-contract.md)
  explicit group roles, Station > Navigation > Weather > Unknown, with no per-ID
  mapping, inferred roles or operational-status claim. [Review recipes](../sim/README.md#static-category-glyph).
- [x] [0046's selected-data transitions](decisions/0046-selected-data-glyph-transition-contract.md):
  four animated glyphs on selected-ID changes, immediate Cat, 250 ms smoothstep,
  explicit presentation time and bounded retarget/loss behavior. The user
  accepted interactive output; headless snapshots stay static.
- [x] [0047's text-first debug access](decisions/0047-text-first-debug-access-contract.md):
  radar-free tracking/data-health page, wide-view `v` toggle and `--wide --debug`
  start, after approval of the [layout/overflow treatment](mockups/0047-debug-layout-proposal.md)
  (illustrative pages, not executable output). [Review recipe](../sim/README.md#debug-access).
  No scene displacement or collision search is approved. Circular-radar/field/graph
  alternatives remain exploration; neither mocks nor replay populations select
  production capacity.

No prediction integration, slowdown or live fetching is required for this slice.
Simulator throughput is not target capacity evidence.

**Done when:** satellites move through a zoomable view traceable to verified
engine outputs, with initial visual feedback on density and layout. Coastlines,
visual slow-down and pass countdowns are not prerequisites.

**Exit assessment — 2026-10-18 UTC:** complete. Explicit-time replay and app
regressions verify motion and lens zoom without changing physical truth;
headless snapshots independently check geometry and selected fields across
catalogues, times and zooms. Accepted camera/layout/density feedback is recorded
in 0027, 0033 and 0035, with the selected fields, transitions and debug access
accepted above. Fresh workspace check/tests, app/render no-default-feature check
and formatting check passed against source revision `64db11a`; no code change
was needed. Existing user visual acceptance is separate from these host checks.
M2 operating budgets/scheduling and physical-display/target validation remain open;
closure neither selects production capacity/cadence nor authorizes follow-on work.

## M4 — Useful display behavior
- [x] Agree [0048's bounded wide-radar empty-view notice contract](decisions/0048-refined-empty-view-notices.md):
  distinguish arrow-eligible off-screen objects from clean no-above-horizon
  samples, retaining conservative fallback and independent warnings. The user
  approved wording and wider backing after reviewing illustrative native-size
  comparisons. Separate implementation authorization was granted; those
  comparisons are not executable visual acceptance. Square diagnostics
  and existing radar/selection behavior remain unchanged.
- [x] Implement 0048 in a separately authorized bounded slice, preserving
  0039/0040 notice lifecycles and provenance boundaries. Host checks and tests
  cover classification, backing and headless real-data cases. The user separately
  reviewed and accepted fresh executable arrow and horizon snapshots.
- [x] Agree [0049's stable wide-selection contract](decisions/0049-stable-wide-selection.md):
  retain a successfully tracked above-horizon selection across viewport exits
  from zoom or motion, keep current true-state fields, and use first in-view ID
  only for initial acquisition or genuine loss. Tab still chooses in-view markers.
  Agreement preceded implementation and separate interactive visual acceptance.
- [x] Separately authorize and implement 0049 in the wide view: keep the
  selected object's current true-state fields outside the viewport, Tab to an
  in-view marker, and clear/fall back on horizon or tracking loss. Square/circular
  selection stays visible-only. Host app/renderer/simulator regressions cover
  continuity, strip/debug and no-stale-data behavior; no selected-arrow styling.
- [x] User confirmed reviewing and accepting interactive executable 0049
  selection continuity as well as the code/contract, separately from host tests.
- [x] [0061](decisions/0061-filled-arrows-id-backing-none-in-view.md) after a
  snapshot review: filled off-screen arrows, 1px white backing behind the
  selected ID and selected diamond, and `NONE IN VIEW` replacing `SKY OUTSIDE VIEW`.
  Unselected markers stay unchanged after a style comparison.
- [x] Refine secondary information with visual feedback and choose any further
  true-state readouts separately. 0032 removes routine normal-view FOV/counts;
  time and data age are not starting-layout requirements. The [range comparison](mockups/0050-range-vs-no-readout.md)
  ended with range remaining in debug under [0050](decisions/0050-keep-range-in-debug-and-defer-display-prediction.md);
  no additional normal-view readout was approved.
  Marked done for now (user, 2026-10-24) after the 0061 snapshot review.
- [x] Refine empty-sky behavior without adding display prediction for now;
  next-pass countdown study/integration is deferred under 0050, not cancelled
  or substituted for [0048's notices](decisions/0048-refined-empty-view-notices.md).
  Marked done for now (user, 2026-10-24): current empty-sky feel accepted.
- [x] Prepare coastlines offline and render them legibly across zoom levels.
  Marked done for now (user, 2026-10-24); device feel and timing follow 0060.
  - Appearance: [0051's angular direction](decisions/0051-angular-coastline-appearance-direction.md)
    and [native 1-bit comparisons](mockups/0054-coastline-overview-comparison.md)
    cover 5°–100°; [0054](decisions/0054-first-executable-coastline-comparison.md)'s
    wide-only executable comparison (40 km look, exceptional `COAST ERROR` notice)
    was accepted by the user.
  - Completeness: host suites and a [bounded segment study](evaluations/m4-coastline-bounded-segment-study.md)
    (each of 1,734 segments at seven scenes) found no qualifying gap.
    [0055](decisions/0055-defer-all-view-coastline-proof-for-m4.md) stops the
    all-view proof as a gate for this M4 iteration, keeping 0053's
    no-known-ordinary-gap intent and a later completeness TODO; not all observers
    are covered, unknown visibility is not called invisible, and floating-point
    margins lack formal bounds. A host regression covers coastline-only failure
    rollback; natural late numeric failures and their target cost are unmeasured.
  - Cost path: the first candidate failed the provisional 5 ms target gate
    (9.78 s maximum rebuild; [protocol](evaluations/m4-coastline-device-measurement-protocol.md),
    [provisional ceilings](evaluations/m4-coastline-budget-preflight.md)).
    [0056](decisions/0056-bounded-coastline-architecture-trial.md)'s quadratic
    ECEF [representation](evaluations/m4-coastline-representation-spike.md) was
    accepted in appearance but [measured](evaluations/m4-coastline-representation-target.md)
    at 22.5 ms and not adopted for the device.
    [0057](decisions/0057-fixed-observer-coastline-preparation-scope.md) accepts a
    fixed observer during use for this version (globe scrolling/panning stays open
    for later), with a visible one-time setup phase and full loading screen
    ([proposal](evaluations/m4-coastline-location-preparation-scope.md),
    [host trials](evaluations/m4-coastline-observer-preparation-host.md)).
  - Adopted for now: [0058](decisions/0058-prepared-wide-coastline-slice.md)'s
    heap-free fixed-observer layer prepared once, plus
    [0059](decisions/0059-current-view-coastline-raster-cache.md)'s 10,560 B
    current-view raster cache. The [S3 measurement](evaluations/m4-coastline-prepared-target.md)
    gives cached coast 386–1,153 µs, zoom-step rasters up to 21 ms, ~5 ms
    preparation and zero heap. [0060](decisions/0060-defer-timing-to-physical-prototype.md)
    accepts this and defers timing work, including non-coast frame cost, to the
    physical prototype.
  - Still open: location configuration, setup duration, retained memory and
    revised resource policy; selected non-coast frame cost and Sharp/whole-device
    cadence. The first firmware prototype still follows 0004 until a successor
    contract is chosen.

**Done when:** the view is useful and understandable during passes and empty sky.

## M5 — Motion and visual refinement
- [ ] Smooth motion between calculation updates; add configurable minimum screen
  traversal time with explicit zoom/selection-change behavior.
- [ ] Keep presentation state separate; verify readouts use true positions.
- [ ] Tune density, labels, transitions, and rendering performance.

**Done when:** fast passes are watchable without contaminating physical state.

## M6 — Standalone device
- [x] Physical display diagnostics, each separately gated, flashed and captured;
  details and limits live in the [firmware README](../firmware/README.md):
  - First integration: a fixed-time, single real-ISS [radar frame](../firmware/README.md#fixed-fixture-radar-on-sharp-candidate)
    from the shared app/renderer reached the Sharp panel; the user confirmed the
    marker and labelled strip.
  - Motion: an accelerated [deterministic ISS replay](../firmware/README.md#deterministic-radar-replay-v2-physically-accepted)
    was visually accepted after a first, too-short replay looked static.
  - Frame cost: the [cost profile](../firmware/README.md#sharp-full-frame-cost-diagnostic-flashed-short-uart-capture),
    [stage trace](../firmware/README.md#stage-trace-follow-up-flashed-short-uart-capture)
    and [individual-glyph v3](../firmware/README.md#individual-glyph-trace-v3-flashed-short-uart-capture)
    traced ~115 ms of a ~141 ms render to the period glyph. An opt-in renderer
    [period cache](../firmware/README.md#period-cache-comparison-v1-flashed-short-uart-capture)
    gives ~26 ms warm renders, byte-identical to uncached output; ordinary replay
    firmware and the simulator still use the uncached entry points.
  - Transfer: a full 2 MHz frame takes ~52.4 ms (~49.9 ms wire floor). The
    [selective-line diagnostic](../firmware/README.md#selective-line-target-diagnostic-flashed-short-uart-capture-user-visual-check)
    sends changed lines in ~5.4–8.8 ms with host pixel parity; the user reported
    the display looks good.
  - Cadence: the [natural-speed historical diagnostic](../firmware/README.md#natural-speed-historical-diagnostic-flashed-short-uart-capture)
    and [v2 command audit](../firmware/README.md#natural-speed-command-audit-v2-flashed-bounded-uart-capture)
    run host-preflight frame/refresh policy on a 50 ms grid. Skipped deadlines
    occur at startup and after retained full refreshes; the user saw stable,
    gradual motion with no visible flicker or stale rows. This is fixture time,
    not trusted UTC, physical COM-edge, sustained VCOM or production 20 Hz proof.
- [x] Time trust and live UTC, host-first then bounded device diagnostics:
  - [0062](decisions/0062-no-trusted-time-display-preflight.md): `NO FRAME` on
    boot/time loss, full-send confirmation, idle COM independence, fault rejection
    and new-anchor recovery (host preflight).
  - [0063](decisions/0063-bounded-utc-holdover-direction.md)–[0066](decisions/0066-prototype-live-sntp-stack-and-source.md):
    bounded holdover, ≤100 ms desired total UTC error, screened unauthenticated
    unicast SNTP, a [host trust gate](../firmware/src/utc_trust.rs) and
    [reply screen](../firmware/src/sntp_screen.rs), holdover ending before any UTC
    month end, prototype `D = 20 ppm` / `H = 3,600 s` (measured ≈+1.2 ppm on one
    board), and a Cloudflare/NIST source whose one-hour
    [live run](../firmware/README.md#live-sntp-source-evidence-flashed-1-hour-uart-run)
    accepted all 59 replies. Renewals within 100 ms keep tracking state
    ([0063 note](decisions/0063-bounded-utc-holdover-direction.md#note--2026-11-03-small-resyncs-keep-tracking),
    [evidence](../firmware/README.md#small-sntp-resyncs-keep-tracking-0063-note)).
    Remaining device-loop evidence is outlined in the
    [proposal](utc-trust-loss-proposal.md#remaining-evidence-capture-bound-drift-and-stack-proposal).
  - [0067](decisions/0067-opt-in-live-utc-iss-radar-scope.md): the
    [live-UTC radar candidate](../firmware/README.md#live-utc-iss-radar-candidate-flashed-bounded-uart-capture)
    and the [dated-OMM diagnostic](../firmware/README.md#pinned-omm-live-utc-marker-diagnostic-flashed-no-visual-observation).
    On 2026-10-29 PDT the user watched ISS move across the Sharp panel during a
    modelled fixed-observer window, then saw the empty-horizon notice return.
    This is visual evidence, not verified clock/image identity, actual-location
    accuracy, physical COM edges or sustained cadence.
  - Wi-Fi/display contention, SPI failure service, aging elements and
    whole-device timing remain open.
- [ ] Run shared engine/renderer on S3 + Sharp. Intended controls under 0060:
  button cycles satellites, knob zooms continuously (presets as fallback).
  BOOT cycling under [0068](decisions/0068-boot-selection-gesture.md) with
  [0070](decisions/0070-boot-input-capture-independent-of-render.md) interrupt
  capture is implemented in diagnostics and passed user tap sessions;
  [HANDOFF](HANDOFF.md) tracks open BOOT risks. The device app build exists
  under [0076](decisions/0076-device-app-build.md) (`--features device`,
  shared runtime with the `live-fetch-radar` diagnostic, event log only;
  [evidence](../firmware/README.md#device-application-0076)); knob zoom remains.
- [ ] Add Wi-Fi credentials, HTTPS elements, SNTP, persistence, and refreshes
  per [0002](decisions/0002-data-and-time-over-wifi.md). First slice accepted as
  [0071](decisions/0071-https-element-fetch-first-slice.md): mbedtls-rs, STATIONS JSON,
  one request per boot; slices 1–3 done: one on-board fetch ingested 20/20 STATIONS records
  ([evidence](../firmware/README.md#https-element-fetch-on-the-board-flashed-one-boot-uart-capture));
  slice 4 done under [0074](decisions/0074-live-utc-radar-on-fetched-elements.md): the
  `live-fetch-radar` diagnostic checks certificate dates against screened UTC and tracks
  the fetched STATIONS on the live-UTC radar (user-observed pass with BOOT cycling,
  [evidence](../firmware/README.md#live-utc-radar-on-fetched-stations-flashed-uart-captures)).
  Refresh done under [0075](decisions/0075-fetched-element-refresh-and-staleness.md):
  8-hour refresh with request-boundary backoff and a build-time home site;
  seven on-board refreshes with no heap growth and a user-observed pass
  ([evidence](../firmware/README.md#refresh-staleness-and-home-site-0075)).
  Flash persistence and a real location source remain open.
- [x] Handle network failure, missing/invalid time or data, and stale elements.
  Under 0075: failed refreshes keep the previous catalogue, `OLD DATA` after
  24 h, unload after 7 days. Shown on the board with the opt-in
  `live-fetch-faults` build: DNS, TCP, handshake and verification failures
  classified and scheduled as decided, `OLD DATA` raised and cleared by a load,
  expiry to `NO DATA`, Wi-Fi drop recovered without reboot
  ([evidence](../firmware/README.md#fetch-failure-paths-on-the-board-live-fetch-faults-0075)).
  Post-GET failures, the repeated-handshake rule and reload after expiry
  remain host-tested only.
- [ ] Verify end-to-end performance and extended hardware operation, including
  the frame-timing work deferred by 0060.

**Done when:** the prototype boots with valid time/data and tracks between
connections. Cached elements alone do not enable accurate offline cold boot.

## Boundaries
Keep tuning inputs explicit; they need not be device settings. Defer uncertain
presentation policies rather than building a plugin system. Knob hardware and
continuous zoom are in scope under M6 ([0060](decisions/0060-defer-timing-to-physical-prototype.md)).
Enclosure, battery integration, RTC, real location source, and Starlink/full
catalogue remain outside this roadmap.
