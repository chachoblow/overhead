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
- [x] Validate minimal Sharp output/refresh before full firmware integration:
  the opt-in [wiring/refresh test](../firmware/README.md#isolated-sharp-wiringrefresh-test)
  booted on the S3 and the user confirmed the alternating pattern on the physical
  #4694 panel. This is not radar integration or whole-device cadence evidence.

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
  no qualifying gaps. A [selected-segment bounded host study](evaluations/m4-coastline-bounded-segment-study.md)
  tests segment 46 and four short viewport-edge runs (2–7 reference cells)
  independently of the renderer's fixed sampling. A further finite-scene control
  checks **each of the 1,734 segments** at seven scenes, conservatively covering
  every unknown interval's possible screen cells with isolated ink within 3px.
  No qualifying gap was found. Unknown visibility is not called invisible; not
  all observers are covered and floating-point margins lack formal bounds.
  [0055](decisions/0055-defer-all-view-coastline-proof-for-m4.md) stops the
  all-view proof effort as a **gate for this M4 iteration**, retaining 0053's
  no-known-ordinary-gap intent and a later completeness TODO; it does not
  certify all observers or close device acceptance. A
  [host-only cost preflight](evaluations/m4-coastline-host-cost-preflight.md)
  now times coast/full-frame controls and counts raster writes on reviewed and
  boundary scenes; it does not measure S3 cost, late-failure cost or compiled size.
  A separate host regression injects a coastline-only failure after confirmed ink
  and checks rollback, foreground and notice pixels in selected/empty warning
  scenes. Natural late numeric failures and their target cost remain unmeasured.
  Target resource measurement and physical-display acceptance remain open. An
  [Stage A measurement harness and S3 capture](evaluations/m4-coastline-device-measurement-protocol.md)
  now have separately authorized, exact-gated coast-on/off release images and a
  1-bit RAM-target capture. The clean coast-off code/asset comparison is
  65,020 B of `.rodata` + `.text` (516 B below the provisional 64 KiB screen),
  but the maximum observed complete coast rebuild was **9,781,141 µs**, far
  above the provisional **5 ms** gate. The current candidate therefore fails
  target-time screening; no Sharp or whole-device validation follows. The
  [M2-informed ceilings](evaluations/m4-coastline-budget-preflight.md) remain
  **provisional measurement gates**, not proven capacity or shipping approval.
  [0056](decisions/0056-bounded-coastline-architecture-trial.md) now limits further
  investigation to one host representation trial and, if promising, at most one
  separately authorized target feasibility check before choosing the architecture.
  The [completed host trial](evaluations/m4-coastline-representation-spike.md) uses
  a 32,211 B quadratic ECEF representation and screen-scale refinement; it improves
  all eleven host scenes ~36–65×, with bidirectional isolated raster differences
  within 1px in the tested controls. The user accepted its appearance and separately
  authorized the [single target feasibility check](evaluations/m4-coastline-representation-target.md),
  now complete: **22,489 µs** maximum coast rebuild (~435× faster than before, still
  above 5 ms); **58,156 B** compiled increment fits, but candidate-specific nested
  stack frames alone exceed 1 KiB. All 48 captured framebuffers match the host and
  measured draws allocate no heap. **Not adopted for the device.** Stop general
  renderer optimization under 0056 and scope set-location/preparation/reuse for
  different users. [0057](decisions/0057-fixed-observer-coastline-preparation-scope.md)
  accepts a fixed observer during use for this version, without excluding future
  globe scrolling/panning. A [preparation/reuse proposal](evaluations/m4-coastline-location-preparation-scope.md)
  records the user's acceptance of a visible one-time setup phase, observer-space
  geometry as the first host feasibility path and a full loading screen (no old
  coastline) during location preparation—not a cache implementation or format
  contract. A [host-only uncompressed preparation trial](evaluations/m4-coastline-observer-preparation-host.md)
  matches 55 candidate rasters but retains 37,000–73,864 B per observer; a
  second host-only 16-bit trial retains 15,240–24,996 B but differs by up to 1px
  on those masks. A further host-only heap probe found 51,984–98,604 B peak
  logical requested heap while both forms coexist, with a conservative realloc
  envelope up to 110,592 B; no new target or live-stack evidence follows. Screen details,
  lifecycle, zoom and evidence details remain proposals; configuration, setup
  duration, retained memory and revised
  resource policy remain open. The first firmware prototype still follows 0004
  until a successor contract is chosen. [0058](decisions/0058-prepared-wide-coastline-slice.md)
  now implements the host/simulator slice: the wide view draws the accepted
  quadratic coastline from a heap-free fixed-observer layer prepared once
  behind a provisional full-screen loading frame, reused across satellite motion
  and zoom, exactly matching the candidate rasters tested. Host draw savings are
  modest; [0059](decisions/0059-current-view-coastline-raster-cache.md) therefore
  caches the current view's 10,560 B coastline raster, re-rastering only on zoom
  (layer 10,960 B). The approved [S3 measurement](evaluations/m4-coastline-prepared-target.md)
  gives cached coast 386–1,153 µs (passes 5 ms), zoom-step rasters up to 21 ms,
  ~5 ms preparation and zero heap, matching the candidate exactly.
  [0060](decisions/0060-defer-timing-to-physical-prototype.md) accepts this for
  now and defers timing work, including non-coast frame cost, to the physical
  prototype.
  Selected non-coast frame cost and Sharp/whole-device cadence remain separate
  open blockers.

**Done when:** the view is useful and understandable during passes and empty sky.

## M5 — Motion and visual refinement
- [ ] Smooth motion between calculation updates; add configurable minimum screen
  traversal time with explicit zoom/selection-change behavior.
- [ ] Keep presentation state separate; verify readouts use true positions.
- [ ] Tune density, labels, transitions, and rendering performance.

**Done when:** fast passes are watchable without contaminating physical state.

## M6 — Standalone device
- [x] First bounded physical integration: a fixed-time, single real-ISS
  [radar frame](../firmware/README.md#fixed-fixture-radar-on-sharp-candidate)
  from the shared app/renderer reached the Sharp panel; the user confirmed the
  ISS marker and labelled strip. This does not establish motion, live inputs,
  responsiveness or device cadence.
- [x] A deliberately accelerated [deterministic ISS replay](../firmware/README.md#deterministic-radar-replay-v2-physically-accepted)
  repaints the shared frame on the S3; the user confirmed visible movement and
  the repeating reset on the physical panel. The first, too-short replay looked
  static and was not accepted. This verifies motion output, not a live clock,
  natural-speed orbit, 20 Hz cadence or sustained operation.
  A separate [host-only timing preflight](../firmware/README.md#utcframe-scheduling-preflight-host-only)
  now tests trusted-UTC gating, monotonic frame deadlines/overrun skips and idle
  VCOM. It is not integrated on the board; the uncached replay's ~196 ms
  update/render/transfer exceeded the 50 ms target frame interval. A separate [cost-profile diagnostic](../firmware/README.md#sharp-full-frame-cost-diagnostic-flashed-short-uart-capture)
  was approved, gated, flashed and UART-captured on the S3; the user confirmed
  that its moving replay remains visible on the panel. The ~141 ms full
  render contrasts with offscreen controls of ~13.7 ms full clear, ~1.1 ms cached
  coast (including fast RAM clear) and ~138 ms full coast-free frame; controls
  are not additive. The [host-tested stage trace](../firmware/README.md#stage-trace-follow-up-flashed-short-uart-capture)
  was approved, offline-gated, flashed and UART-captured. In the ~138 ms
  offscreen coast-free draw, ~13.8 ms is clear, ~6.1 ms Earth and **~116.2 ms
  data glyphs**. [Individual-glyph v3](../firmware/README.md#individual-glyph-trace-v3-flashed-short-uart-capture)
  was host-tested, approved, offline-gated, flashed and UART-captured. The
  offscreen strip's ~116.5 ms includes **~115.02 ms in the period field** (label,
  4,097 samples and raster). An opt-in host-tested renderer period cache now
  reuses exactly the same raster for an unchanged mapped turn count; ordinary
  replay firmware and simulator still use the uncached entry points. A [separate cache
  comparison image](../firmware/README.md#period-cache-comparison-v1-flashed-short-uart-capture)
  was approved, gated, flashed and short-captured. It sends cached frames and
  compares each to an uncached offscreen byte oracle; all captured buffers matched.
  Warm cached render ~26 ms versus ~141 ms separate uncached draw; full-frame
  2 MHz transfer still takes ~52.4 ms and update/render/transfer ~82 ms. The
  user confirmed moving replay on v2 and v3 and that the cache comparison's
  moving replay still looks correct on the panel. At the datasheet's 2 MHz maximum,
  a full frame has a ~49.9 ms wire-time floor before overhead. A separate
  [host-only line-transfer preflight](../firmware/README.md#sharp-line-transfer-feasibility-host-only)
  reconstructs exact pixels across three fixture loops with 18–34 changed lines
  per transition. A [separate selective-line diagnostic](../firmware/README.md#selective-line-target-diagnostic-flashed-short-uart-capture-user-visual-check)
  now has host pixel-parity/error-resync tests, gated target image and approved
  flash. Its short UART capture reports ~5.4–8.8 ms changed-line transfers and
  ~52.4 ms periodic full transfers. Asked about movement, reset and stale rows,
  the user reported the display looks good. Sustained service remains open;
  the diagnostic ticks at 500 ms, not 20 Hz.
  A separate [natural-speed historical fixture diagnostic](../firmware/README.md#natural-speed-historical-diagnostic-flashed-short-uart-capture)
  now wires host-preflight frame/refresh policy to the cached renderer and
  selective sender at a 50 ms target grid. Its offline-gated S3 image received
  an approved flash and physical visual acceptance. A short capture found three
  initial full-frame overrun skips; an 85-second UART-only follow-up logged ten
  ~8-second retained full refreshes without reported panic or additional skips.
  A separate 315-second UART-only run crossed the 300-second historical fixture
  wrap and logged 38 full refresh completions with **four further skipped frame
  deadlines immediately after full refreshes**; it reported no panic. This
  remains fixture time, **not** trusted UTC, network/controls, per-frame timing
  proof, sustained VCOM verification, or production 20 Hz capacity. A separately
  [host-tested and offline-gated v2 command audit](../firmware/README.md#natural-speed-command-audit-v2-flashed-bounded-uart-capture)
  was separately approved, flashed and captured for ~115 seconds. It reported
  one over-50-ms frame (startup), four skipped deadlines (three startup, one
  after retained refresh), 14 retained full transfers of ~51.7 ms, and
  alternating issued VCOM bits across completed commands. This is not physical
  COM-edge or production 20 Hz verification. The user separately confirmed v2
  looks stable, with gradual motion and no visible flicker or stale rows.
- [x] [0062's host-only no-trusted-time preflight](decisions/0062-no-trusted-time-display-preflight.md):
  explicit app-frame invalidation and `NO FRAME` replacement on boot/time loss,
  with tested full-send confirmation, idle COM/retained-refresh independence and
  checked UTC representation, monotonic/overflow fault rejection and new-anchor
  recovery. That 0062 preflight alone supplies no external time source or device loop.
  [0063](decisions/0063-bounded-utc-holdover-direction.md) accepts bounded
  holdover between connections, a desired ≤100 ms total UTC-error budget and
  immediate fresh-frame reanchoring on a source-validated time jump, and
  screened ordinary unicast SNTP for the first prototype (not authenticated).
  A separately approved [host-only trust gate](../firmware/src/utc_trust.rs)
  composes 0062's preflight with injected screened observations and explicit
  test uncertainty/drift/holdover inputs; by itself it is not an SNTP client
  or device policy. [0064](decisions/0064-prototype-sntp-screen-and-leap-boundary.md)
  adds a pure host [SNTP reply screen](../firmware/src/sntp_screen.rs) (unique-era,
  status, timing/uncertainty) and ends holdover before any UTC month end
  (possible leap). Bracketing send/receive captures reduce `C` to resolution
  plus drift. A 3-hour UART capture measured the S3 timer at ≈+1.2 ppm versus
  UTC (one board, room temperature, host-reference-limited);
  [0065](decisions/0065-prototype-drift-bound-and-holdover.md) sets prototype
  `D = 20 ppm` and `H = 3,600 s`. [0066](decisions/0066-prototype-live-sntp-stack-and-source.md)
  adds an opt-in Wi-Fi/UDP stack and Cloudflare/NIST source; a one-hour
  [live run](../firmware/README.md#live-sntp-source-evidence-flashed-1-hour-uart-run)
  accepted all 59 replies (`U0` median 28 ms) with every consistency check in
  bound. The remaining device-loop validation was outlined in the
  [proposal](utc-trust-loss-proposal.md#remaining-evidence-capture-bound-drift-and-stack-proposal).
  Under [0067's bounded fixture-only integration scope](decisions/0067-opt-in-live-utc-iss-radar-scope.md),
  a [host-only composition preflight](../firmware/README.md#live-utc-iss-composition-preflight-host-only)
  now checks screened replies through trust, frame rendering and Sharp command
  intent using the checked-in ISS elements. A separate
  [live-UTC radar candidate](../firmware/README.md#live-utc-iss-radar-candidate-flashed-bounded-uart-capture)
  has an independently polled display/SPI task and a bounded network-event
  channel. Its separately approved, exact-gated local-credential image was
  flashed; a bounded 90-second UART capture logged cold-boot `NO FRAME`, two
  accepted live syncs, frame activity and retained transfers without a reported
  panic. The user reported a visible empty-sky notice on the Sharp panel;
  the board later switched to a [separate dated-OMM diagnostic](../firmware/README.md#pinned-omm-live-utc-marker-diagnostic-flashed-no-visual-observation).
  That image has host marker-motion and bounded UART evidence; the user was
  not watching during the original predicted window. On 2026-10-29 PDT, the
  user separately observed ISS moving on the Sharp panel during a later modelled
  fixed-observer window, then leaving as the empty-horizon notice returned.
  This is visual evidence, not independently verified clock/image identity,
  actual-location accuracy, physical COM edges or sustained cadence.
  Wi-Fi/display contention, SPI failure service, aging elements and whole-device
  timing remain open.
- [ ] Run shared engine/renderer on S3 + Sharp. Intended controls under 0060:
  button cycles satellites, knob zooms continuously (presets as fallback).
- [ ] Add Wi-Fi credentials, HTTPS elements, SNTP, persistence, and refreshes
  per [0002](decisions/0002-data-and-time-over-wifi.md).
- [ ] Handle network failure, missing/invalid time or data, and stale elements.
- [ ] Verify end-to-end performance and extended hardware operation, including
  the frame-timing work deferred by 0060.

**Done when:** the prototype boots with valid time/data and tracks between
connections. Cached elements alone do not enable accurate offline cold boot.

## Boundaries
Keep tuning inputs explicit; they need not be device settings. Defer uncertain
presentation policies rather than building a plugin system. Knob, enclosure,
battery integration, RTC, real location source, and Starlink/full catalogue
remain outside this roadmap.
