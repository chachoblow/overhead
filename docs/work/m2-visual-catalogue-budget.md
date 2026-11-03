# Device tracks the visual group within a measured operating budget
Status: active
Started: 2026-11-03. Scope: PLAN M2 (remaining limits/cadence/scheduling items);
decisions 0011, 0045, 0050, 0060, 0071, 0074, 0075, 0076; new 0077 (budget), 0078 (visibility glyph).

## Outcome
The device app fetches CelesTrak's `visual` group instead of STATIONS and tracks
it under an explicit operating budget: an object limit with a deterministic
over-limit rule, the selected object recalculated every frame and the rest in a
round-robin within ~1 s, and pass prediction deferred. The wide view's category
slot shows a visibility glyph (sunlit with a dark sky / in Earth's shadow /
daylight at the observer) instead of the category icon. Measured target costs
justify the limit and cadence, and M2 can close.

## Done when
- 0077 (operating budget) and 0078 (visibility glyph, superseding 0045's display
  slot) are written; PLAN M2 boxes updated and M2 done-when amended for deferred
  prediction.
- Host tests: sun position against a published reference; shadow/daylight
  classification; staggered update (selected always current, every object no
  older than the window, full update on time jumps); limit/priority/dropped
  count; glyph rasters and slot isolation; firmware fetch parsing of the pinned
  visual fixture and size caps.
- Simulator headless snapshots show the three glyph states on real pinned data.
- Board capture of the diagnostic build: visual fetch 200 and all records
  accepted, heap (internal and PSRAM) stable across refreshes, per-frame update
  cost max with ~156 objects, fetch display gap.
- Device app built, gated, flashed and booted to tracking.
- User accepts the glyph on the panel/snapshot (gate).
- `cargo check/test --workspace`, fmt, firmware host tests per feature pass.

## Boundaries
- No pass prediction, countdowns, persistence, knob, M5 smoothing.
- One request per refresh as now (0075 schedule); the group changes, not the rate.
- No toolchain change. Frozen M2 evidence untouched.
- Categories stay in the app (over-limit priority); only the display slot changes.

## Human gates
- Intent (visual group, glyph, deferral, cadence, over-limit): answered 2026-11-03, all recommended options.
- Visual acceptance of the visibility glyph: answered 2026-11-03, enlarged/recentred icons accepted "for now" from simulator snapshots; user saw the board (first draft) and said it looks good.

## Plan
- [x] core: `sun_ecef`, `in_earth_shadow`, `Catalogue::limit`; tests vs Meeus 25.a etc.
- [x] app: `visibility` module, `update_staggered`, `with_object_limit`/`dropped_objects`.
- [x] render: `visibility.rs` replaces `category.rs` in the slot (`Vis`); debug
      row 2 right shows `DROPPED n` when nonzero. Sim snapshot tests updated.
- [x] firmware: visual request, MAX_BODY 128 KiB, OBJECT_LIMIT 256, PSRAM first
      region (live_radar + live_fetch_main), STAGGER 1 s, update_max_us/oldest_ms/
      internal/psram telemetry; banners DEVICE_V2, LIVE_FETCH_RADAR_V4, LIVE_FETCH_V2.
- [x] sim: README `#visibility-glyph` recipe (ISS-only from the visual fixture, three subpoint scenes); ran it.
- [ ] Board: diagnostic capture running since 14:45 UTC, `timeout 15600`, log
      `firmware/target/visual-run1.log` (ignored); ELF sha256 dea5dae6…, image
      58785a80…, 1,358,080 B, gate exit 0, rev 69e8a28. Then the 8-hour device app.
- [x] Docs drafted: 0077, 0078, PLAN M2, DESIGN, sim/tools READMEs, fixture README.
- [ ] Firmware README `#visual-group-under-the-m2-budget-0077` with board evidence;
      HANDOFF/PROGRESS at completion.

## Discoveries and assumptions
- `visual` on 2026-11-03T14:32Z: 156 records, 64,659 B, sorted by NORAD ID,
  mean motion 11.3–16.0 rev/day (all LEO), epochs within ~1.5 days. Includes
  ISS, CSS, HST; 91 names contain "R/B". Copy in scratchpad, to be pinned.
- esp-hal 1.2.2 `psram` (unstable, Auto tries octal first); esp-radio/esp-rtos
  allocate with `esp_alloc::InternalMemory`. Plan: add PSRAM as the first heap
  region so ordinary allocations (catalogue, TLS, JSON) use PSRAM and internal
  RAM stays for the radio. Risk: atomics must not live in PSRAM (esp-alloc note).
- Default over-limit priority: category role (0045), then ascending NORAD ID.
  The device's single visual group has no role, so it is NORAD order there.
- Default dark-sky threshold: sun elevation below −6° (civil twilight).
  Cylindrical Earth shadow (no penumbra/refraction).

## Verification
- 2026-11-03 board boot (diagnostic V4, home site): fetch 1 HTTP 200, 64,659 B,
  156/156 accepted, ingest 291 ms, total 2.76 s, handshake 766 ms, display gap
  438 ms; heap peak 490,284 B; internal used 44.7 KB of 160 KiB; PSRAM 313–345 KB;
  stack 7,296 of 39,552 B. Update cost: 97 ms on the complete frame after load,
  then 20–32 ms per frame (≈9 objects × ~2.2 ms, PSRAM-resident); idle frames
  ~36 ms total, single skips only around full refreshes as before.
- 2026-11-03 (host): workspace check/tests pass (app 40, render 100, sim snapshot 12,
  core catalogue 9); firmware host default 19, element-fetch 55, live-utc-boot 70,
  sharp-boot-capture 60; dummy release builds of device, live-fetch-radar,
  live-fetch-faults, live-fetch.

## Open questions
- Object limit 256: memory is ample, but at 256 objects each frame would update
  ~13 objects (~29 ms), tight against 50 ms. Keep 256 (only growth reaches it) or
  lower to ~200? Default: keep, record the measured risk.
- SNTP renewals: the trust gate clears samples/selection on every accepted
  resync (stricter than 0063's >100 ms rule), costing a ~100–175 ms complete
  update and a selection reset. User agreed (2026-11-03) to fix it as a separate
  work item: keep state across resyncs within 100 ms. Record in HANDOFF.
