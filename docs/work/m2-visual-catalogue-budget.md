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
- Visual acceptance of the visibility glyph: needed at the end (snapshot + panel).

## Plan
- [x] core: `sun_ecef`, `in_earth_shadow`, `Catalogue::limit`; tests vs Meeus 25.a etc.
- [x] app: `visibility` module, `update_staggered`, `with_object_limit`/`dropped_objects`.
- [x] render: `visibility.rs` replaces `category.rs` in the slot (`Vis`); debug
      row 2 right shows `DROPPED n` when nonzero. Sim snapshot tests updated.
- [x] firmware: visual request, MAX_BODY 128 KiB, OBJECT_LIMIT 256, PSRAM first
      region (live_radar + live_fetch_main), STAGGER 1 s, update_max_us/oldest_ms/
      internal/psram telemetry; banners DEVICE_V2, LIVE_FETCH_RADAR_V4, LIVE_FETCH_V2.
- [ ] sim: review recipe for three visibility states on the visual fixture.
- [ ] Board: diagnostic capture over ≥2 refreshes (REFRESH_MINUTES=120), then device app.
- [ ] Docs: 0077, 0078, PLAN, DESIGN, READMEs, HANDOFF/PROGRESS at completion.

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
- 2026-11-03 (host): workspace check/tests pass (app 40, render 100, sim snapshot 12,
  core catalogue 9); firmware host default 19, element-fetch 55, live-utc-boot 70,
  sharp-boot-capture 60; dummy release builds of device, live-fetch-radar,
  live-fetch-faults, live-fetch.

## Open questions
- Object limit value: default 256 (body cap 128 KiB), to confirm against
  measured memory/cost.
