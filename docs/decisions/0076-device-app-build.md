# 0076 — The live radar becomes the device application build

Date: 2026-11-02
Status: accepted (user, 2026-11-02: short event log, halt on panic, a
refresh run on the new build, board left on the app afterwards).
Builds on [0074](0074-live-utc-radar-on-fetched-elements.md) and
[0075](0075-fetched-element-refresh-and-staleness.md); input under
[0068](0068-boot-selection-gesture.md)/[0070](0070-boot-input-capture-independent-of-render.md).
Done 2026-11-03: the app booted to tracking on 20 fetched records, refreshed
once at the 2-hour test setting with only event lines printed, and the user
saw the radar on the panel
([evidence](../../firmware/README.md#device-application-0076)). The board runs
the 8-hour app build.

## Decision
1. **A named app build.** `--features device`, binary `overhead-s3-device`
   (`firmware/src/device_main.rs`), is the Overhead device application: the
   live-UTC radar at the build-time home site on fetched STATIONS elements,
   refreshed and aged under 0075, with BOOT cycling. Diagnostics stay
   separate images.
2. **One runtime, two log levels.** The runtime lives in the target-only
   `firmware/src/live_radar.rs`, shared with the `live-fetch-radar`
   diagnostic (and its `live-fetch-faults` variant). They differ only in
   `Log`: the diagnostic prints every telemetry line; the app prints the
   **event log** only — boot banner, Wi-Fi connect/disconnect/failure, each
   fetch's failure stage and `FETCH_SCHEDULE` outcome/next time, `LIVE_FETCH`
   load, `OLD DATA` and expiry, the first radar frame, `NO FRAME` renders,
   SNTP Kiss-o'-Death and panics. Nothing per frame, per SNTP poll or per
   press. Blocking UART prints sit in the display loop, so fewer lines also
   mean fewer loop stalls (not a timing claim).
3. **Halt on panic.** The app prints the reason and stops, with the panel
   frozen, as the diagnostics do. It never reboots itself: every boot costs a
   CelesTrak request, so a crash loop could hammer the server. Unattended
   recovery (SPI faults and the like) stays an open risk.
4. **The fault plan never reaches the app.** `device` with
   `live-fetch-faults` fails to compile.
5. **Configuration unchanged.** Credentials and site from the ignored
   `.wifi.env`/`.site.env`; `REFRESH_MINUTES` (≥120) still shortens the
   refresh for verification runs; the default is 8 hours.

## Why
Every image on the board was labelled a diagnostic although
`live-fetch-radar` already did what the prototype needs (PLAN M6 "done
when": boots with valid time/data and tracks between connections). Copying
it into a second main would let the app and its diagnostic drift; a shared
runtime keeps the diagnostic able to explain the app's behaviour. A silent
app was rejected: without the diagnostic image a misbehaving board could not
explain itself. Auto-reboot was rejected for now (point 3).

## Consequences
- The crate is still `overhead-s3-benchmark` and its default binary is the
  kernel benchmark; renaming either is a separate decision.
- `live-fetch-radar` builds from the shared runtime are new builds (banner
  `OVERHEAD_S3_LIVE_FETCH_RADAR_V3`); its earlier evidence belongs to the
  revisions recorded in the firmware README.
- Knob zoom, persistence, location source and frame-timing policy are later
  work on this app build.

## Note — 2026-11-08: reset on panic for updatable devices
[0086](0086-bootloader-rollback-and-health-check.md): once signed updates
land, the device app resets on panic (and a hardware watchdog resets a hung
loop) so the rollback bootloader can recover a bad image. Halting remains
acceptable for diagnostics.
