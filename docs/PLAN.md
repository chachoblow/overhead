# Overhead — Plan

**Current: M0, M1, M2 and M3 complete (M2 closed under
[0077](decisions/0077-m2-operating-budget-visual-group.md) on 2026-11-03); M4
done for now (user, 2026-10-24); M6 standalone device complete (2026-11-14,
device inputs under [0088](decisions/0088-selection-button-and-captured-knob-switch.md)); M6 element
fetch runs under [0071](decisions/0071-https-element-fetch-first-slice.md),
[0074](decisions/0074-live-utc-radar-on-fetched-elements.md) and
[0075](decisions/0075-fetched-element-refresh-and-staleness.md); the device
app build is [0076](decisions/0076-device-app-build.md). M7 giftable device
(accepted 2026-11-08, [0083–0087](decisions/0083-flash-layout-for-updatable-devices.md)) is in progress: OTA, the BLE setup service and the iOS app done; gift readiness paused for M8 radar revision ([0089](decisions/0089-full-width-radar-revision.md)), current.**
Engine-first correctness remains the foundation
([0005](decisions/0005-engine-first-implementation.md)); visual iteration now informs
remaining product budgets under [0021](decisions/0021-radar-first-iteration-on-verified-engine.md).
Checkboxes are scope; work is taken on as outcome-sized work items under
[0072](decisions/0072-outcome-sized-work-items.md), tracked in `docs/work/` while active.
Unless noted, each interface slice below was contract-approved, separately
authorized, implemented, and then accepted by the user on fresh executable
output, separately from mocks and host checks.

## Complete
Finished milestones are summarized here with links; their checklists, exit
assessments and evidence are in [PROGRESS](PROGRESS.md), the decisions and the
linked evaluations (full checklists: `git show 95ee8b0:docs/PLAN.md`).

- **M0 — Setup:** workspace, simulator window, project workflow.
- **M1 — Verified tracking foundation:** OMM ingestion, timestamped SGP4,
  Earth-fixed/geodetic and observer geometry, a reproducible headless runner,
  then pre-M2 hardening. Conventions: [0006–0010](decisions/0006-sgp4-crate-and-conventions.md);
  provenance: [fixture README](../core/tests/fixtures/README.md).
