# Firmware

The **device application** is the opt-in `device` build
([below](#device-application-0076)); this README covers building, flashing and
capturing it, its on-board evidence, signed releases and gift units. Older
per-image diagnostics are in [firmware diagnostics](../docs/evaluations/firmware-diagnostics.md).
The crate name (`overhead-s3-benchmark`) and default binary are historical:
the early M2 kernel benchmark, documented from [Toolchain/build](#toolchainbuild)
on. The default kernel
binary has no display, Wi-Fi, allocator, PSRAM, catalogue ingestion/aggregation,
or scheduling. A separate opt-in [catalogue memory experiment](CATALOGUE_MEMORY.md)
now has first target captures; its feature/build commands are distinct.
All orbital calculations still run in `overhead-core`. Results and limitations:
[first S3 evaluation](../docs/evaluations/s3-costs.md),
[16-slot follow-up](../docs/evaluations/s3-scaling-16.md), and
[remaining V2 suites](../docs/evaluations/s3-expanded.md). All 122 prepared
kernel workloads are host-tested and now have target captures. Additional distinct
populations and independent numerical checks have separate [frozen-suite evidence](../docs/evaluations/s3-m2.md).
Whole-device peak-memory budgets and scheduling guarantees remain open.

## Board on the Mac (`mac.sh`)

Since 2026-11-16 the user's board is plugged into the MacBook, not devbox;
`mac.sh` finds the single `/dev/cu.usbserial-*` port itself. Build on devbox as usual, then flash
and capture through `./mac.sh` (run `./mac.sh` for usage):
`info` (port and MAC check), `read SECS LOG` (no reset), `monitor SECS LOG`
(reset and capture) and `flash ELF LOG [SECS]`, which is the "USB flash"
command [below](#device-application-0076) (bootloader, table, erased
`otadata`, app in `ota_0`; settings and elements kept). `flash` refuses a
board whose MAC isn't `OVERHEAD_MAC` (default the user's board) and checks the
uploaded ELF's sha256. `erase MAC` wipes the whole chip (new gift boards
only) after checking the board has that MAC; giving the user's own MAC would
wipe its Wi-Fi, site and channel. Run the image gate before flashing, as for USB.

devbox's SSH key can only run the Mac's forced-command gate
(`~/code/devbox/mac/overhead-gate`; why and how in the devbox README,
2026-11-16), so commands on the Mac outside `mac.sh` don't work, and tools
that call `espflash --port` themselves (`capture.py`, `drift_capture.py`, …)
only work with a board on devbox's USB. Logs are host-timestamped on devbox,
a few ms after the Mac reads them. Long captures need the Mac awake (the gate
runs them under `caffeinate -i`, which doesn't survive a closed lid on battery).
On 2026-11-22 `read` returned only garbled bytes (6 KB in 15 min) while
`monitor` worked; use `monitor` (it resets the board) when `read` looks wrong.

## Historical diagnostics
Per-image diagnostic evidence from M2 to M6 (Sharp bring-up, frame cost and
transfer, BOOT capture, live UTC/SNTP, the first HTTPS fetch slices, the coastline
measurement images and the BLE spike) is in
[firmware diagnostics](../docs/evaluations/firmware-diagnostics.md).

## Device application (0076)

Since [0089](../docs/decisions/0089-full-width-radar-revision.md) (M8) the
device draws the full-width radar: left pills, octagon and below-horizon
markers, pulsing selected ring, shaded coastline (12,000 B raster) and scale
ruler; the period cache is gone. Accepted on the panel 2026-11-22 with dev
image `0.1.15-m8b` (`target/m8b-8h.elf`, OVERHEAD_BUILD=15, beta, 8 h;
`OTA boot … state=Valid`, first frame, no panic in `target/m8/flash2.log`).
Trails and the selected orbit followed (`0.1.15-m8c`, `target/m8c/m8c-8h.elf`,
accepted on the panel 2026-11-23; [0089 note](../docs/decisions/0089-full-width-radar-revision.md#note--2026-11-23-trails-and-selected-orbit)).
`live-fetch-radar` adds `motion_max_us= orbit_points= trails=` to
`INPUT_CAPTURE_STATS` and a per-window `RENDER_STAGES_MAX_US` line (longest
time to each `FrameStage` boundary, then `send_partial_max_us`).

**On-board evidence, 2026-11-22/23** (Mac `/dev/cu.usbserial-21410`, later
`-110`, MAC checked, every image gated; logs `target/m8c/`, local):

| Run | Result |
|---|---|
| Item 1 frame, no motion (`base-run1.log`) | 14 Hz, idle work max ~70 ms, update ~22 ms |
| Motion, before fixes (`diag-run1.log`, `diag2-run1.log`) | 11.4 Hz, ~86 ms; stage maxima clear 14.6, markers 17.8, motion 13–15, overlays 9.4 ms |
| After fixes, 30 min (`diag5-review2.log`) | 20 Hz: idle 22,999 frames / 4 skipped, p50 41, p99.9 51 ms; heap 504 KB (PSRAM 460 KB, internal 44.7 KB used, 127 KB free); no panic |
| User at the panel (`diag5-review3.log`) | 10 IO7 presses (applied next loop, ~5 ms), select frames 93–98 ms; 42 knob lines, knob frames 159–163 ms (coast re-raster ~45 + all-rows send ~52 ms); the user accepted it |
| Device image `0.1.15-m8c` (`m8c-flash.log`) | `OTA boot … state=Valid`, `STORE loaded`, first frame, no panic |

The fixes: `coast_measurement::Screen` fills byte-wide (`fill_solid`, counters
still per pixel), `Sample::hidden_by_earth` cached at sampling, trail and orbit
camera points cached in `Motion`, marker halos and pill fills as spans, f32
line clipping and ring trig.

`--features device` builds `overhead-s3-device` (`src/device_main.rs`), the
Overhead device app under [0076](../docs/decisions/0076-device-app-build.md):
the live-UTC wide radar at the home site on fetched elements (the `visual`
group since [0077](../docs/decisions/0077-m2-operating-budget-visual-group.md),
banner `OVERHEAD_DEVICE_V2`; STATIONS before), refreshed and aged under 0075,
with BOOT cycling. It runs the same runtime
(`src/live_radar.rs`) as the `live-fetch-radar` diagnostic and its fault build;
since [0088](#selection-button-and-captured-knob-switch-0088) a button on IO7
cycles instead of BOOT;
the app prints only the event log (banner, `SNTP_WIFI`, `FETCH_FAILED`,
`FETCH_SCHEDULE`, `LIVE_FETCH` load/stale/expired, `RADAR first_frame`,
`LIVE_UTC no_frame_sent`, `SNTP KoD` (0080), `OVERHEAD_DEVICE_FAILED` on panic,
and since 0086 the `OTA`/`SETTINGS` lines below). Since
[0086](../docs/decisions/0086-bootloader-rollback-and-health-check.md) it
resets on panic (it halted before). `device` with `live-fetch-faults` does not
compile. **Since 0083–0086 the recipe below is historical**: runtime images
need the rollback bootloader, the OTA table and `dev-settings` for a local
build; see [Signed OTA and rollback](#signed-ota-and-rollback-00830086).

```sh
# from firmware/, after . "$HOME/export-esp.sh" and the Wi-Fi/site exports above
cargo build --release --locked --bin overhead-s3-device --features device
ELF="$PWD/target/xtensa-esp32s3-none-elf/release/overhead-s3-device"
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --partition-table partitions.csv --skip-update-check \
  "$ELF" target/device.bin
python3 image_gate.py "$ELF" target/device.bin
# flash and capture in one command (absolute paths: a backgrounded shell may
# not keep the cd); a boot costs one CelesTrak request unless a stored copy
# younger than the interval loads (0079, below)
timeout 600 espflash flash --port /dev/ttyUSB0 --chip esp32s3 --flash-size 8mb \
  --flash-mode dio --flash-freq 40mhz --partition-table "$PWD/partitions.csv" \
  --non-interactive --skip-update-check \
  --monitor "$ELF" > "$PWD/target/device.log" 2>&1
```

The same `--partition-table` applies to `live-fetch-radar` and its fault build
since [0079](../docs/decisions/0079-fetched-element-persistence.md); without it
the runtime logs `STORE unavailable` and fetches every boot.

To check the MAC without booting the old image (and spending a request):
`espflash board-info --port /dev/ttyUSB0 --chip esp32s3 --after no-reset`.

**On-board evidence, 2026-11-03** (devbox, `/dev/ttyUSB0`, MAC
`80:45:6b:2a:45:08`, rev `2904edf`, home site, `REFRESH_MINUTES=120`; image
1,346,208 B, 16.17%, gate passed):

| Run | Result |
|---|---|
| Boot (05:17 UTC) | only event lines: banner, `NO FRAME`, Wi-Fi at 1 s, first radar frame, fetch 1 `Success` at 14 s, 20 records loaded, one object in view (67688) |
| User check | radar, 67688 marker and ID, glyph strip; three BOOT presses with no visible change, as expected with one object in view (multi-object cycling was shown on the diagnostic, same runtime) |
| Refresh (to 07:55 UTC, 2h40m) | fetch 2 `Success` at uptime 7,268 s, 20 records, nothing above the horizon then (selection cleared as designed); no `FETCH_FAILED`, Wi-Fi drop or panic; nothing but event lines |

Afterwards the board was left on the default 8-hour app build (gate passed,
`refresh_s=28800`); its 07:57 UTC boot fetch loaded 20 records. Not covered by
this run: heap/stack figures (the app does not print them; use the diagnostic),
BOOT cycling among several in-view objects on this build (shown on the
diagnostic, same runtime).

## Visual group under the M2 budget (0077)

[0077](../docs/decisions/0077-m2-operating-budget-visual-group.md) changes the
shared runtime (`src/live_radar.rs`) for both the app and the diagnostic: one
`GROUP=visual` GET per refresh, bodies up to 128 KiB, PSRAM as the first heap
region (response buffer, catalogue, TLS; internal RAM keeps the radio's
internal-only allocations), at most 256 objects (`dropped=` on `LIVE_FETCH
loaded`, `DROPPED n` on the debug page), and staggered tracking: the selected
object every frame, the rest within 1 s. The selected object's slot shows the
[0078](../docs/decisions/0078-visibility-glyph.md) visibility icon.
`live-fetch-radar` (banner `OVERHEAD_S3_LIVE_FETCH_RADAR_V4`) adds
`update_max_us` (largest app update in the 10 s window), `update_boot_max_us`,
`oldest_ms` (age of the oldest unselected sample), `internal_used`,
`internal_free` and `psram_used` to `BOOT_CAPTURE_STATS`, and the region
figures to `FETCH_DONE`. Build, gate and flash as in the sections above.

**On-board evidence, 2026-11-03** (devbox, `/dev/ttyUSB0`, MAC
`80:45:6b:2a:45:08`, rev `69e8a28`, home site, `REFRESH_MINUTES=120`; image
1,358,080 B, gate passed; one capture 14:45–18:48 UTC, stopped after the second refresh):

| Measure | Result |
|---|---|
| Fetches | boot + 2 refreshes, all HTTP 200, 64.6 KB, 156/156 accepted, `dropped=0`; ingest 291 ms; whole fetch 2.8–10.6 s (transfers 1.5, 7.4 and 9.0 s); handshake 0.77–1.09 s; display-loop gap 438, 463 and **484 ms** per fetch (close to the 500 ms VCOM interval) |
| Heap | internal 44,700 B used of 160 KiB throughout (119,140 B free); live heap in idle windows **389,708 B** (PSRAM 345,008 B) after every load, identical after all three loads: no leak. Peak 490,284 B at boot, 699,008 B during a refresh (old and new catalogue plus TLS, in PSRAM) |
| Update cost | complete update of 156 objects ~97 ms after a load; largest staggered update per 10 s window (1,223 windows): median 20.3 ms, p99 32.3 ms, max 42 ms (~9 objects at ~2.2 ms each from PSRAM) |
| Resyncs | every accepted SNTP renewal (~2 min) forces a complete update (229 of 1,452 windows; 173–181 ms, ~3 skipped frames) and returns selection to the first object in view; see 0077's consequences. Fixed below |
| Staleness | `oldest_ms` at most 932 ms: every object recomputed within the 1 s window |
| Stack | 7,296 of 39,552 B |

Selection was kept across both refresh loads (17590, then 16719). No panic,
Wi-Fi drop or `FETCH_FAILED`. Not measured: the 256-object limit on the board
(only host-tested; it would update ~13 objects per frame, ~29 ms) and
`DROPPED` on the panel.

Afterwards the board was left on the default 8-hour app build (rev `a750a06`,
image 1,357,920 B, gate passed, `refresh_s=28800`): banner
`OVERHEAD_DEVICE_V2`, Wi-Fi at 1 s, first frame, fetch `Success` at 14 s, 156
records loaded, 9 in view.

## Small SNTP resyncs keep tracking (0063 note)

Under the [0063 note](../docs/decisions/0063-bounded-utc-holdover-direction.md#note--2026-11-03-small-resyncs-keep-tracking),
a renewal within 100 ms of the old anchor's projected UTC keeps samples and
selection, and logs `LIVE_UTC source=Some(Resynced { step_us: n })` on the
diagnostic; a first anchor or larger jump still logs `Accepted` and clears.
No banner change; build, gate and flash as above.

**On-board evidence, 2026-11-03** (devbox, `/dev/ttyUSB0`, MAC
`80:45:6b:2a:45:08`, `live-fetch-radar` at rev `27e8fea`, home site, default
8-hour refresh; image 1,358,992 B, gate passed; one capture 20:10–20:50 UTC):

| Measure | Result |
|---|---|
| Renewals | 1 `Accepted` at boot, then 35 `Resynced` (~every 64 s), none cleared; steps −25.5 to +14.2 ms, median \|step\| 4.2 ms |
| Update cost | after the boot load's complete update (87 ms), largest staggered update per 10 s window (229 windows): median 19.3 ms, p99 30.8 ms, max 31.0 ms; **no window over 60 ms** (0077 run: 229 of 1,452 over 100 ms) |
| Skipped frames | 2+ deadlines skipped only at boot/first load and once at each of four selection changes; all other skips single idle deadlines, as before |
| Selection | four changes, none in a window with a renewal (selected object left the sky, 0049) |
| Other | `oldest_ms` ≤ 916; live heap 389,708 B (as under 0077); stack 7,312 of 39,568 B; one fetch, 156/156; no panic, Wi-Fi drop or `FETCH_FAILED` |

Not shown on the board: a renewal beyond 100 ms or after expiry (host-tested
in `utc_trust`), and BOOT presses across a renewal (no presses in this run;
the selection itself survived every renewal). Afterwards the board was left
on the default 8-hour app (image 1,358,208 B, gate passed): 156 loaded, 11 in
view.

## Selection button and captured knob switch (0088)

[0088](../docs/decisions/0088-selection-button-and-captured-knob-switch.md):
gift units expose a button and a knob, not BOOT. In the runtime
(`src/live_radar.rs`, device app and `live-fetch-radar`):

- **Wiring:** a momentary button from **IO7 to G** (internal pull-up, active
  low). On a four-leg tactile button use **diagonally opposite legs**:
  same-side pairs are always connected, so wires there never switch. Encoder
  wiring is unchanged ([knob check](#rotary-encoder-wiring-check-knob-test)).
- **Capture:** after the setup hold, IO7 (selection) and IO6 (knob switch,
  info page) are sampled by 0070's 5 ms TIMG1 interrupt
  (`boot_capture::start_with_info`, 20 ms qualification) and drained at the
  display loop boundary; `button_input::toggle_info` (host-tested) toggles
  the info page once per drained press. The diagnostics
  (`sharp_boot_capture_main`, `live_utc_main`) still read GPIO0.
- **Log:** `INPUT_CAPTURE_READY select=IO7 info=IO6 …` (diagnostic only);
  the app prints `SELECT drained= applied= uptime_us= loop_gap_us= fetching=
  valid= selected=` and `INFO_PAGE presses= shown= loop_gap_us= fetching=`.
  A press held through a stall shows as `loop_gap_us` far above 5 ms.

```sh
# session image (from firmware/, after . "$HOME/export-esp.sh" and the env
# exports); the 8 h build is the same without REFRESH_MINUTES
OTA_CHANNEL=beta REFRESH_MINUTES=120 OVERHEAD_BUILD=15 \
  OVERHEAD_VERSION=0.1.15-inputs cargo build --release --locked \
  --bin overhead-s3-device --features device,dev-settings
# save-image, image_gate.py and flash as in the OTA section
```

`OVERHEAD_BUILD=15` keeps a dev image from reinstalling release 15 from
`beta` at its next refresh (`Offer::Current`); release 16 still installs over
it.

**On-board evidence, 2026-11-14** (devbox `/dev/ttyUSB0`, MAC
`80:45:6b:2a:45:08`, every image gated, settings kept; the board was left on
the 8 h build):

| Check | Result |
|---|---|
| First flash (04:44 UTC, build 0) | `OTA boot … state=Valid channel=beta`, `STORE loaded`, Wi-Fi 2 s, first frame, no panic (`target/inputs-boot.log`) |
| Session image (05:02 UTC) | `OTA boot build=15 version=0.1.15-inputs … Valid channel=beta`; fetch at 05:02:44, 156 loaded, 5 in view (`target/inputs-session.log`) |
| IO7 selection (user) | 11 presses, each `SELECT drained=1 applied=1`, cycling 20443→22286→22803→28415→20443…; user saw the selection change |
| Knob info page (user) | `INFO_PAGE presses=1 shown=true`, then `shown=false` on the next press |
| Knob presses across a refresh (user, 17:00–17:09 UTC) | 214 `INFO_PAGE` lines, ~1 per second; six during the fetch (`fetching=true`, normal gaps); one pressed during a 479 ms stall right after the store save, applied when the loop resumed (`loop_gap_us=478539`); fetch `Success`, `OTA_CHECK … offer=Current`, no panic or `NO FRAME` (`target/inputs-refresh.log`, host-timestamped) |
| 8 h build (17:11 UTC) | `OTA boot build=15 … Valid channel=beta`, `STORE loaded` (17:08 copy, so no boot fetch), Wi-Fi 2 s, first frame (`target/inputs-8h.log`) |

Not shown: a press during the ~34–41 ms Wi-Fi association gap (0070's limit,
now for both inputs).

## Rotary encoder wiring check (knob-test)

`overhead-s3-knob-test` (`--features knob-test`) is an isolated, UART-only
encoder check, not the device app. Wiring (direct, internal pull-ups, no
caps): A→IO4, C (middle)→G, B→IO5, switch→IO6 and G. PCNT unit 0 decodes
quadrature with the maximum (1023-cycle) glitch filter and logs `KNOB_COUNT`,
`KNOB_SWITCH` and periodic `KNOB_LEVELS`. Build/flash as for the Sharp test,
with `--bin overhead-s3-knob-test --features knob-test`.

2026-11-03 on the board (`/dev/ttyUSB0`, gate passed): 4 counts per detent,
detent rest at A=B=1, clockwise counts negative; 10 clicks each way returned
to 0, three presses registered once each, and a fast spin ended on a detent
with no lost counts. The part number is unknown (24-detent PEC11-style).

**Knob zoom in the device runtime** (`src/live_radar.rs`, so both the device
app and `live-fetch-radar`): PCNT unit 0 as above with ±20,000 limits;
`src/knob.rs` (host-tested) unwraps the counter and rounds to detents, and the
display loop applies `RadarApp::zoom_detents` once per loop (clockwise zooms
in, √1.2 per detent). The diagnostic prints `KNOB_READY` and `KNOB detents=
fov_deg= selected=`; the app prints nothing for it. 2026-11-03 on the board
(`live-fetch-radar`, gate passed, 156 loaded, 8 in view): ~180 `KNOB` lines,
fast spins arriving as multi-detent steps (up to 9), three BOOT presses still
applied, no panic; the user judged the zoom feel good. The board was then left
on the device app with the knob (gate passed, 156 loaded).

## Extended operation and frame cadence (24 h soak)

`live-fetch-radar` (banner `OVERHEAD_S3_LIVE_FETCH_RADAR_V5`) charges every
skipped 50 ms deadline to a cause (`src/frame_stats.rs`, host-tested): the
strongest of the previous frame's and the interval's causes, ordered idle <
log (diagnostic UART only) < knob < select < full < sntp < load < fetch <
store (0079, added after the soak). It
prints `FRAME_WINDOW span_ms= frames= skipped= cause=frames/skips/max_work_ms…`
every 10 s and `FRAME_BOOT cause= frames= skipped= p50_ms= p90_ms= p99_ms=
p999_ms= max_us=` (since boot, work = update + render + send) every 10 min.
The device app prints nothing new. The 8 KiB statistics static comes out of
the CPU0 stack region (30,808 B stack, was 39,568 B). Build, gate and flash as
above; the default 8-hour refresh.

**On-board evidence, 2026-11-04/05** (devbox, `/dev/ttyUSB0`, MAC
`80:45:6b:2a:45:08`, rev `00f55d0`, home site, image 1,365,776 B, gate passed;
one capture 14:28–14:36 UTC next day):

| Measure | Result |
|---|---|
| Cadence | 1,721,795 frames, 11,547 skipped deadlines (0.67%): effectively 20 Hz |
| Per cause (frames / skips / work p50, p99.9 ms) | idle 1,673,407 / **1** / 41, 45 · full 10,687 / 7,214 / 91, 105 (8 s retained full resend: ~52 ms SPI + render) · log 33,660 / 3,582 / 41, 55 (diagnostic only) · select 202 / 396 / 165, 179 · sntp 1,345 / 151 / 42, 94 (max 166) · fetch 197 / 113 / 41, 256 · knob 108 / 75 / 86, 120 |
| Fetches | boot + 3 refreshes (22:29, 06:30, 14:30 UTC), all 156/156, 3.2–3.9 s; selection kept across every load |
| SNTP | 1,357 polls: 1,345 observations (1 `Accepted`, 1,342 `Resynced`, 2 `NoRenewal`), 5 timeouts, 1 DNS failure, 6 ignored |
| Memory | live heap in idle windows 389,708 / 389,896 B throughout (no leak); peak 707,528 B during a refresh (PSRAM); internal 44.7–53.2 KB used; stack 7,760 of 30,808 B |
| BOOT sampling | max gap 34.4 ms at boot (Wi-Fi association), otherwise ≤23 ms |
| User feel pass (2026-11-04 ~14:40 UTC) | 119 knob steps, 16 BOOT drains: "Everything seemed good" |

No panic, Wi-Fi drop or `FETCH_FAILED`. Why the costly frames: a selection
change misses the one-entry period cache (`period_rows`, 4,097 soft-float f64
sin/cos samples, ~120 ms); a turned knob frame re-rasters the coastline (~45 ms
extra; zoom does not force a complete update). Accepted as limits under the
[0060 note](../docs/decisions/0060-defer-timing-to-physical-prototype.md#note--2026-11-05-24-hour-soak-limits-accepted).
Afterwards the board went back to the 8-hour device app (image 1,368,416 B,
gate passed): banner `OVERHEAD_DEVICE_V2`, fetch `Success` at 15 s, 156
loaded, 5 in view.

## Element persistence (0079)

The shared runtime saves each fetched body to the `elements` partition and
loads it at boot ([0079](../docs/decisions/0079-fetched-element-persistence.md);
format in `src/element_store.rs`, host-tested). Flash every runtime image with
`--partition-table partitions.csv` (recipe [above](#device-application-0076)).
Event lines (device app and diagnostic): `STORE loaded slot= seq= fetched_at=
body_bytes= load_ms=`, `STORE empty`, `STORE rejected`, `STORE unavailable
reason=`, `STORE saved slot= seq= body_bytes= steps= total_ms= erase_max_us=
write_max_us=`, `STORE_FAILED`, and at the first in-budget SNTP observation
`FETCH_RESUME stored_age_s= next_in_s=` (fetch deferred) or `… due=now`. The
diagnostic adds `STORE scan` (both slots' headers) and `DISPLAY_STORE_GAP`.

```sh
# from firmware/: clear the stored copy (next boot fetches and saves slot 0)
espflash erase-region --port /dev/ttyUSB0 --chip esp32s3 0x7b0000 0x50000
# damage slot 1's body (boot falls back to slot 0) and watch the boot
head -c 4096 /dev/urandom > /tmp/corrupt-4k.bin
timeout 150 espflash write-bin --port /dev/ttyUSB0 --chip esp32s3 --monitor \
  --non-interactive 0x7d9000 /tmp/corrupt-4k.bin > target/store-corrupt.log 2>&1
```

Slot 0 starts at 0x7b0000 and slot 1 at 0x7d8000; each body starts one sector
after its header.

**On-board evidence, 2026-11-05/06** (devbox, `/dev/ttyUSB0`, MAC
`80:45:6b:2a:45:08`, home site; `live-fetch-radar` V5 at
`REFRESH_MINUTES=120`, image 1,384,896 B, 17.32%, gate passed; device app
8-hour build 1,384,800 B, gate passed):

| Run | Result |
|---|---|
| 1. Erased partition (14:57 UTC) | `STORE empty`, fetch 156/156, `STORE saved slot=0 seq=1 body_bytes=65214 steps=34 total_ms=1727`; sector erase ≤2.0 ms, 4 KiB program ≤4.2 ms. The save overlapped the catalogue load frame, so a 2 s settle was added before saving |
| 2. Reset (15:04 UTC) | `STORE loaded slot=0 seq=1` (read 28 ms, load 321 ms), `NO FRAME` at 0.96 s, `FETCH_RESUME stored_age_s=346 next_in_s=6854`, no request; radar on stored data |
| 2. Its 2-hour refresh (16:59 UTC) | fetch 156/156, `STORE saved slot=1 seq=2 total_ms=1112 erase_max_us=2992 write_max_us=4133`; `DISPLAY_STORE_GAP` 11.8 ms; store window `store=23/0/39` (no skipped frame); live heap 389,708 B before and after; BOOT max gap 11.9–16.2 ms over 2 h 15 min |
| 3. Slot 1 body corrupted (17:16 UTC) | `slot1=Err(BodyCrc)`, `STORE loaded slot=0 seq=1`, `FETCH_RESUME stored_age_s=8248 due=now`, fetch 156/156, `STORE saved slot=1 seq=3` (erase max 18.3 ms, store gap 18.6 ms, one skipped frame) |
| 4. Reset (17:18 UTC) | `STORE loaded slot=1 seq=3`, `FETCH_RESUME stored_age_s=146 next_in_s=7054`, no request in 60 s |
| Device app (17:21 UTC on) | loads `slot=1 seq=3` with no request; its 8-hour refresh at 01:17 UTC saved `slot=0 seq=4`. The user reset it at 04:07 UTC: `STORE loaded slot=0 seq=4`, `FETCH_RESUME stored_age_s=10214 next_in_s=18586`, `RADAR first_frame` at ~6 s, no request; the user saw the radar come back |

Unexplained: before that reset, the user found the device app at `NO FRAME`.
It had been running at 01:17 UTC, when it fetched and saved, and nothing was
capturing its console. `NO FRAME` while running means trusted UTC lapsed (SNTP
not renewing; a KoD would also stop queries until a reboot, since fixed by
[0080](#sntp-kod-recovery-0080)). The user's first
reset press showed no change. A non-resetting capture of the device app was
started at 04:08 UTC (`target/device-overnight.log`, 12 h).

## SNTP KoD recovery (0080)

The shared runtime no longer stops SNTP until reboot after a KoD
([0080](../docs/decisions/0080-sntp-kod-backoff-and-fetch-independence.md);
policy in `src/sntp_policy.rs`, dating check `UtcTrustPreflight::projection_trusted`,
both host-tested). Event line: `SNTP KoD server= code= action=backoff|block
next_in_s= interval_s= uptime_s=`. The diagnostic adds `FETCH_UTC age_s=
uncertainty_us=` (the observation dating each fetch) and `FETCH_DEFERRED
no_trusted_utc` (a due fetch waiting for trusted UTC).

The fault build injects kiss codes in place of real queries (no packet is
sent; the display gets the same outcome as a real KoD): `FAULT_KOD` lists
`server:CODE:from-to` items (server 0 Cloudflare, 1 NIST; four uppercase
letters; uptime minutes, `to` exclusive; at most 8), and
`FAULT_KOD_BLOCK_MIN` shortens the 24 h `DENY`/`RSTR` block. It prints
`FAULT_INJECT_KOD` per injected reply.

```sh
REFRESH_MINUTES=120 FAULT_KOD="0:RATE:100-135,1:DENY:100-101,0:DENY:200-275,1:DENY:200-275" \
  FAULT_KOD_BLOCK_MIN=30 \
  cargo build --release --locked --bin overhead-s3-live-fetch-radar --features live-fetch-faults
# plus the Wi-Fi/site exports; save-image, gate, flash+capture as for the device app
```

**On-board evidence, 2026-11-07 03:01–08:06 UTC** (devbox, `/dev/ttyUSB0`,
MAC `80:45:6b:2a:45:08`, home site, the command above; image 1,399,760 B,
17.51%, gate passed; local `target/kod-faults.log`):

| Phase (uptime) | Result |
|---|---|
| Boot | fetch 156/156 dated by a fresh observation (uncertainty 21.9 ms, so the 1 h holdover, not the 100 ms budget, bounds trust) |
| A: Cloudflare `RATE` 100–135 min, NIST `DENY` once | Cloudflare backed off 128 → 256 → 512 → 1024 → 2048 s; NIST blocked 1,800 s, asked again after. Refresh at 7,263 s ran with no server answering that poll: `FETCH_UTC age_s=1280`, Success. Cloudflare answered again at 10,019 s and its interval halved back towards 64 s |
| B: both `DENY` 200–275 min | Both blocked at 12,006 s, again at 13,862 s and 15,722 s. Refresh at 14,506 s with both blocked: `FETCH_UTC age_s=2560 uncertainty_us=22069`, Success. Holdover lapsed: `NO FRAME` at 15,543 s. First reply after the window: `source=Some(Accepted)` at 17,578 s, radar frames resumed, no reboot |
| Whole run | 3 fetches, all 156/156; live heap 389,708 B throughout; no panic, no Wi-Fi drop |

Found on the way (fixed since by [0081](#nist-fallback-renews-utc-0081)):
`time.nist.gov` replies were mostly `Ignored(BadVersionOrMode)` (39 of 44;
5 observations), because NIST answers a version-4 request with version 3,
which the 0064 screen rejected. The NIST fallback therefore renews time only now and then,
and Cloudflare is effectively the one source. In phase A two NIST
observations happened to keep trust alive while Cloudflare was backed off.

## NIST fallback renews UTC (0081)

The reply screen accepts VN 3 as well as 4
([0081](../docs/decisions/0081-sntp-accept-version-3-replies.md); NIST answers
a VN=4 request with VN=3), and the trust gate keeps the tighter anchor
([0082](../docs/decisions/0082-keep-the-tighter-utc-anchor.md)): a looser
in-budget reply logs `LIVE_UTC source=Some(NoRenewal)` while the anchor is
trusted. Both are host-tested (`sntp_screen`, `utc_trust`).

Cloudflare is held off with the 0080 fault build (`DENY` re-injected for the
whole window; the 30-minute block makes Cloudflare's first real query come
after it):

```sh
REFRESH_MINUTES=120 FAULT_KOD="0:DENY:5-185" FAULT_KOD_BLOCK_MIN=30 \
  cargo build --release --locked --bin overhead-s3-live-fetch-radar --features live-fetch-faults
# plus the Wi-Fi/site exports; save-image, gate, flash+capture as for the device app
```

**On-board evidence, 2026-11-07** (devbox, `/dev/ttyUSB0`, MAC
`80:45:6b:2a:45:08`, home site; gate passed on every flashed image; local
`target/nist-fallback-run1.log`, `target/nist-fallback.log`,
`target/device-0082.log`):

| Run | Result |
|---|---|
| Run 1, 0081 only (16:29–17:40 UTC) | 61 NIST observations, 0 rejected; tracking held past the 1 h holdover on NIST alone. At uptime 4,184 s one NIST reply carried 99,906 µs; the gate replaced a 23.9 ms anchor with it, it lapsed ~4.7 s later: `NO FRAME` 4,188–4,248 s, then `Accepted` (selection cleared). Led to 0082; run stopped |
| Run 2, 0081 + 0082 (19:02–22:32 UTC; image 1,402,000 B) | Cloudflare `DENY`ed six times from 335 s; 174 NIST observations, 0 rejected (uncertainty median 27 ms, p90 50 ms, max 54 ms); gate: 1 `Accepted` at boot, 102 `Resynced`, 92 `NoRenewal` (longest kept-anchor run 10 polls, ~11 min); no `NO FRAME` after boot over ~3 h NIST-only. Refresh at 7,260 s dated by NIST `seq=114` (`FETCH_UTC age_s=0 uncertainty_us=24024`), 200, 156/156. Cloudflare answered again at 11,496 s. Live heap 389,708 B throughout |
| Device app (22:33 UTC; image 1,388,704 B) | stored `seq=8` older than 8 h → fetch 156/156 at 20 s, saved `seq=9`, radar up, next refresh 28,800 s |

Not shown on the board: a near-budget reply arriving under 0082 (none in
run 2; host test `looser_observation_keeps_tighter_anchor` replays run 1's
values).

## BLE setup service (0087)

Every runtime image (`device`, `live-fetch-radar`) carries the setup window
of [0087](../docs/decisions/0087-ios-setup-app-over-ble.md) (contract: its
[2026-11-09 note](../docs/decisions/0087-ios-setup-app-over-ble.md#note--2026-11-09-setup-protocol-and-firmware);
protocol crate `setup/`). Code: `src/setup_mode.rs` (the window: panel,
boot hold, GATT server, scan cache, test join), `src/setup_policy.rs` (rules,
host-tested), screens in `render/src/setup.rs`; the info page lives in
`live_radar.rs`'s display task. BLE is off outside the window.

- **Opens** with no Wi-Fi stored (`reason=Unconfigured`), after 5 min without
  joining (`JoinTimeout`, an RTC flag across the reset), or with IO6 held
  3 s at boot (`Button`; the Sharp says when to release). Held 10 s:
  factory reset (settings and elements erased, `FactoryReset`). A short
  press while tracking shows the info page for 30 s; since
  [0088](#selection-button-and-captured-knob-switch-0088) each press toggles
  it and presses are interrupt-captured.
- **Leaves** by a software reset 3 s after `Saved`, or, when settings exist,
  after 10 min without an answered request, even with a phone connected
  (`SETUP idle_timeout`, [0091](../docs/decisions/0091-setup-idle-counts-requests-not-links.md)).
  A link not paired 60 s after connecting or its last passkey is dropped
  (`SETUP pairing_timeout`).
- **Tests without saving** (protocol 2, [0087 note](../docs/decisions/0087-ios-setup-app-over-ble.md#note--2026-11-16-protocol-2-tests-a-password-before-saving)):
  `TestWifi` runs the same test join, logs `SETUP request test_wifi` and the
  usual `SETUP test …` lines (`result=Saved` there is the join passing,
  reported as `Joined`) and stays in setup. 2026-11-16, dev build
  `0.1.15-wifitest` on the Mac's port with the user's iPhone: two wrong
  passwords `WrongPassphrase` (4.9 s, 9.3 s), right ones joined (0.9 s,
  10.7 s), no link drop (`target/wifitest-setup.log`).
- Static heap is 96 KiB (was 128) so pairing's P-256 work fits the CPU0 stack.
- `setup-test` (dev only) prints `SETUP_PASSKEY n` on UART for
  [`overhead-setup-client`](../tools/README.md#overhead-setup-client). Test
  from empty settings on an image without `dev-settings` (it re-seeds an
  empty store); erase settings with `espflash erase-region --port
  /dev/ttyUSB0 --chip esp32s3 0x610000 0x10000` (add elements: `0x60000`).

```sh
. "$HOME/export-esp.sh"
OTA_CHANNEL=beta cargo build --release --locked --bin overhead-s3-device \
  --features device,setup-test
# save-image, image_gate.py and flash as in the OTA section, with --monitor
# into target/setup/runN.log; then from the repo root:
target/debug/overhead-setup-client --passkey-log firmware/target/setup/runN.log \
  configure --wifi-env firmware/.wifi.env --site-env firmware/.site.env
```

Lines (never the SSID, passphrase or site): `SETUP boot_hold held_ms= hold=`,
`SETUP factory_reset settings_ok= elements_ok=`, `SETUP open reason= name=
configured= internal_free=`, `SETUP ble_init`, `SETUP scan seen= listed=`,
`SETUP advertising`, `SETUP ble_connected`, `SETUP conn_params …`,
`SETUP passkey_shown`, `SETUP paired level= bonded=`, `SETUP
stack_watermark`, `SETUP request …`, `SETUP test channel=`, `SETUP test
result= ms=`, `SETUP saved; restarting`, `SETUP join_timeout after_s=300`,
`SETUP ble_disconnected reason=`, `INFO_PAGE presses= shown= loop_gap_us=
fetching=` (0088).

**On-board evidence, 2026-11-08/09** (devbox `/dev/ttyUSB0`, MAC
`80:45:6b:2a:45:08`, every image gated; client on devbox's AX200, which
needs its antennas: without them −95 dBm and half the sessions dropped):

| Check | Result |
|---|---|
| Empty settings | setup `Unconfigured`; paired `EncryptedAuthenticated`, not bonded, MTU 246, ~4.4 s; `Info` 88 ms; `Scan` 11 networks from the cache in 97 ms |
| Wrong passphrase | `WrongPassphrase` in 9.4 s, nothing stored, `Info.last_result` shows it |
| Correct network | `Saved` in ~11 s → reset → joined at 2 s, tracking |
| Join timeout | `dev-settings` seeded a network that doesn't exist: 300 s → reset → setup `JoinTimeout`; restored over BLE |
| IO6 3 s / 10 s (user) | `Button` setup, code on the Sharp, saved; 14–16 s hold erased settings and elements, `FactoryReset`, refetch 2.7 s |
| Info page (user) | shown and hidden; user: looked good |
| iPhone, nRF Connect (user) | paired with the Sharp's code, `GetInfo` answered; after a factory reset paired again with no "Forget"; iOS had cached the spike's table until Bluetooth was toggled; iOS applies the 4 s supervision timeout |
| Memory | BLE controller ~35 KB; internal free 92.5 KB paired, 122–127 KB in the normal run; pairing stack watermark 22.6 of 46.9 KB |
| Release 13 | user-signed `0.1.13-beta` (no dev features, no SSID/password/site in the `.bin`), from build 0 set up over BLE: found on `overhead.wesklein.com`, installed in 47 s, joined with the BLE-saved settings, `VALID` at 13 s |
| iOS app (user, 2026-11-09) | [setup app](../ios/README.md) 0.1.0 (1) from TestFlight: IO6 3 s, paired with the Sharp's code, wrong passphrase reported, correct one `Saved`, board restarted and tracked (no UART log: unplugging for the hold ends the capture) |
| Release 14 (2026-11-10) | user-signed `0.1.14-beta` (same firmware as 13) on `beta`: at the next 8 h refresh `offer=Install`, installed in 47.7 s; build 14 joined at 2 s on the app-saved settings, `VALID` at 13 s (devbox `target/setup/ios-ota14.log`) |

The board was left on release build 14 (`beta`, settings from the iOS app);
since 2026-11-12 it runs release 15 ([checklist evidence](#release-and-recovery-checklist)).
Not shown: BLE during a refresh (never: separate boot mode), a slow router
recovering inside the 10 min window, setup on another iPhone, the app's
reconnect after a link loss during `Testing`.

## Signed OTA and rollback (0083–0086)

Runtime images (device app, `live-fetch-radar`, its fault build) run on the
[0083](../docs/decisions/0083-flash-layout-for-updatable-devices.md) table
(`partitions.csv`: two 3 MiB slots, `otadata`, `settings`, `elements` at
0x620000) under the [0086](../docs/decisions/0086-bootloader-rollback-and-health-check.md)
rollback bootloader, with `dram2` as a second internal heap. Code:
`src/live_ota.rs` (device side, a child of `live_radar`), `src/otadata.rs`
(IDF `otadata` entries, written directly: `esp-bootloader-esp-idf` 0.6.0's
`Ota` picks a wrong slot after an erased `otadata`), `src/ota_health.rs`,
`src/settings_store.rs` (0084 record, format 2: channel, failed/pending
build, Wi-Fi, site), `src/release_keys.rs`; manifests in the `update/` crate
([0085](../docs/decisions/0085-signed-updates-and-release-keys.md)); signing
with [`overhead-release`](../tools/README.md#overhead-release).

**Bootloader.** `bootloader/build.sh` builds `bootloader/bootloader.bin` in
rootless Docker from `espressif/idf:v6.1` (digest in `image-digest.txt`, IDF
commit `fff9895c` in `idf-commit.txt`, sdkconfig and sha256 committed);
`CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE=y` is the only change from espflash's
bundled one, whose load segments it matches (so `dram2` stays reclaimable).
Rebuilding it is a toolchain change: ask first.

**Features.** `dev-settings` (developer only) seeds missing Wi-Fi/site
settings from `.wifi.env`/`.site.env`; without it an image holds no personal
data and, on an empty store, opens setup (`SETTINGS empty …`, then `SETUP
open reason=Unconfigured`, [0087](#ble-setup-service-0087)). `ota-dev` asks only the plain-HTTP LAN host `OTA_DEV_HOST`
(`a.b.c.d:port`) and also trusts `OTA_DEV_PUBLIC_KEY`, with `OTA_CHECK_MINUTES`
for frequent checks; `ota-drill` with `OTA_DRILL=panic|hang|no-manifest`
misbehaves on probation and never confirms itself. Neither is ever a release.
Build env: `OVERHEAD_BUILD` (decimal, default 0), `OVERHEAD_VERSION`,
`OTA_CHANNEL` (`beta`/`stable`, only seeds an empty store; default `stable`).

```sh
# local device build for the user's board (from firmware/)
. "$HOME/export-esp.sh"; set -a; . ./.wifi.env; . ./.site.env; set +a
OTA_CHANNEL=beta cargo build --release --locked --bin overhead-s3-device \
  --features device,dev-settings
ELF="$PWD/target/xtensa-esp32s3-none-elf/release/overhead-s3-device"
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --partition-table partitions.csv --skip-update-check \
  "$ELF" target/device.bin
python3 image_gate.py "$ELF" target/device.bin
# USB flash: writes ota_0, erases otadata; settings and elements are kept
timeout 600 espflash flash --port /dev/ttyUSB0 --chip esp32s3 --flash-size 8mb \
  --flash-mode dio --flash-freq 40mhz --bootloader "$PWD/bootloader/bootloader.bin" \
  --partition-table "$PWD/partitions.csv" --erase-parts otadata \
  --non-interactive --skip-update-check --monitor "$ELF" > "$PWD/target/device.log" 2>&1
```

**Release.** On devbox build without `dev-settings`/`ota-dev`/`ota-drill`,
with `OVERHEAD_BUILD=N` (larger than every published build) and no Wi-Fi/site
env; save-image and gate as above; check the `.bin` holds no SSID. The key
holder copies `N.bin` to the Mac into a throwaway directory (`cd "$(mktemp -d)"`,
not `~` or the repo), checks its sha256, runs `overhead-release
sign KEY beta N VERSION N.bin OUT` and `verify`, and copies `OUT` back; once
`pages` is pushed, delete that directory (its files are on `pages`). Push
`OUT/ota/…` to the orphan `pages` branch (signed files, a README and GitHub's
`CNAME` for `overhead.wesklein.com`: pull first and never force-push, or the
custom domain drops and devices fall back to raw GitHub).
The device checks after each element refresh (8 h) and installs once the knob
and selection button (BOOT before 0088) have been idle 2 min (`UPDATING n%`).

**Lines.** `OTA boot build= version= slot= state= outcome=Normal|Verify|RolledBack
channel= failed_build=`; `OTA_HEALTH met=… missing=0x..`, `valid`, `failed …;
resetting`; `OTA_CHECK source= build= offer=Install|Current|Failed|…` or
`failed=`/`refused=`; `OTA_INSTALL start|done|failed=`; `OTA_TLS`,
`OTA_CERT_DATES`, `OTA_HTTP`; `SETTINGS seeded from dev-settings|empty slot0= slot1=|unavailable|save_failed`.

**Drill host.** `python3 ota_dev_server.py ROOT [--port 8765] [--cut-after N]`
serves an `overhead-release sign` output over HTTP/1.0 (exact
`Content-Length`) on 0.0.0.0, and with `--cut-after` closes image downloads
after N bytes. Drill keys live in the ignored `target/ota-dev/`.

**On-board evidence, 2026-11-08** (devbox, `/dev/ttyUSB0`, MAC
`80:45:6b:2a:45:08`; `ota-dev` drill builds 1–8 on the LAN host, release
builds 9–12; each image gated):

| Check | Result |
|---|---|
| First USB flash (build 1) | bootloader/table/erased otadata; boot normal (IDF wrote an `ota_0` entry, `state=Valid`), settings seeded, one fetch, stored at 0x620000 |
| Update 1 → 2 | 20 s install, probation, five checks at 13 s, `VALID` |
| Panic drill (4) | panic on probation → bootloader aborted it → build 3, `failed_build=4`, then `offer=Failed` |
| Hang drill (5) | executor stopped after first frame → `RTCWDT_RTC_RST` → build 3, `failed_build=5` |
| No manifest (6) | 404 each poll → health failed at 10 min (`missing=0x10`) → `INVALID` → build 3 |
| Bad signature (7, other key) | `refused=signature`, no image request |
| Cut download | `--cut-after 700000`, then a server restart mid-transfer: `failed=truncated` twice, nothing activated, retried |
| Settings 1 → 2 (build 7) | format-1 record read, Wi-Fi/site seeded, `VALID` |
| Power cut (8) | user pulled USB mid-download → build 7, user saw the radar; next check reinstalled 8, `VALID` |
| Release 9 (bridge) | `OTA_CERT_DATES NoCertificate` on raw GitHub (TLS 1.3 hides certificates) → health failed → rolled back to 8; fixed by capping OTA TLS at 1.2 |
| Release 11 (bridge) → 12 | 11 verified the user-signed `beta` manifest on raw GitHub (Pages DNS not set: `failed=dns`, fallback), `VALID`; installed 12 from raw GitHub (37.5 s); 12 `VALID` |

The board was then left on release build 12 (`beta`, Wi-Fi/site from settings); since 2026-11-09 it runs build 13 ([setup service](#ble-setup-service-0087)).
Not shown on the board: `overhead.wesklein.com` (no CNAME yet), BLE with OTA,
a >1 release cycle offline, the `stable` channel. (Later shown: the custom
domain from release 13, BLE settings through OTA from 13; `stable` is
published from release 15 but no board runs it yet.)

## Release and recovery checklist

Exercised end to end on release 15 (2026-11-11/14, evidence below). Commands
for each step are in
[Signed OTA and rollback](#signed-ota-and-rollback-00830086) and
[`overhead-release`](../tools/README.md#overhead-release).

**Release N** (gift units use `stable`, the user's board `beta`; one image
signed for both channels when it goes to both):
1. **Build on devbox** from a clean, pushed `main`: no `dev-settings`,
   `ota-dev`, `ota-drill`; `OVERHEAD_BUILD=N` above every published build;
   `OVERHEAD_VERSION=0.1.N` (one image, so no channel suffix when it goes to
   `stable`); `OTA_CHANNEL` unset (`stable` seed for USB-flashed gift units);
   no Wi-Fi/site env. `save-image` to `target/release/N.bin`, keep the ELF as
   `target/release/N.elf`, run `image_gate.py`.
2. **Personal-data check:** none of `.wifi.env`'s `SSID`/`PASSWORD` or
   `.site.env`'s `SITE_*` values occur in `N.bin`
   (`grep -c -a -F -- "$value" N.bin` prints `0` for each; never echo the
   values). A 1–3 digit `SITE_ALT_M` matches by chance (release 14 holds the
   same count); SSID, password, latitude and longitude must be `0`.
3. **Hand over:** `sha256sum N.bin`; the key holder copies it to the Mac and
   checks the hash.
4. **Sign on the Mac:** `git pull` first (from release 16 `sign` refuses an
   image whose `overhead-build:` marker is missing or isn't N; an older
   checkout signs without checking). `sign KEY beta N 0.1.N N.bin OUT`, and for `stable`
   `sign KEY stable N 0.1.N N.bin OUT` into the same `OUT`; `verify` each
   manifest (the Mac shell is zsh: wrap the cargo command in a function, not
   a `$R` string, and keep `#` comments out of pasted lines). Copy `OUT`
   back to devbox.
5. **Publish:** in a `pages` worktree, pull, add `ota/images/N.bin` and the
   manifests, commit, fast-forward push. Never force-push (it drops the
   `CNAME` and devices fall back to raw GitHub).
6. **Check:** after ~10 min (Pages CDN), fetch
   `https://overhead.wesklein.com/ota/CHANNEL/manifest.json` and run
   `verify` on devbox with the public key from `src/release_keys.rs`; the
   board takes it after its next 8 h refresh (`OTA_CHECK … offer=Install`,
   `OTA_INSTALL done`, `OTA_HEALTH valid`).

**Key.** The private key lives only on the key holder's Mac
(`~/overhead-release.key`) with a 1Password backup; it never touches devbox
or GitHub. Losing both means no device can be updated without USB (a new key
needs a USB flash of every unit).

**USB recovery flash** (with the board on the Mac: `./mac.sh flash N.elf LOG`, the same command; a unit that won't boot or update; keeps `settings`
and `elements`, so Wi-Fi, site and channel survive): with the board on the
UART port, flash the newest published image's ELF exactly as the "USB flash"
command above (bootloader, partition table, `--erase-parts otadata`, app in
`ota_0`). After a factory reset, or for a board whose store is empty, the
image's `OTA_CHANNEL` seed decides the channel: build with
`OTA_CHANNEL=beta` for the user's board. Then confirm `OTA boot … slot=Some(0)
state=Valid channel=…`, Wi-Fi, `STORE loaded` and `RADAR first_frame`. A
USB-flashed image is `Valid` at once (no probation), so its first
`OTA_CHECK` comes at the next 8 h refresh.

**Evidence, 2026-11-11/14** (devbox `/dev/ttyUSB0`, MAC `80:45:6b:2a:45:08`,
host-timestamped capture `target/soak/soak72.log`, 72 h from 11-11 03:05 UTC):

| Check | Result |
|---|---|
| Soak on app-saved settings | 9 refreshes, 8 h 0–1 min apart, all `Success`; no `NO FRAME`, panic, rollback, `failed=`/`refused=` or KoD; one reset (the OTA one). Uptime continuous: build 14 from 11-10 07:45 to 48 h, build 15 from install to 40 h |
| Release 15 (`0.1.15`, same firmware as 14) | built at `d7e8f03`, gate pass, personal-data check 0 (2-digit altitude matches by chance, 42× in 14.bin too), user-signed for `beta` and `stable` into one `OUT`, verified on devbox against `release_keys.rs`; `pages` `71303d8`; live manifests and image served and verified ~1 min after push |
| OTA install mid-soak (11-12 07:51 UTC, ~29 h in) | `offer=Install`, `OTA_INSTALL done ms=51111`, build 15 in slot 1 `PendingVerify`, Wi-Fi 2 s, `VALID` 13 s, channel `beta`, settings intact |
| USB recovery flash (11-14 03:07 UTC) | `target/release/15.elf` with the command above: build 15 in `ota_0`, `state=Valid`, `channel=beta`, Wi-Fi 2 s from saved settings, stored elements loaded, first frame (`target/soak/recovery15.log`) |

Not shown: a board on `stable` installing a release (gift units), recovery
of a board with an empty store.

## Gift unit build and check

Per unit, before it is given (written 2026-11-14; **not yet exercised**, it
needs a spare board). A gift unit runs a published release on `stable`,
leaves the bench unconfigured and is set up by its recipient with the
[iOS app](../ios/README.md).

**Release.** Use the newest published release that carries
[0088](#selection-button-and-captured-knob-switch-0088) (release 16 or
later): release 15 and earlier select with BOOT and ignore IO7. Its ELF is
`target/release/N.elf` from the [checklist](#release-and-recovery-checklist)
(built without `OTA_CHANNEL`, so an empty store seeds `stable`).

**Wiring** (ESP32-S3-DevKitC-1-N8R8, direct, internal pull-ups):

| Part | Connections |
|---|---|
| Sharp #4694 | GND→G, Vin→5V, Clk→IO13, DI→IO11, CS→IO10 |
| Encoder | A→IO4, C (middle)→G, B→IO5; switch→IO6 and G |
| Selection button | IO7 and G, on **diagonally opposite** legs of a four-leg tactile button |

**Flash** (from `firmware/`, board on the UART port; confirm the port and
MAC first and write the MAC in the table below). A new board is erased
whole, so no factory test app, settings or elements survive. On the Mac
([Board on the Mac](#board-on-the-mac-macsh)), `./mac.sh info` with
`OVERHEAD_MAC` unset prints the MAC (and refuses, since it isn't the user's
board), then:

```sh
python3 image_gate.py target/release/N.elf target/release/N.bin
./mac.sh erase MAC
OVERHEAD_MAC=MAC ./mac.sh flash target/release/N.elf target/gift/MAC.log 300
```

With the unit on devbox's USB instead:

```sh
. "$HOME/export-esp.sh"
espflash board-info --port /dev/ttyUSB0 --chip esp32s3 --after no-reset
timeout 300 espflash erase-flash --port /dev/ttyUSB0 --chip esp32s3 \
  --non-interactive --skip-update-check
python3 image_gate.py target/release/N.elf target/release/N.bin
timeout 600 espflash flash --port /dev/ttyUSB0 --chip esp32s3 --flash-size 8mb \
  --flash-mode dio --flash-freq 40mhz --bootloader "$PWD/bootloader/bootloader.bin" \
  --partition-table "$PWD/partitions.csv" --erase-parts otadata \
  --non-interactive --skip-update-check --monitor "$PWD/target/release/N.elf" \
  > "$PWD/target/gift/MAC.log" 2>&1
```

**Bench check** (~5 min, UART capture running; the bench Wi-Fi is any
network the tester's iPhone can set):

1. **Unconfigured boot:** `OTA boot build=N … state=Valid channel=stable`,
   `SETTINGS empty …`, `SETUP open reason=Unconfigured`; the Sharp shows
   the setup screen.
2. **Set up with the app:** pair with the Sharp's code, save the bench
   Wi-Fi and a site → `SETUP saved; restarting` → Wi-Fi joined, `LIVE_FETCH
   loaded`, `RADAR first_frame`; radar on the Sharp.
3. **Inputs:** the knob zooms; each button press logs `SELECT … applied=1`
   (the marker changes when two or more are in view); a knob press opens the
   info page (`INFO_PAGE … shown=true`), the next closes it.
4. **Factory reset:** unplug, hold the knob switch while plugging in until
   the Sharp shows the factory reset (≥10 s; 14–16 s on the user's board) →
   `SETUP factory_reset settings_ok=true elements_ok=true`, `SETUP open
   reason=FactoryReset`. The channel returns to `stable`.
5. **Hand over unplugged.** No panic, `NO FRAME` after the first frame, or
   rollback in the log. The recipient sets it up; it takes the next `stable`
   release after its first 8 h refresh.

| Unit (MAC) | Release | Date | Result |
|---|---|---|---|
| — | — | — | — |

## Hardware and permission

Ask before installing or changing toolchains. Flashing, resetting and
capturing need no per-flash approval ([0069](../docs/decisions/0069-flash-without-per-flash-approval.md));
confirm the port and gate the image first. Flashing replaces the existing
application and writes a bootloader/partition table; it is not a read-only test.
The capture command resets the device but does not flash it.

Confirmed board: ESP32-S3-DevKitC-1-N8R8, WROOM-1 module, S3 revision v0.2,
8 MB flash, 8 MB PSRAM (the device app keeps the `visual` group there,
[0077](../docs/decisions/0077-m2-operating-budget-visual-group.md); the kernel
benchmark uses none). Use a data-capable micro-USB
cable in the board's **UART** port (CP2102N bridge). Direct laptop connection
worked during bring-up; the Anker dock path did not enumerate. No drivers were
installed. The Sharp display is now connected via breadboard jumpers as above;
soldering is not required for the current prototype.

```sh
espflash list-ports --skip-update-check
# Replace the example port below with the currently enumerated UART port.
espflash board-info --port /dev/cu.usbserial-110 --non-interactive --skip-update-check
```

## Toolchain/build

This directory is an isolated workspace with its own checked-in `Cargo.lock`,
Xtensa target configuration, and local `esp` toolchain selection. The host
workspace keeps stable Rust; firmware dependencies/features cannot leak into it.
Run target builds **from this directory**, so Cargo reads its `.cargo/config.toml`.

Installed for initial measurements: espup 0.18.0, espflash 4.6.0, Xtensa Rust
1.97.0.0. `esp-hal` 1.2.2 provides startup/clock/timer, `esp-println` 0.18.0
provides UART diagnostics, and `esp-bootloader-esp-idf` 0.6.0 provides the image
descriptor. The existing sgp4/serde_json parsers run only in the host build script.

```sh
# Once, with permission:
cargo install espup --version 0.18.0 --locked
cargo install espflash --version 4.6.0 --locked
espup install --targets esp32s3 --toolchain-version 1.97.0.0

# From repository root; source the exports in each build shell:
. "$HOME/export-esp.sh"
cd firmware
BENCH_SUITE=baseline BENCH_SAMPLES=5 cargo build --release --locked
```

`BENCH_SUITE` and `BENCH_SAMPLES` are build-time experiment selections, not
runtime device settings. Defaults are `baseline` and `5`; samples must be 1–5.
Unknown suites/invalid samples fail the build. Cargo tracks changes to both
variables; always specify them explicitly when preparing a measurement image.

Default Cargo release optimization (`opt-level=3`), debug information retained,
no LTO override. The initial GCC linker emits a LOAD-segment RWX warning for the
embedded image; build/flash/boot succeed. No warning suppression is configured.

## Flash and capture

Before any flash, run the [offline image gate](IMAGE_GATE.md)
on the exact normal-build ELF/application image. The local DROM linker remedy
passes offline and in [two corrected catalogue-memory captures](../docs/evaluations/s3-drom-diagnostic.md#corrected-normal-build-target-verification);
the corrected kernel binary has not been boot-tested. Stress-test artifacts are
not flash candidates.

First generate expected work counts on the **host**, from repository root.
Use the same revision, suite, and sample count as the target build; retain the
manifest with the ELF/hash and raw log. This executes the workloads but does not
claim independent numeric accuracy or measure target performance.

```sh
cargo +stable run --release --locked --manifest-path firmware/Cargo.toml \
  --example expected -- scaling-16 3 > /tmp/overhead-scaling-16-expected.txt

. "$HOME/export-esp.sh"
cd firmware
BENCH_SUITE=scaling-16 BENCH_SAMPLES=3 cargo build --release --locked
```

From `firmware/`, after checking the port (with the board on devbox's USB;
on the Mac use `mac.sh`):

```sh
espflash flash --port /dev/cu.usbserial-110 --chip esp32s3 \
  --flash-size 8mb --flash-mode dio --flash-freq 40mhz \
  --non-interactive --skip-update-check \
  target/xtensa-esp32s3-none-elf/release/overhead-s3-benchmark

python3 capture.py --port /dev/cu.usbserial-110 \
  --elf target/xtensa-esp32s3-none-elf/release/overhead-s3-benchmark \
  --expected /tmp/overhead-scaling-16-expected.txt \
  --output /tmp/overhead-scaling-16-run-1.log --timeout 900
```

`capture.py` uses only the Python standard library plus installed `espflash`.
It resets the board, saves unmodified serial/monitor output, stops at the DONE
line, and validates the suite, settings, sample matrix, and host work counts.
V2 output requires `--expected`; malformed manifests fail before board reset.
It refuses to overwrite a result and retains partial logs on timeout/failure.
The original V1 14 × 5 capture remains valid without a manifest.

Default timeout is 600 seconds, **not sufficient for every suite**. Use the
suggestions below and retain timeouts as failed/partial runs, never as complete
measurements. Repeat with a different output filename. After DONE the firmware
spins; resetting reruns the selected suite. Display and second CPU core are unused.

## Measurement contract

The build script reads the **original** [ISS OMM](../core/tests/fixtures/iss-25544.json)
and [HEO/GEO/GNSS TLEs](../tools/fixtures/pass-intervals.tle), emits lossless numeric
element fields, and precomputes the same timestamp grids as the
[host benchmark](../tools/README.md#overhead-benchmark). Fixture provenance stays
in [tools/fixtures/README.md](../tools/fixtures/README.md). No live data is fetched.

### Separately runnable suites

| `BENCH_SUITE` | Workloads | Coverage | Suggested capture timeout |
|---|---:|---|---:|
| `baseline` | 14 | Original four classes × propagation/geometry/search, plus four-slot mixed geometry/search | 600s |
| `scaling-16` | 2 | 16 repeated slots, full geometry and 24h search | 900s |
| `scaling-64` | 2 | 64 repeated slots, full geometry and 24h search | 2400s |
| `ages-leo`, `ages-heo`, `ages-geo`, `ages-gnss` | 14 each | Geometry and 24h search at grid centers −30, −7, −1, 0, +1, +7, +30 days from epoch | 1800s |
| `search-leo`, `search-heo`, `search-geo`, `search-gnss` | 12 each | 1h/24h × 5/30/60s detection × 250ms/5s tolerance | 1800s |

Timeouts are conservative capture allowances, **not measured runtime bounds**.
Start with three samples and one suite at a time to avoid another unbounded
combined matrix. One untimed warm-up is always additional to the selected samples.
The 16/64-slot suites still reference only four satellite instances, not a
larger distinct working set. Additional distinct populations are measured only in
the separate [frozen M2 harness](M2_CAPTURE.md), not added to these historical suites.

### Timing and input contract
- Each tracking invocation evaluates 1,441 timestamps at 60s spacing, from the
  selected epoch-relative grid center −12h to +12h. Mixed slots iterate times
  then satellites. Signed age describes that **center**, not the start; e.g.
  +30 days evaluates +29.5 through +30.5 days. No elements are mutated.
- Search starts at the same grid start (center −12h), including 1h searches.
  Unless swept explicitly: 24h, 60s detection, 5s crossing tolerance. All use a
  10° threshold and Colorado observer (39.007°, −104.883°, 2.187 km). These are
  **experiment settings**, not production defaults or an accuracy guarantee.
  Each fixture has a different epoch; this is not a common-UTC catalogue.
  Old-element completion does not establish usable orbital accuracy/freshness.
- Each workload has one untimed warm-up, then 1–5 samples of one invocation each.
  `esp_hal::time::Instant` measures elapsed microseconds at 240 MHz CPU clock.
  Inputs/outputs use `black_box`; UART printing is outside timed sections.
  All searches must complete and repeated work counts must agree.
- Initialization, parsing, timestamp/config preparation, and reporting are not
  timed. Age grids are generated from original fixtures by the host build script,
  in separate flash-rodata statics (69,168 bytes per age on S3). All seven remain
  linked in the measured V2 scaling-16 build, even though only age zero is used. Four satellite instances, prepared slot indices/configs,
  and search state use internal RAM/stack. No heap or PSRAM is used. Reported
  type sizes are an inventory only, not peak stack or a catalogue memory budget.
- The host's calibration/batching is not reproduced: device invocations are
  already long. `baseline` operations/counts still match the original 14 host
  rows; V2 changes harness/reporting and is not the original measured binary.
  Record new ELF section sizes/hash and build context for any new measurement.
- Full geometry means ECEF rotation and observer look angles, not geodetic
  satellite altitude, rendering, or a complete application tick.
- All prepared suites now have validated target captures; see the linked results.
  Age and search-setting sweeps are separate, not a Cartesian product. Broader
  distinct-population, heap/stack and independent numerical evidence now lives in
  the separate [frozen M2 suite](M2_CAPTURE.md).
  The first larger-matrix capture timed out; partial
  exploratory output is not part of the published final-build runs.

V2 `CASE` records retain operation, orbit class, slot count, signed center age,
window, detection interval, and tolerance. Tracking uses zero detection/tolerance
fields (not applicable). `ROW` records identify the case/sample and retain raw
microseconds plus evaluation/search/pass counts. The host example shares the
suite definitions, executes them, and emits `CASE`/`WORK` expectations. Capture
requires ordered, complete records matching that external manifest; target
warm-up counts alone are not the validation oracle.

This measures costs, not independent target numeric accuracy. Matching pass and
evaluation counts does not prove equality of positions/crossing timestamps or
that all short passes were detected. Never turn a sample maximum into a
worst-case bound, or `size_of::<Satellite>()` into a catalogue memory budget.

## Offline checks

From the **repository root**, without connecting/resetting/flashing hardware:

```sh
cargo check --workspace
cargo test --workspace
cargo +stable test --manifest-path firmware/Cargo.toml --lib --locked
cargo +stable clippy --manifest-path firmware/Cargo.toml --lib --tests --examples -- -D warnings
cargo fmt --manifest-path firmware/Cargo.toml -- --check
python3 -m unittest discover -s firmware -p 'test_capture.py'
python3 -m unittest discover -s firmware -p 'test_catalogue_capture.py'
python3 -m unittest discover -s firmware -p 'test_image_gate.py'
```

These Python suites cover the historical protocols and image gate without the
frozen M2 source guard. Full discovery (`test_*.py`) also includes
`test_m2_capture.py` and must run in the
[pinned historical export](../docs/evaluations/s3-m2/README.md#offline-validation-and-table-regeneration),
not the evolving workspace. Keep current-source checks separate from frozen
capture reproduction; do not bypass or renew the freeze to make a test pass.

Run the separate [image regression runner](IMAGE_GATE.md#complete-regression-run--no-hardware)
to check both binaries, alignment stress, and a source-generated negative control.
It uses only the installed toolchain and never accesses hardware.

For the optional memory binary's additional feature tests and protocol, see
[CATALOGUE_MEMORY.md](CATALOGUE_MEMORY.md); the commands above retain default features.

Firmware library tests check fixture IDs, signed-age grids, preserved baseline
counts, all 122 workload invocations/repeatability, suite selection, orthogonal
search settings, and repeated-slot accounting. Capture tests cover legacy V1,
V2 manifests/settings/sample counts, missing/duplicate/reordered/malformed
records, version mismatches, and failure markers. No timing assertions are used.
