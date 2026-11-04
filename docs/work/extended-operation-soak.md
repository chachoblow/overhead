# The device runtime's frame cadence and resources are measured over a day
Status: active
Started: 2026-11-04. Scope: PLAN M6 "Verify end-to-end performance and extended
hardware operation"; decisions [0060](../decisions/0060-defer-timing-to-physical-prototype.md),
[0075](../decisions/0075-fetched-element-refresh-and-staleness.md),
[0076](../decisions/0076-device-app-build.md), [0077](../decisions/0077-m2-operating-budget-visual-group.md)

## Outcome
Measured evidence that the shared device runtime (`firmware/src/live_radar.rs`)
holds its 50 ms (20 Hz) frame grid over ≥24 h at the home site, with every
skipped deadline attributed to a cause (fetch, catalogue load, SNTP event,
full refresh, knob zoom, selection change, idle), plus heap/stack peaks and
refresh/renewal outcomes over the run.

## Done when
- One ≥24 h `live-fetch-radar` capture (default 8 h refresh: boot + 3 refreshes,
  ~1,350 SNTP polls) with per-cause frame/skip/work-time telemetry.
- A summary (firmware README) with skipped deadlines and work-time
  percentiles per cause, effective frame rate, heap/stack peaks, every
  refresh and renewal outcome.
- User feel pass with knob and BOOT use during the run.
- Pass bar (user, 2026-11-04): 20 Hz with skips only around fetches and full
  refreshes. Any other skip cause is fixed in this item if small, otherwise
  recorded as an accepted limit in a 0060 note (user decides).
- Board back on the 8-hour device app at the end (gate passed).

## Boundaries
No persistence, no device pass prediction, no 0077 budget change, no toolchain
change. Telemetry only in the diagnostic (`Log::Diagnostic`); the app's event
log stays unchanged. Each reset costs one CelesTrak request; avoid needless
resets. Site/credentials never committed or printed.

## Human gates
- Item choice, 24 h duration, pass bar, board occupied ~1 day: asked/answered
  2026-11-04: yes to all recommendations.
- Feel pass (knob + BOOT on the running diagnostic): needed, during the soak.
- Accepting any non-fetch/full skip cause as a limit: only if one remains.

## Plan
- [ ] Host-tested `firmware/src/frame_stats.rs`: per-cause frame/skip counts,
      work-time histograms, window and since-boot reports.
- [ ] Wire into `live_radar.rs` (diag only): causes from FETCHING, DATA loads,
      EVENTS, knob detents, selection change, Full transfers / RefreshRetained.
- [ ] Host tests, `cargo check/test --workspace`, firmware host tests, release
      build, image gate; commit.
- [ ] Short flash + capture (~15 min) to sanity-check telemetry; fix if needed.
- [ ] Start ≥24 h capture in background; ask user for feel pass.
- [ ] Analyse; fix small non-fetch causes (then re-soak the affected part) or
      ask for acceptance.
- [ ] Reflash device app; docs (README evidence, 0060 note, PLAN, HANDOFF,
      PROGRESS); delete this file; handoff.

## Discoveries and assumptions
- Baseline (`target/resync.log`, 40 min, 2026-11-03): 45,800 frames ≈ 20.0 Hz;
  ~200 skipped deadlines (176 single, a few multi at boot/selection changes);
  logged `total_us` p50 44 ms but only skip/every-200th frames are logged.
- Display loop sleeps 5 ms after each iteration; a ~45–48 ms frame plus the
  sleep may itself miss the next deadline (suspected idle-skip cause).

## Verification

## Open questions