- **Early hardware checks:** S3 propagation/prediction baseline
  ([s3-costs](evaluations/s3-costs.md)) and the isolated Sharp
  [wiring/refresh test](evaluations/firmware-diagnostics.md#isolated-sharp-wiringrefresh-test).

## M2 — Catalogue, pass prediction, and operating budget
Complete; closed under [0077](decisions/0077-m2-operating-budget-visual-group.md)
on 2026-11-03.
- Catalogue configuration, merge and provenance ([0011](decisions/0011-catalogue-ingestion-and-provenance.md),
  [tool usage](../tools/README.md#overhead-catalogue)).
- Allocation-free physical-pass search ([0012](decisions/0012-physical-pass-semantics.md)–[0016](decisions/0016-catalogue-pass-aggregation.md)),
  host-tested; device pass prediction is not planned (see Boundaries).
- Cost and memory evidence on host and S3, the DROM remedy and
  [offline image gate](../firmware/IMAGE_GATE.md) ([0018](decisions/0018-source-level-drom-remedy-and-image-gate.md)),
  and the frozen 20-case capture ([0019](decisions/0019-bounded-m2-evidence-preparation.md),
  [0020](decisions/0020-m2-capture-manifest-freeze.md), [results](evaluations/s3-m2.md)).
  Reproduce only at the [pinned revision](evaluations/s3-m2/README.md#offline-validation-and-table-regeneration).
- Operating budget (0077): CelesTrak's `visual` group, 128 KiB bodies in PSRAM,
  at most 256 objects, the selected object every frame and the rest within 1 s,
  verified on the board ([evidence](../firmware/README.md#visual-group-under-the-m2-budget-0077)).
- Still open: the linker's separate RWX segment warning.

## M3 — Minimal real-data radar (complete)
Exit assessment 2026-10-18 (in PROGRESS). Shared clockless app state, 1-bit
renderer and explicit-time replay ([0022](decisions/0022-minimal-ground-map-simulator.md));
the fixed-camera perspective with lens zoom ([0026](decisions/0026-fixed-camera-perspective-slice.md),
[0027](decisions/0027-higher-fixed-camera-overview.md), [0028](decisions/0028-borderless-square-viewport.md));
the wide radar ([0032](decisions/0032-wide-radar-and-selected-satellite-glyphs.md),
[0033](decisions/0033-wide-radar-comparison-contract.md)); bottom-left notices
([0039](decisions/0039-bottom-left-condition-driven-notices.md)/[0040](decisions/0040-white-backed-bottom-left-notice-contract.md));
and text-first debug access ([0047](decisions/0047-text-first-debug-access-contract.md)).
The ID label (0034/0035) and glyph strip (0036–0046) built here were retired by
[0089](decisions/0089-full-width-radar-revision.md).

## M4 — Useful display behavior
Done for now (user, 2026-10-24).
- Empty-view notices ([0048](decisions/0048-refined-empty-view-notices.md)),
  stable wide selection ([0049](decisions/0049-stable-wide-selection.md)) and
  filled arrows with `NONE IN VIEW` ([0061](decisions/0061-filled-arrows-id-backing-none-in-view.md)),
  each implemented and accepted by the user.
- Range stays out of the normal view and display prediction (including a
  next-pass countdown) is deferred, not cancelled ([0050](decisions/0050-keep-range-in-debug-and-defer-display-prediction.md));
  0089 later put range in a pill.
- Coastlines: angular appearance ([0051](decisions/0051-angular-coastline-appearance-direction.md)),
  Natural Earth source ([0052](decisions/0052-natural-earth-coastline-candidate.md)),
  the all-view completeness proof deferred ([0055](decisions/0055-defer-all-view-coastline-proof-for-m4.md)),
  and the adopted fixed-observer prepared layer with a current-view raster cache
  ([0057](decisions/0057-fixed-observer-coastline-preparation-scope.md)–[0059](decisions/0059-current-view-coastline-raster-cache.md),
  [S3 measurement](evaluations/m4-coastline-prepared-target.md)); timing work
  deferred to the prototype ([0060](decisions/0060-defer-timing-to-physical-prototype.md)).
- Still open from M4: a completeness TODO for coastlines (0055: not all
  observers covered, unknown visibility not called invisible, floating-point
  margins lack formal bounds) and the setup-phase duration and retained memory
  for preparing coastline at a recipient's location. Location itself is now a
  stored setting from the setup app ([0084](decisions/0084-settings-store.md),
  [0087](decisions/0087-ios-setup-app-over-ble.md), superseding 0004).

## M5 — Motion and visual refinement
- [ ] Smooth motion between calculation updates; add configurable minimum screen
  traversal time with explicit zoom/selection-change behavior.
- [ ] Keep presentation state separate; verify readouts use true positions.
- [ ] Tune density, labels, transitions, and rendering performance.

**Done when:** fast passes are watchable without contaminating physical state.

## M6 — Standalone device
- [x] Physical display diagnostics, each separately gated, flashed and captured;
  details and limits live in the [firmware README](../firmware/README.md):
  - First integration: a fixed-time, single real-ISS [radar frame](evaluations/firmware-diagnostics.md#fixed-fixture-radar-on-sharp-candidate)
    from the shared app/renderer reached the Sharp panel; the user confirmed the
    marker and labelled strip.
  - Motion: an accelerated [deterministic ISS replay](evaluations/firmware-diagnostics.md#deterministic-radar-replay-v2-physically-accepted)
    was visually accepted after a first, too-short replay looked static.
  - Frame cost: the [cost profile](evaluations/firmware-diagnostics.md#sharp-full-frame-cost-diagnostic-flashed-short-uart-capture),
    [stage trace](evaluations/firmware-diagnostics.md#stage-trace-follow-up-flashed-short-uart-capture)
    and [individual-glyph v3](evaluations/firmware-diagnostics.md#individual-glyph-trace-v3-flashed-short-uart-capture)
    traced ~115 ms of a ~141 ms render to the period glyph. An opt-in renderer
    [period cache](evaluations/firmware-diagnostics.md#period-cache-comparison-v1-flashed-short-uart-capture)
    gave ~26 ms warm renders, byte-identical to uncached output. The glyph
    strip and the cache were retired by 0089; build them from `4ac3778`.
  - Transfer: a full 2 MHz frame takes ~52.4 ms (~49.9 ms wire floor). The
    [selective-line diagnostic](evaluations/firmware-diagnostics.md#selective-line-target-diagnostic-flashed-short-uart-capture-user-visual-check)
    sends changed lines in ~5.4–8.8 ms with host pixel parity; the user reported
    the display looks good.
  - Cadence: the [natural-speed historical diagnostic](evaluations/firmware-diagnostics.md#natural-speed-historical-diagnostic-flashed-short-uart-capture)
    and [v2 command audit](evaluations/firmware-diagnostics.md#natural-speed-command-audit-v2-flashed-bounded-uart-capture)
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
    [live run](evaluations/firmware-diagnostics.md#live-sntp-source-evidence-flashed-1-hour-uart-run)
    accepted all 59 replies. Renewals within 100 ms keep tracking state
    ([0063 note](decisions/0063-bounded-utc-holdover-direction.md#note--2026-11-03-small-resyncs-keep-tracking),
    [evidence](../firmware/README.md#small-sntp-resyncs-keep-tracking-0063-note)).
    Remaining device-loop evidence is outlined in the
    [proposal](utc-trust-loss-proposal.md#remaining-evidence-capture-bound-drift-and-stack-proposal).
  - [0067](decisions/0067-opt-in-live-utc-iss-radar-scope.md): the
    [live-UTC radar candidate](evaluations/firmware-diagnostics.md#live-utc-iss-radar-candidate-flashed-bounded-uart-capture)
    and the [dated-OMM diagnostic](evaluations/firmware-diagnostics.md#pinned-omm-live-utc-marker-diagnostic-flashed-no-visual-observation).
    On 2026-10-29 PDT the user watched ISS move across the Sharp panel during a
    modelled fixed-observer window, then saw the empty-horizon notice return.
    This is visual evidence, not verified clock/image identity, actual-location
    accuracy, physical COM edges or sustained cadence.
  - Wi-Fi/display contention, SPI failure service, aging elements and
    whole-device timing remain open.
- [x] Run shared engine/renderer on S3 + Sharp. Intended controls under 0060:
  button cycles satellites, knob zooms continuously (presets as fallback).
  BOOT cycling under [0068](decisions/0068-boot-selection-gesture.md) with
  [0070](decisions/0070-boot-input-capture-independent-of-render.md) interrupt
  capture passed user tap sessions on the diagnostics; since
  [0088](decisions/0088-selection-button-and-captured-knob-switch.md) the
  device selects with a button on IO7 (gift units don't expose BOOT) and the
  knob switch is captured the same way (user tap session 2026-11-14,
  [evidence](../firmware/README.md#selection-button-and-captured-knob-switch-0088));
  [HANDOFF](HANDOFF.md) tracks the input limits. The device app build exists
  under [0076](decisions/0076-device-app-build.md) (`--features device`,
  shared runtime with the `live-fetch-radar` diagnostic, event log only;
  [evidence](../firmware/README.md#device-application-0076)). Knob zoom done
  ([0060 note](decisions/0060-defer-timing-to-physical-prototype.md#note--2026-11-03-knob-implemented-and-accepted)).
- [x] Add Wi-Fi credentials, HTTPS elements, SNTP, persistence, and refreshes
  per [0002](decisions/0002-data-and-time-over-wifi.md). First slice accepted as
  [0071](decisions/0071-https-element-fetch-first-slice.md): mbedtls-rs, STATIONS JSON,
  one request per boot; slices 1–3 done: one on-board fetch ingested 20/20 STATIONS records
  ([evidence](evaluations/firmware-diagnostics.md#https-element-fetch-on-the-board-flashed-one-boot-uart-capture));
  slice 4 done under [0074](decisions/0074-live-utc-radar-on-fetched-elements.md): the
  `live-fetch-radar` diagnostic checks certificate dates against screened UTC and tracks
  the fetched STATIONS on the live-UTC radar (user-observed pass with BOOT cycling,
  [evidence](evaluations/firmware-diagnostics.md#live-utc-radar-on-fetched-stations-flashed-uart-captures)).
  Refresh done under [0075](decisions/0075-fetched-element-refresh-and-staleness.md):
  8-hour refresh with request-boundary backoff and a build-time home site;
  seven on-board refreshes with no heap growth and a user-observed pass
  ([evidence](evaluations/firmware-diagnostics.md#refresh-staleness-and-home-site-0075)).
  Persistence done under [0079](decisions/0079-fetched-element-persistence.md):
  the last fetched body in a two-slot `elements` partition, loaded at boot; a
  copy younger than the interval defers the fetch; corrupt-slot fallback and
  reset-without-fetch shown on the board, user-observed on the device app
  ([evidence](../firmware/README.md#element-persistence-0079)).
  Credentials and the location source moved to M7 and are done there
  ([0084](decisions/0084-settings-store.md),
  [0087](decisions/0087-ios-setup-app-over-ble.md)).
- [x] Handle network failure, missing/invalid time or data, and stale elements.
  Under 0075: failed refreshes keep the previous catalogue, `OLD DATA` after
  24 h, unload after 7 days. Shown on the board with the opt-in
  `live-fetch-faults` build: DNS, TCP, handshake and verification failures
  classified and scheduled as decided, `OLD DATA` raised and cleared by a load,
  expiry to `NO DATA`, Wi-Fi drop recovered without reboot
  ([evidence](evaluations/firmware-diagnostics.md#fetch-failure-paths-on-the-board-live-fetch-faults-0075)).
  Post-GET failures, the repeated-handshake rule and reload after expiry
  remain host-tested only.
  SNTP Kiss-o'-Death backs off per server and recovers without reboot, and
  refreshes run on uptime with holdover-dated UTC
  ([0080](decisions/0080-sntp-kod-backoff-and-fetch-independence.md),
  [evidence](../firmware/README.md#sntp-kod-recovery-0080)). The NIST
  fallback renews UTC (VN=3 accepted) and a looser reply no longer replaces a
  tighter anchor; ~3 h NIST-only on the board with no `NO FRAME`
  ([0081](decisions/0081-sntp-accept-version-3-replies.md),
  [0082](decisions/0082-keep-the-tighter-utc-anchor.md),
  [evidence](../firmware/README.md#nist-fallback-renews-utc-0081)).
- [x] Verify end-to-end performance and extended hardware operation, including
  the frame-timing work deferred by 0060: a [24-hour soak](../firmware/README.md#extended-operation-and-frame-cadence-24-h-soak)
  held 20 Hz for 99.3% of deadlines with boot + 3 refreshes, no heap growth and a
  user feel pass; selection/knob/SNTP frame hitches accepted as limits
  ([0060 note](decisions/0060-defer-timing-to-physical-prototype.md#note--2026-11-05-24-hour-soak-limits-accepted)).

**Done when:** the prototype boots with valid time/data and tracks between
connections. Cached elements alone do not enable accurate offline cold boot.

## M7 — Giftable device
A handful of units given as gifts, set up by recipients on an iPhone and
updated over the air afterwards ([proposal and research](giftable-device-proposal.md)).
- [x] **Signed OTA with rollback on the user's board:**
  [0083](decisions/0083-flash-layout-for-updatable-devices.md) layout and
  `dram2` heap, [0086](decisions/0086-bootloader-rollback-and-health-check.md)
  custom bootloader (rootless Docker, `espressif/idf` v6.1), reset on panic,
  watchdog and health check, [0085](decisions/0085-signed-updates-and-release-keys.md)
  manifest, `overhead-release` signing tool (key made by the user on the Mac),
  Pages hosting, and the brick drill. Ends with the board updating itself
  from `beta`. Done 2026-11-08: all six drills passed; the board installed
  user-signed release 12 from `beta` on raw GitHub
  ([evidence](../firmware/README.md#signed-ota-and-rollback-00830086)). The
  0084 Wi-Fi/site settings and release images without personal data were
  pulled in here; `overhead.wesklein.com` went live 2026-11-09.
- [x] **BLE setup service (firmware half of item 2; split agreed with the
  user 2026-11-08):** [0087](decisions/0087-ios-setup-app-over-ble.md)'s
  setup window, passkey pairing, versioned protocol (`setup/`,
  `overhead-setup`), factory reset and info page, driven from devbox by
  `overhead-setup-client`. Done 2026-11-09: set up from empty settings,
  join-timeout and 3 s re-entry, factory reset, wrong passphrase, nRF Connect
  pairing on the user's iPhone (no bond needed), and user-signed release 13
  installed from `beta` with the BLE-saved settings intact
  ([evidence](../firmware/README.md#ble-setup-service-0087)).
- [x] **iOS setup app:** the app in `ios/` (SwiftUI + CoreBluetooth) on
  TestFlight for the user, written against the
  [0087 protocol note](decisions/0087-ios-setup-app-over-ble.md#note--2026-11-09-setup-protocol-and-firmware)
  and `setup/`'s byte vectors. Done 2026-11-10: the user set the board up
  only through the app (TestFlight 0.1.0 (1)), then it installed user-signed
  release 14 from `beta` with those settings intact
  ([0087 app note](decisions/0087-ios-setup-app-over-ble.md#note--2026-11-10-the-ios-app),
  [evidence](../firmware/README.md#ble-setup-service-0087)).
- [ ] **Gift readiness:** the app on the App Store, a soak of a release image
  set up only through the app, setup on an iPhone that isn't the user's, the
  release and recovery checklist (key backup, USB recovery flash), and
  flashing the gift units. Soak and release path done 2026-11-14: 72 h on
  app-saved settings with no `NO FRAME` or unexpected reset; release 15
  installed mid-soak and is the first `stable` release; checklist exercised,
  recovery flash tried
  ([evidence](../firmware/README.md#release-and-recovery-checklist)).
  Remaining: App Store, another iPhone, gift units.

**Done when:** a device set up only through the iOS app on an iPhone that
isn't the user's tracks at its recipient's location and receives a signed
`stable` release with no hands-on step, and the brick drill has passed on the
user's board. M6's open items stay in M6; the 2026-11-06 `NO FRAME` remains a
concern, and a recurrence on any board blocks gift readiness.

## M8 — Radar revision
The radar lab's Favourite C, as tweaked on the panel, replaces the wide view
([0089](decisions/0089-full-width-radar-revision.md)). Taken on before M7's
remaining gift readiness (user, 2026-11-22); release 16 carries it with 0088.
- [x] **Full-width frame:** 400 px radar, left pills, octagon markers and
  below-horizon diamonds with halos, selected ring, sphere-shaded coastline,
  scale ruler, haloed arrows; the strip, glyph and ID-label code retired;
  host tests and simulator snapshots; accepted on the panel. Done 2026-11-22:
  the user accepted dev image `0.1.15-m8b` on the panel after one tweak (the
  pulse restarts with each new selection).
- [x] **Trails and selected orbit:** past-line trails and the selected orbit
  as presentation state outside the renderer, in the simulator and on the
  device, with frame time, heap and input latency measured on the S3;
  accepted on the panel. Done 2026-11-23: `overhead_app::motion`, frame back
  to 20 Hz, the user accepted dev image `0.1.15-m8c`
  ([0089 note](decisions/0089-full-width-radar-revision.md#note--2026-11-23-trails-and-selected-orbit)).
  Left for M8: release 16.

**Done when:** the device app shows the revised radar, the user accepts it on
the panel, its cost is measured within 0077/0088's limits, and release 16
carries it.

## Boundaries
Keep tuning inputs explicit; they need not be device settings. Defer uncertain
presentation policies rather than building a plugin system. Knob hardware and
continuous zoom are in scope under M6 ([0060](decisions/0060-defer-timing-to-physical-prototype.md)).
Enclosure, battery integration, RTC, an Android app, and Starlink/full
catalogue remain outside this roadmap.
Possible later improvement, not planned: pass prediction on the device
([0077 note](decisions/0077-m2-operating-budget-visual-group.md#note--2026-11-10-device-pass-prediction-not-planned)).
