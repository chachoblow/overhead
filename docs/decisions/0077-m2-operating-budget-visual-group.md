# 0077 — M2 operating budget: the visual group, an object limit and staggered tracking

Date: 2026-11-03
Status: accepted (user, 2026-11-03: the `visual` group, recompute the selected
object every frame and the rest within ~1 s, keep by priority with a debug-only
dropped count, defer pass prediction). Board evidence pending; see
[the work item](../work/m2-visual-catalogue-budget.md) while it is active.
Changes the group fetched under [0071](0071-https-element-fetch-first-slice.md)
and [0074](0074-live-utc-radar-on-fetched-elements.md); refresh, staleness and
backoff stay [0075](0075-fetched-element-refresh-and-staleness.md)'s. The
category display slot changes under [0078](0078-visibility-glyph.md).

## Decision
1. **Catalogue.** The device fetches CelesTrak's `visual` group
   (`GROUP=visual&FORMAT=JSON`) instead of STATIONS: about 150 naked-eye
   objects (156 on 2026-11-03, all low Earth orbit, including ISS, CSS and
   HST). Still one request per refresh on 0075's schedule.
2. **Input limits.** Response bodies up to 128 KiB (~310 records; the group
   measured 64,659 B). The response buffer, catalogue and TLS live in PSRAM,
   the first heap region; internal RAM serves the radio's internal-only
   allocations.
3. **Object limit.** At most **256** objects per load. Over the limit, keep
   higher 0045 roles first, then lower NORAD IDs (`Catalogue::limit`,
   `RadarApp::with_object_limit`). The device's single group has no role, so
   it keeps the lowest IDs. The dropped count appears only on the debug page
   (`DROPPED n`) and in the `LIVE_FETCH loaded` event line; the normal view
   shows no notice. Dropping is an operating limit, not a validity rule.
4. **Tracking cadence.** Each frame recomputes the selected object at the
   frame UTC, plus a round-robin share of the others sized so every object is
   recomputed within **1 s** (`RadarApp::update_staggered`). A first frame, a
   load, time running backwards or a gap of 1 s or more is a complete frame.
   Unselected markers may lag up to 1 s; every readout comes from the selected
   object, which is always current. M5 smoothing may hide the steps later.
5. **Pass prediction deferred.** No pass prediction runs on the device. The
   pass-search engine, its semantics (0012–0016) and the offline tool stay;
   0050's display deferral stands. M2 closes on the tracking budget, and its
   done-when no longer requires prediction accuracy on the device.

## Why
STATIONS gave an often-empty sky; the `visual` group is what a person outside
could actually see, and its size suits the board. Full-frame recomputation
costs ~0.8–1.5 ms per object on the S3 ([costs](../evaluations/s3-costs.md)),
so ~150 objects every 50 ms frame is impossible; staggering keeps the selected
object exact and the frame cost bounded. A 24-hour search costs 1–2 s per
object, so prediction would need its own background scheduling for a feature
0050 already deferred. Keeping objects by role then ID is deterministic and
needs no per-ID tables; rejecting the whole fetch over the limit was rejected
because a growing group could stop updates entirely. The limit is ~1.6× the
measured group so normal growth never drops anything.

## Consequences
- Simulator replay keeps complete frames (`update`); only the device runtime
  staggers. Square diagnostics are unchanged.
- Diagnostic telemetry adds `update_max_us`, `update_boot_max_us`,
  `oldest_ms` and internal/PSRAM heap use. Banners: `OVERHEAD_DEVICE_V2`,
  `OVERHEAD_S3_LIVE_FETCH_RADAR_V4`, `OVERHEAD_S3_LIVE_FETCH_V2`.
- Measured cost of staggering on the board: ~20–32 ms per frame for ~9
  objects (catalogue in PSRAM, ~2.2 ms each), against ~0.8–1.5 ms per object
  in internal RAM before. At the 256 limit a frame would update ~13 objects
  (~29 ms), tight against 50 ms; only group growth reaches it.
- The UTC trust gate (`firmware/src/utc_trust.rs`) discards all samples and
  the selection on every accepted SNTP resync, so the next frame is complete:
  ~100–175 ms with 156 objects (a few skipped frames per renewal), and BOOT
  selection returns to the first object in view. That is stricter than
  [0063](0063-bounded-utc-holdover-direction.md), which requires a fresh start
  only when the new time differs by more than 100 ms. Agreed follow-up (user,
  2026-11-03): keep samples and selection across resyncs within 100 ms, as a
  separate work item with a note on 0063; jumps and time loss still clear
  everything (0062/0063).
- Not decided: scheduling for any future prediction, persistence, a real
  location source, physical COM edges or 20 Hz proof.

## Verification
Host tests cover `Catalogue::limit`, staggered updates against complete
frames on the pinned `visual` fixture, fallbacks, the debug count and the
fetch caps. Board evidence (fetch, heap across refreshes, update cost) is
recorded in the [firmware README](../../firmware/README.md#visual-group-under-the-m2-budget-0077).
