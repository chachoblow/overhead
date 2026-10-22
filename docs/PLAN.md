# Overhead — Plan

**Current: M3 complete after exit verification on 2026-10-18 UTC;
M4 empty-view notices implemented under 0048; the user accepted fresh
executable output separately from contract mocks and host checks. Opt-in wide radar and per-satellite off-screen arrows are implemented and visually approved under 0033; square/diagnostic default retained. Selected-marker
identification is implemented under 0034, refined by 0035's configurable 70°/60°
hide/show hysteresis after review of crowding; the user has now accepted that behavior.
0037's visually accepted elevation-only capsule is now replaced by 0041's
static circular cutout, bitmap `Elev` label and padded separator. This is the
first bounded implementation of 0038's labelled borderless direction, not the
full glyph stack. The user reviewed and accepted 0041's executable output.
0039's bottom-left condition-driven notices remain implemented under 0040's
white-backed overlay and visually accepted. 0042's static altitude addition is
now implemented after separate authorization; the user accepted its fresh
executable output separately from the approved contract/illustrative comparison.
0043's static TEME speed arc is implemented after approval of its contract,
illustrations and implementation. The user subsequently reviewed and accepted
fresh executable speed output. 0044's static nominal-period spiral is implemented
after approval of meaning, fixed logarithmic scale, 2½-turn appearance and
lifecycle, plus authorization to record exact raster rules and implement.
The user subsequently reviewed and accepted fresh executable period output;
0045 now implements static category after approval of icons, explicit group-role
assignment, Station > Navigation > Weather > Unknown priority and immediate
five-field lifecycle, followed by implementation authorization. The user subsequently
reviewed and accepted fresh executable category output; 0046's bounded animated
transitions are implemented and user visually accepted. 0047's text-first,
radar-free debug access and reviewed layout/overflow treatment are implemented
after separate authorization; the user subsequently accepted fresh executable output.
M2 operating-budget and scheduling work remains open.**
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
- [x] Agree and implement [0034's selected-marker identification](decisions/0034-selected-marker-identification.md)
  in the opt-in wide view: bare NORAD ID / ISS, right-side placement with left-side
  fallback, vertical clamping and transparent overlaps allowed. Baseline unchanged.
- [x] Refine crowded overview identification under [0035](decisions/0035-label-zoom-visibility.md):
  clean label hide at ≥70° / restore at ≤60° vertical FOV with hysteresis,
  configurable launch thresholds, selected diamond retained. No dithering.
- [x] User reviewed and accepted 0035's 70°/60° zoom-visibility behavior for now;
  retain configurability and existing placement/selection rules.
- [x] Agree [0036's shared glyph capsule](decisions/0036-shared-glyph-capsule.md):
  1px rounded outline with one centered break per long side. Visual direction
  only; [retained mock](DESIGN.md#glyph-capsule-reference), not implemented UI.
- [x] Agree [0037's static elevation-only contract](decisions/0037-static-elevation-glyph-contract.md):
  24×24 true-elevation fill, shortened shared capsule, no capsule/glyph without
  selection, and a fixed warning area below. No animation or other glyphs.
- [x] Implement 0037 in the opt-in wide view, preserving radar pixels, square
  diagnostics, selection, labels and explicit warnings; pixel/lifecycle tests and
  independent real-data glyph snapshots pass.
- [x] User reviewed and accepted the static elevation/capsule snapshot output
  under 0037, including the shorter capsule and separate warning area.
- [x] Agree and retain [0038's revised glyph direction](decisions/0038-labelled-borderless-glyph-direction.md):
  borderless labelled stack, padded vertical separator, outlined circular
  elevation cutout, concentric altitude circles, speed arc and period spiral;
  thin navigation cross and simple station body-and-wings icon. This amends the
  earlier capsule/shapes/no-label direction; 0041 below implements one field.
- [x] Select [0039's bottom-left condition-driven notices](decisions/0039-bottom-left-condition-driven-notices.md)
  after reviewing top-left/bottom-left mocks; preserve existing notice meanings
  and clear only resolved conditions, not on a timeout. Retained composites are
  not executable warning-state snapshots.
- [x] Agree and implement [0040's notice-only overlay](decisions/0040-white-backed-bottom-left-notice-contract.md):
  bottom-left, complete notice lines, small borderless white backing drawn last,
  explicit permission to visually cover crossing graphics without changing
  eligibility/selection. Preserve the elevation capsule and square diagnostics;
  pixel/lifecycle/real-marker-overlap and headless warning tests pass.
- [x] User confirmed review and acceptance of 0040's executable notice snapshots.
- [x] Agree [0041's circular elevation-only contract](decisions/0041-circular-elevation-only-contract.md):
  one static cutout, bitmap `Elev` label and separator, with explicit endpoint,
  raster and selection-loss rules, including retaining the separator without
  selection. User approved the contract; implementation is a separate step.
- [x] Implement 0041, preserving radar/notices and square diagnostics. Exact
  raster/lifecycle/real-data snapshot tests pass; pre/post captures preserve all
  radar pixels and square output. No other glyphs or animation were added.
- [x] User reviewed and accepted 0041's fresh executable output, separately
  from the earlier illustrative comparisons and contract approval.
- [x] Agree [0042's static altitude contract](decisions/0042-static-altitude-glyph-contract.md):
  fixed compressed WGS-84 altitude scale, concentric outlines, bitmap `Alt` label,
  exact raster and static lifecycle. User approved the illustrative comparison
  and behavior rules; recording the contract does not authorize implementation.
- [x] Separately authorize and implement 0042 alongside elevation, preserving
  radar, notices, square diagnostics and physical/app behavior. Exact-pixel,
  mapping, lifecycle and independent real-data altitude tests pass; pre/post
  captures preserve all pixels outside the new altitude slot and square bytes.
- [x] User reviewed and accepted fresh executable altitude snapshots separately
  from the contract mocks.
- [x] Agree [0043's static speed contract](decisions/0043-static-speed-glyph-contract.md):
  instantaneous TEME velocity magnitude, fixed linear 0–12 km/s clockwise arc,
  bitmap `Spd`, zero/invalid distinction and immediate three-field lifecycle.
  The user approved the illustrative comparison and remaining behavior rules.
- [x] Authorize and implement 0043, retaining true speed in app state and adding
  the wide-only arc. Mapping/raster/lifecycle and independent headless tests
  cover the field; existing radar, notices, elevation/altitude and square output
  are preserved. No new dependency or tracking/selection policy.
- [x] User reviewed and accepted fresh executable speed output separately from
  the mocks.
- [x] Agree [0044's static period contract](decisions/0044-static-period-glyph-contract.md):
  nominal element-set period, fixed logarithmic 90-minute/2½-turn to 24-hour/
  one-turn mapping, bitmap `Per`, field-local invalid suppression and immediate
  four-field lifecycle. The user selected 2½ turns after comparing against three.
- [x] Authorize recording the exact raster and implementing 0044. The app reads
  existing selected-element metadata without another propagation or retained
  per-sample scalar; wide rendering adds only the period slot. Mapping, pixel,
  lifecycle and independent real-data snapshot tests cover the new field.
- [x] User reviewed and accepted fresh executable period output, separately from
  the approved illustrative appearance and scale. No target-cost or physical-display
  validation follows.
- [x] Agree and implement [0045's static group-derived category](decisions/0045-static-group-category-contract.md):
  explicit optional group roles, Station > Navigation > Weather > Unknown,
  approved Cat/icons and immediate five-field lifecycle. No per-ID mapping,
  inferred roles, operational-status claim or orbital-data change.
- [x] User reviewed and accepted fresh executable category output, separately
  from approved icons/assignment/lifecycle. [Review recipe](../sim/README.md#static-category-glyph).
- [x] Agree and implement [0046's selected-data transitions](decisions/0046-selected-data-glyph-transition-contract.md):
  four animated glyphs on selected-ID changes, immediate Cat, 250 ms smoothstep,
  explicit presentation time and bounded retarget/loss behavior. The user
  separately accepted interactive output; headless snapshots stay static.
- [x] Agree [0047's text-first debug access](decisions/0047-text-first-debug-access-contract.md):
  radar-free tracking/data-health page, interactive wide-view `v` toggle and
  explicit `--wide --debug` start. Default square baseline retained.
- [x] User reviewed and approved the [debug layout/overflow treatment](mockups/0047-debug-layout-proposal.md),
  closing 0047's preimplementation renderer-review gate. The six pages remain
  illustrative references, not executable output or implementation authorization.
- [x] Separately authorize and implement 0047, preserving current app truth,
  wide normal-view notices and historical snapshot behavior. Tests cover access,
  bounded text/values, state isolation and static return without queued motion.
- [x] User reviewed and accepted fresh executable debug output under 0047,
  separately from the approved illustrative pages and automated checks.
  [Review recipe](../sim/README.md#debug-access). No scene displacement or collision
  search is approved. Circular-radar/field/graph alternatives remain exploration;
  neither mocks nor replay populations select production capacity.

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
- [ ] Refine secondary information with visual feedback and choose any further
  true-state readouts separately. 0032 removes routine normal-view FOV/counts;
  time and data age are not starting-layout requirements. The [range comparison](mockups/0050-range-vs-no-readout.md)
  ended with range remaining in debug under [0050](decisions/0050-keep-range-in-debug-and-defer-display-prediction.md);
  no additional normal-view readout was approved.
- [ ] Refine empty-sky behavior without adding display prediction for now;
  next-pass countdown study/integration is deferred under 0050, not cancelled
  or substituted for [0048's notices](decisions/0048-refined-empty-view-notices.md).
- [ ] Prepare coastlines offline and render them legibly across zoom levels.
  [0051's reviewed angular appearance direction](decisions/0051-angular-coastline-appearance-direction.md)
  and [native 1-bit comparisons](mockups/0054-coastline-overview-comparison.md)
  cover 5°–100°. [0054](decisions/0054-first-executable-coastline-comparison.md)
  now authorizes a bounded, executable **wide-only comparison** using the reviewed
  40 km look and an explicit exceptional `COAST ERROR` notice; its host export,
  renderer and tests exist. The user reviewed the fresh executable images and
  simulator, accepting the appearance. A follow-up host suite now compares
  coast/no-coast selected and empty/notice frames and probes dense visible samples
  on the checked asset at SF, polar and dateline sites, plus synthetic limb,
  viewport and run-boundary cases. A further host test isolates exported segments
  in those scenes and at computed limb/viewport-edge observer positions; it found
  no qualifying gaps. Short runs, unsampled excursions and no-ordinary-omission
  proof remain open. A [selected-segment bounded host study](evaluations/m4-coastline-bounded-segment-study.md)
  tests segment 46 and four short viewport-edge runs (2–7 reference cells)
  independently of the renderer's fixed sampling. A further finite-scene control
  checks **each of the 1,734 segments** at seven scenes, conservatively covering
  every unknown interval's possible screen cells with isolated ink within 3px.
  No qualifying gap was found. Unknown visibility is not called invisible; not
  all observers are covered and floating-point margins lack formal bounds, so
  0053 is not closed. A
  [host-only cost preflight](evaluations/m4-coastline-host-cost-preflight.md)
  now times coast/full-frame controls and counts raster writes on reviewed and
  boundary scenes; it does not measure S3 cost, late-failure cost or compiled size.
  A separate host regression injects a coastline-only failure after confirmed ink
  and checks rollback, foreground and notice pixels in selected/empty warning
  scenes. Natural late numeric failures and their target cost remain unmeasured.
  Target resource measurement and physical-display acceptance remain open. The
  [M2-informed ceilings](evaluations/m4-coastline-budget-preflight.md) are
  **provisional measurement gates**, not proven capacity or shipping approval.

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
