# ESP32-S3 kernel benchmark

Early M2 measurement firmware, **not the device application**. The default kernel
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

## Frozen M2 harness and numerical checks

The separate [frozen M2 harness](M2_CAPTURE.md) now implements the timed and
numerical/stack diagnostic images for all 20 frozen cases. Both exact normal
images pass offline gates and now have two accepted captures each
([results and limits](../docs/evaluations/s3-m2.md)). Its commands/protocol are
separate from the historical suites documented below.

The opt-in `numerical-reference` library feature provides the same allocation-free
75-row checker for host and the new target diagnostic example. Historical
measurement suites are unchanged. Run the checker from the repository root:

```sh
cargo +stable run --offline --locked --manifest-path firmware/Cargo.toml \
  --example numerical_expected --features numerical-reference
```

[Reference selection/provenance and tests](../tools/fixtures/m2-numerical/README.md);
[frozen expanded-suite annex](../docs/evaluations/m2-frozen-manifest.md).
All 75 rows pass in both approved diagnostic boots. Future flashes/resets still
require approval; exact image gates and compiled-frame review are recorded in
[M2_CAPTURE](M2_CAPTURE.md).

## Isolated coastline representation feasibility image

The single [0056 candidate check](../docs/evaluations/m4-coastline-representation-target.md)
is complete. `overhead-s3-coast-representation` is a separate opt-in measurement
binary, not the device app or a replacement for the earlier Stage A image. Features
`coast-representation` / `coast-representation-on` select matched off/on builds.
It uses a 1-bit RAM target, existing app fixtures and the
[measurement-only packed asset](fixtures/README.md); no Sharp driver or PSRAM.
[Exact build/capture commands, image identities and offline audits](../docs/evaluations/m4-coastline-representation-s3/README.md)
are retained with the evidence. Future builds/flashes/runs require new approval;
the completed check does not authorize continuing optimization or adopting it.

## Prepared coastline layer image

`overhead-s3-coast-prepared` (feature `coast-prepared`) measures the 0058/0059
prepared, cached wide coastline layer: preparation, cached and fresh-raster coast,
and complete frames, with on-target exactness checks against the candidate.
[Results](../docs/evaluations/m4-coastline-prepared-target.md) and
[commands/evidence](../docs/evaluations/m4-coastline-prepared-s3/README.md).
`representation_capture.py --marker OVERHEAD_S3_COAST_PREPARED` captures it.
Future builds/flashes require new approval.

## Isolated Sharp wiring/refresh test

`overhead-s3-sharp-test` (`--features sharp-test`) is an opt-in, allocation-free
panel test, **not the radar or a 20 Hz demonstration**. For the Adafruit #4694
breakout wired to the DevKitC-1: GND→G, VIN→5V, Clk→IO13, DI→IO11,
CS→IO10. SPI2 sends mode-0, LSB-first at 2 MHz with manual **active-high** CS.
It clears white at boot, then refreshes the full 400×240 panel at ~2 Hz,
alternating black/white quadrant bands every two seconds with a black border.
Serial VCOM alternates at ~1 Hz. The image logs `SHARP_FRAME phase=… transfer_us=…`
over UART; the initial target transfer measured ~52 ms, which **does not**
establish a 20 Hz whole-device cadence. No framebuffer, tracking, coast,
network, knob or BOOT-button behavior is included. See the [Sharp datasheet](https://cdn-learn.adafruit.com/assets/assets/000/094/215/original/LS027B7DH01_Rev_Jun_2010.pdf)
and [Adafruit driver reference](https://github.com/adafruit/Adafruit_SHARP_Memory_Display).

After approval for a new target build/flash, from `firmware/` with the existing
Espressif environment (`. "$HOME/export-esp.sh"`):

```sh
BENCH_SUITE=baseline BENCH_SAMPLES=5 cargo build --release --locked \
  --bin overhead-s3-sharp-test --features sharp-test
E=target/xtensa-esp32s3-none-elf/release/overhead-s3-sharp-test
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --skip-update-check "$E" target/sharp-test.bin
python3 image_gate.py "$E" target/sharp-test.bin
# After explicit flash approval, replace <UART> with the verified port:
espflash flash --port <UART> --chip esp32s3 --flash-size 8mb \
  --flash-mode dio --flash-freq 40mhz --non-interactive --skip-update-check "$E"
espflash monitor --port <UART> --elf "$E" --non-interactive --skip-update-check
```

The 2026-10-25 first build/flash used `/dev/cu.usbserial-21410` with the user's
approval and passed the offline image gate; UART showed clean boot and alternating
phases, with no panic. The user then confirmed the alternating pattern on the
physical #4694 panel. That pattern test alone established neither radar nor
target-cadence acceptance.

## Fixed-fixture radar-on-Sharp candidate

`overhead-s3-sharp-radar` (`--features sharp-radar`) is a separate **opt-in**
image. It uses the pinned real ISS OMM fixture at its epoch, places the fixed
observer at the computed ISS ground point (so the selected marker is visible),
prepares `CoastLayer` once in static internal memory and renders the shared
352×240 wide radar + 48px labelled strip to a static 400×240, 1-bit framebuffer.
Framebuffer ink is inverted on transfer to Sharp; the full static frame is
resent every 500 ms to keep serial VCOM alternating, without repainting or
simulating new time. UART prints `RADAR_READY` and `RADAR_TRANSFER`. No live
clock, motion, network, buttons, zoom input, production cadence or sustained
operation is claimed. The target passed its offline image gate and was flashed
with user approval on 2026-10-25; UART reported `RADAR_READY selected=Some(25544)`
(render ~155 ms) and repeated ~52 ms transfers without a panic. **The user subsequently confirmed the expected physical view:
static wide radar, ISS marker and narrow labelled strip.** This accepts only
the fixed-time image,
not live motion or target cadence. The feature reuses the earlier
coast-measurement fixture implementation for this one bounded image, not as a
production device architecture.

With the existing environment, from `firmware/`:

```sh
BENCH_SUITE=baseline BENCH_SAMPLES=5 cargo build --release --locked \
  --bin overhead-s3-sharp-radar --features sharp-radar
E=target/xtensa-esp32s3-none-elf/release/overhead-s3-sharp-radar
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --skip-update-check "$E" target/sharp-radar.bin
python3 image_gate.py "$E" target/sharp-radar.bin
# For subsequent flashes, only after new authorization and checking the UART port:
espflash flash --port <UART> --chip esp32s3 --flash-size 8mb \
  --flash-mode dio --flash-freq 40mhz --non-interactive --skip-update-check "$E"
```

## Deterministic radar replay (v2 physically accepted)

`overhead-s3-sharp-replay` (`--features sharp-replay`) preserves the accepted
static radar binary as a separate image. It uses the same pinned ISS elements,
fixed observer and prepared coast, but a **5° vertical FOV** and seven frames
at fixture UTC epoch +0, +50, …, +300 seconds, one frame per 500 ms of real
time. It repaints the 12 KB static framebuffer and transfers all 240 rows each
tick, alternating serial VCOM; resetting to +0 every 3.5 seconds is deliberate.
It is **not live time, natural orbit speed, smooth motion or a 20 Hz device**.
If work exceeds 500 ms, the loop slips rather than dropping a frame. UART logs
per-frame update/render/transfer/total times, selection and checksum. Host tests
require the ISS to remain selected in every frame, move >9 pixels vertically
per step and >60 pixels across the view, produce different adjacent frames and
return exactly to the first image. The exact gated v2 ELF (SHA-256
`9497f6c68b4b2ad749c6e57ec78f69dbfadf562afaccaf6cabc596dab1d7f9dc`)
was flashed with user approval on 2026-10-25. UART showed seven different
checksums, deterministic wrap and no panic; after the first frame, update
~3.2 ms, coast-cached but period-uncached render ~141 ms, full-frame
transfer ~52 ms, total ~196 ms.
**The user confirmed visible marker movement and the repeated jump back to its
starting position on the physical panel.** This accepts only accelerated replay
output; these samples do not prove sustained latency, safe stack capacity or a
20 Hz budget.

Historical v1 (+0..+6 seconds at 20° FOV) was also flashed with approval, but
showed only two image checksums; the user reported that it **looked static**.
That is a failed visual-motion check, not acceptance. Future flashes need new
approval.

With the existing environment, from `firmware/`:

```sh
BENCH_SUITE=baseline BENCH_SAMPLES=5 cargo build --release --locked \
  --bin overhead-s3-sharp-replay --features sharp-replay
E=target/xtensa-esp32s3-none-elf/release/overhead-s3-sharp-replay
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --skip-update-check "$E" target/sharp-replay-v2.bin
python3 image_gate.py "$E" target/sharp-replay-v2.bin
# For subsequent flashes, only after new authorization and checking the UART port:
espflash flash --port <UART> --chip esp32s3 --flash-size 8mb \
  --flash-mode dio --flash-freq 40mhz --non-interactive --skip-update-check "$E"
```

## Sharp full-frame cost diagnostic (flashed; short UART capture)

`overhead-s3-sharp-profile` (`--features sharp-profile`) is a **separate**
opt-in image. It shows the same 5° accelerated ISS replay as v2 using the same
2 MHz SPI settings. After each transferred frame, it measures one offscreen
control, cycling through the RAM target's full-screen `clear`, cached prepared
coast plus a fast RAM clear, and a shared full frame with no coast. The control
pixels are never transferred; the next normal draw clears them. Host tests
verify the following real frame is byte-identical after each control. UART
reports stage times, selection, frame checksum and the named control timing;
these controls are not additive components of the normal draw, and the coast
control includes the fast RAM clear. Profiling adds work after transfer, and
the main loop cannot preempt a long control to maintain VCOM: this is a
**short diagnostic only**, not an endurance image or shipping scheduler. The
Sharp wire format requires 12,482 bytes for a full frame; at 2 MHz the bit-time
floor alone is ~49.9 ms, before SPI overhead and rendering.

This identifies the next area to isolate, not an optimization or a 20 Hz
result. In a short 2026-10-25 UART capture after an approved flash, every frame
retained the v2 seven-checksum loop, selection and reset without a panic.
Steady measured update was ~3.2 ms, full cached render ~141 ms, full SPI
transfer ~52.4 ms and combined work ~196.6 ms; first frame ~205 ms.
The separate controls repeated at ~13.7 ms for a full-screen clear (96,000
pixel writes), ~1.09 ms for cached coast including the fast RAM clear (~1,133
writes), and ~138.1 ms for a full coast-free frame (~97,150 writes). **These
controls are not additive parts of the normal render.** Most cost remains in
the coast-free frame; clear alone and coast alone do not explain it. No finer
component attribution, extended operation, safe-stack bound or 20 Hz claim
follows. The user confirmed that the panel still shows the moving replay while
this diagnostic runs.

Host tests, release build and offline image gate passed for ELF SHA-256
`0ce6a5b3916a0015d7bb0ab08de8bf4145c402ed9a368ea9283e62c0629218fe`
(gated image SHA-256 `06e6843083b890e56c7ff437627d14661b699be823d239df9e11c2bedead5bab`).
Flashed with explicit approval after rechecking the same port, ELF and gate;
the user confirmed moving replay on the panel. This **v1 cost diagnostic** has
since been replaced by the stage-trace v2 below. Any further flash (including
return to the original replay binary) needs fresh approval and checks.

### Stage-trace follow-up (flashed; short UART capture)

The previously built **v2** of this same separate opt-in diagnostic samples
elapsed timestamps only in its offscreen `frame_no_coast` control at the shared
compositor's clear, coast (disabled), Earth outline, radar cross/N/indicators,
markers, selected label, category, data glyphs and status boundaries. UART
reported the per-stage differences **after** rendering; no debug printing
occurs inside a stage. This is the existing coast-free draw, not a duplicate
renderer or a timed subtraction from v1. Timestamp sampling adds measurement
overhead; stage results apply only to this diagnostic build. Production
calls the same compositor with an optimized-away no-op callback. Host tests
assert nine ordered callbacks and exact framebuffer parity with the old
coast-free control, selected and empty scenes, and the next normal full frame
at all seven fixture times. With fresh approval, the port, exact ELF and gate
were rechecked, v2 was flashed, and a short UART trace captured. The user
confirmed moving replay on v2; the board later ran v3, cache v1 and now
runs the selective-line diagnostic below.

Host tests, release build and offline gate passed for v2 ELF SHA-256
`d1a12b1c57285edf19be6df9987f7009831d68e458ba147a333c9cd9f5e4ed2b`
(gated image SHA-256 `f1f0ba7e4c7772c475d1390efb4921157d935f9717057c7ab531a7925ac66dfc`).
UART booted without reported panic, selected ISS stayed visible in telemetry,
and all seven normal-frame checksums repeated exactly. Steady normal render was
~141 ms, full transfer ~52.3 ms, and total update/render/transfer ~196.4 ms.
Repeated **offscreen coast-free** stage intervals were ~13.8 ms clear, ~6.1 ms
Earth outline, ~0.5 ms cross/N/indicators, ~0.4 ms markers, ~0.3 ms label,
~0.2 ms category, **~116.2 ms data glyphs**, and ~0.15 ms status; disabled
coast and timestamp/callback gaps were each ~0.03 ms. Intervals describe this
instrumented control, not independent performance guarantees or an additive
breakdown of the normal cached frame. The data glyphs dominate this scene.
`render/src/glyphs.rs::period_rows` evaluates 4,097 samples including `libm`
trigonometry; this is a **source-level hypothesis** for the glyph cost, not
isolated period timing or an approved optimization. The subsequent v3 trace
below measures the period field; neither profile is the current board image.
At 2 MHz full-frame transfer alone is already ~52 ms; no 20 Hz or endurance
claim.

### Individual-glyph trace v3 (flashed; short UART capture)

The current source builds **v3** of the same opt-in profile image. Within the
existing offscreen `frame_no_coast` control it records five additional
boundaries in the **shared** strip code: separator, elevation, altitude, speed
and period. It retains the v2 nine-stage trace, normal replay transfer and
500 ms fixture tick. All timestamps are captured in memory; UART printing
occurs only after the control. The measured field interval includes its label,
geometry, raster writes and callback overhead; this is not a standalone `libm`
benchmark or a proposed pixel change. Host renderer
tests verify the five callbacks in order and byte-identical traced/untraced
coast-free output with and without selection; firmware tests verify that all
seven replay frames redraw identically after each offscreen control. Existing
v2 was visually confirmed; v3 replaced it and the user confirmed the moving
replay remained visible. Later separate cache and selective-line diagnostics
replaced v3; the board now runs the selective-line image. Any further flash
requires fresh approval and checks.

Host tests, target release build and offline gate passed for v3 ELF SHA-256
`617bba88a936b4125b3444a4b84072728822cac836f947b35e13fcd8dcb675c7`
(gated image SHA-256 `5c03957f6559ac4b7a1c8e4b4e6e13e77dab525c4684da5560d1f45f4643571d`).
After explicit approval and fresh port/ELF/gate checks, v3 flashed and booted
without reported panic. Its seven normal-frame checksums and selection repeated
exactly; steady render ~141 ms and 2 MHz full transfer ~52.3 ms. Repeated
**offscreen coast-free** traces show the data-glyph stage ~116.5 ms: separator
~0.41 ms, elevation ~0.38–0.61 ms, altitude ~0.22–0.24 ms, speed ~0.36–0.39 ms,
and **period ~115.02 ms**. This is the period field's full current draw
(label, 4,097 samples and raster), not an isolated `libm` benchmark. It is
~99% of this scene's glyph stage. An opt-in single-raster cache for unchanged
mapped period turns is now implemented and host parity-tested in
`overhead-render` (`PeriodCache`, `draw_cached`, `draw_at_cached`). Ordinary
replay firmware and simulator still use the uncached draw, so this v3 capture is
**not cached target evidence**; the separate comparison below tests the cache. Even eliminating the measured
period draw would not make a 2 MHz full-frame transfer fit 50 ms. No sustained
cadence, stack-safety or production behavior follows.

From `firmware/`, with the existing environment:

```sh
BENCH_SUITE=baseline BENCH_SAMPLES=5 cargo build --release --locked \
  --bin overhead-s3-sharp-profile --features sharp-profile
E=target/xtensa-esp32s3-none-elf/release/overhead-s3-sharp-profile
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --skip-update-check "$E" target/sharp-profile-glyph-v3.bin
python3 image_gate.py "$E" target/sharp-profile-glyph-v3.bin
# Only after a NEW approval for another flash, exact ELF and UART-port checks:
espflash flash --port <UART> --chip esp32s3 --flash-size 8mb \
  --flash-mode dio --flash-freq 40mhz --non-interactive --skip-update-check "$E"
```

### Period-cache comparison v1 (flashed; short UART capture)

`overhead-s3-sharp-cache` is a **separate opt-in image** using the same seven
accelerated historical fixture times and 500 ms tick as the visually confirmed
v3 diagnostic. It transfers only cached frames. After each transfer, it draws
an uncached reference into a separate static 12 KB framebuffer, checks all
12,000 bytes against the transferred frame, and reports both independent draw
intervals and checksums. The reference is not a second panel frame; the two
measurements are not additive costs of a normal frame. The first cached draw is
a miss and later unchanged-period draws should hit. A full offscreen reference
on each tick consumes additional time; short captures cannot establish sustained
VCOM service, 20 Hz or normal operational cadence. Host tests compare every
fixture frame across three complete loops. The user confirmed the moving replay
still looks correct on the panel with this image.

Host tests, target release build and offline gate passed for ELF SHA-256
`7b67b7a93c7cda3270130b21f794c22bee78b266f6f789d9faa0b1a8964a6b80`
(gated image SHA-256 `0c4145f010f1537249aca0d46668f65dce4e458e536790174bb7717c77f61014`).
After explicit approval and rechecking `/dev/cu.usbserial-21410`, the exact ELF
and offline gate, the image was flashed. A short UART capture booted without a
reported panic, repeated all seven accepted fixture-frame checksums, and
reported `parity=bytes_equal` for each of its full 12,000-byte comparisons.
The first cached render took ~150 ms (cache miss); subsequent cached renders
~25.9–26.1 ms, while the separate offscreen uncached redraws took ~141.2 ms.
Transfer remained ~52.4 ms, giving ~81.6–81.8 ms per warm
update/render/transfer in this fixture—not 20 Hz. The uncached reference draw
is a separate control, not part of that ~82 ms. Natural-speed scheduling,
representative catalogue, stack bound or sustained VCOM do not follow from this
short capture; the user separately confirmed visible motion on cache v1.
The board later switched to the selectively updating `sharp-lines` v1
diagnostic; v3 is preserved separately and was visually confirmed.
Any further flash requires fresh approval and port/ELF/gate checks.

From `firmware/`, with the existing environment:

```sh
BENCH_SUITE=baseline BENCH_SAMPLES=5 cargo build --release --locked \
  --bin overhead-s3-sharp-cache --features sharp-cache
E=target/xtensa-esp32s3-none-elf/release/overhead-s3-sharp-cache
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --skip-update-check "$E" target/sharp-cache-v1.bin
python3 image_gate.py "$E" target/sharp-cache-v1.bin
# Only after separate approval and fresh checks:
espflash flash --port <UART> --chip esp32s3 --flash-size 8mb \
  --flash-mode dio --flash-freq 40mhz --non-interactive --skip-update-check "$E"
```

## Sharp line-transfer feasibility (host only)

`src/sharp_transport.rs` is compiled **only in firmware host tests** under
`sharp-replay` (for example `cargo +stable test --manifest-path firmware/Cargo.toml
--lib --locked --features sharp-cache sharp_transport -- --nocapture` from the
repo root). The candidate sender in `src/sharp_lines.rs` is available only to
the separate `sharp-lines` image; the earlier flashed images send full frames.
The [LS027B7DH01 datasheet](https://cdn-learn.adafruit.com/assets/assets/000/094/215/original/LS027B7DH01_Rev_Jun_2010.pdf)
§§6-3, 6-5-1/2/3/5 and 6-6 specifies **2 MHz maximum** SCLK, up to 20 Hz SCS
frame frequency, arbitrary-address single/multiple-line updates, a display-mode
command without pixel updates, and serial M1 COM inversion with roughly equal
polarity periods. Raising the panel clock above 2 MHz is not a supported fix.
Its operating precautions (p. 4, item 3) also call for refreshing still image
*data* before two hours; VCOM-only packets do not meet that requirement. An
occasional full refresh/resync and independently scheduled VCOM service would
still be needed in a production loop; the separate diagnostic below exercises
both but does not establish a sustained service policy.

The offline candidate compares complete 50-byte pixel rows with the last
**confirmed** panel frame. A changed-row transaction consists of a one-byte
M0/M1 command, 52 bytes per addressed line (address, 50 inverted pixel bytes,
dummy), and a final dummy; with no changes, two bytes toggle M1 in display
mode. A full/unknown-state refresh uses the existing 12,482-byte transfer.
A host panel-memory model applies the candidate and existing full transactions
independently, validates unique addresses, phase and erased pixels, and checks
12,000 reconstructed framebuffer bytes on all 21 cached fixture draws (three
replay loops including resets), plus synthetic idle/erase/resync cases.

| Replay transition | Lines | Bytes | Ideal 2 MHz bit time |
| --- | ---: | ---: | ---: |
| Initial unconditionally full | 240 | 12,482 | 49.928 ms |
| +0→+50, +50→+100, +100→+150 | 34, 30, 25 | 1,770, 1,562, 1,302 | 7.080, 6.248, 5.208 ms |
| +150→+200, +200→+250, +250→+300 | 23, 18, 22 | 1,198, 938, 1,146 | 4.792, 3.752, 4.584 ms |
| +300→+0 wrap | 33 | 1,718 | 6.872 ms |

The next two loops repeat those **transition** counts; first startup is not a
wrap. This is only the seven-frame, single-ISS, fixed-view historical replay.
The 2 MHz numbers are bit-time **floors**, not measured partial-transfer latency:
SPI-call/CS overhead, line comparison, full-refresh stalls, real catalogue
changes, zoom and network work are excluded. Even an ideal full transfer nearly
fills 50 ms before ~26 ms warm render; periodic full frames require an explicit
skip/resync policy, not a 20 Hz guarantee. Physical timing/appearance/VCOM
checks are needed before any device adoption.

## Selective-line target diagnostic (flashed; short UART capture, user visual check)

`overhead-s3-sharp-lines` (`--features sharp-lines`) is a **separate opt-in**
fixture image, currently on the board, not a live clock. It keeps the same
5° real-ISS excerpt, cached period raster, 2 MHz SPI and 500 ms display step.
In every eight steps, it renders the seven +0…+300 s fixture frames (50 s
apart), then performs one display-mode, no-render VCOM-only step holding +300 s.
Every 16th step (8 s) sends a full frame, including startup and the second
loop's +0 reset. Other frames compare complete rows to the last successfully
transferred frame. A failed SPI transaction invalidates that state; the next
frame would be full (the diagnostic halts visibly on error instead of pretending
VCOM was serviced). The full step is a bounded exercise of refreshing unchanged
lines well within the datasheet's still-image precaution, **not** a production
refresh/scheduling policy. Active-high CS encloses each complete command; M1
alternates even on the explicit idle step. UART distinguishes `Full`, `Lines`
and `Idle` with row/byte counts, checksum and timed update/render/transfer; idle
and offscreen controls are not added to the frame cost.

Host tests run the real sender over 40 scheduled steps against an independent
full-frame panel-memory oracle, including idle, wrap, forced full refresh and
error/resync, plus synthetic erase and unchanged-frame behavior. This establishes
offline byte parity, not physical validation. The image was built and passed
the offline gate, then flashed after separate approval and fresh
port/ELF/gate checks. A bounded UART capture showed clean
boot, no reported panic, repeated seven-frame checksums, the scheduled idle
steps and full resyncs at ticks 0, 16, 32 and 48. Non-full transitions sent
18–34 rows (938–1,770 bytes) in ~5.4–8.8 ms; warm cached render ~25.9–26.1 ms
and update ~3.3–3.5 ms, so the observed partial-frame update/render/transfer
was ~34.7–38.3 ms. Periodic full transfer stayed ~52.4 ms and warm full
update/render/transfer ~81.9 ms. Idle display-mode transfers reported ~84–85 µs.
The fixture loop *still schedules only every 500 ms*, not 20 Hz, and this
short capture is not sustained VCOM or visual evidence. Asked whether the ISS
moves and resets without stale rows/ghosting (including at full refresh), the
user reported that the display **looks good**. This is physical visual feedback
for this diagnostic, not a 20 Hz, stack-safety or sustained-operation check.
The board now runs `sharp-lines` v1, not `sharp-cache` v1. Another flash needs
fresh approval and checks. The gated ELF SHA-256 is
`e419c8ddfe398e24a15bd6ff303e775d4d85a15d803f4b23c7623a6834ffdbe6`;
saved image SHA-256 is
`9916b416cfa68c7006f96851a8a3fd9c6b89461d8c015bd01e63f6598075ffcd`.
The existing linker RWX warning remains.

From `firmware/` after `. "$HOME/export-esp.sh"`:

```sh
BENCH_SUITE=baseline BENCH_SAMPLES=5 cargo build --release --locked \
  --bin overhead-s3-sharp-lines --features sharp-lines
E=target/xtensa-esp32s3-none-elf/release/overhead-s3-sharp-lines
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --skip-update-check "$E" target/sharp-lines-v1.bin
python3 image_gate.py "$E" target/sharp-lines-v1.bin
# Flashed after separate approval and fresh port/ELF/gate checks:
espflash flash --port <UART> --chip esp32s3 --flash-size 8mb \
  --flash-mode dio --flash-freq 40mhz --non-interactive --skip-update-check "$E"
```

## UTC/frame scheduling preflight (host only)

`src/device_timing.rs` is an opt-in, no-alloc timing policy tested under
`sharp-replay`; **it is not wired into any S3 image yet**. A trusted/validated
UTC sample must be supplied by a future SNTP integration before a frame is due;
invalid time suppresses frame sampling. The 50 ms target grid uses monotonic
uptime to sample UTC at the actual frame start, skipping missed deadlines rather
than bursting stale frames. A separate 500 ms idle-VCOM reminder alternates its
bit after successful transfers, even when there is no render. Neither helper
can maintain VCOM during a blocked loop, verify UTC quality or make the current
~196 ms render+transfer meet the 50 ms target. This offline timing policy added
no dependency and is not used by the selective-line image currently on the
board.

## Hardware and permission

Ask before installing toolchains or flashing. Flashing replaces the existing
application and writes a bootloader/partition table; it is not a read-only test.
The capture command resets the device but does not flash it.

Confirmed board: ESP32-S3-DevKitC-1, WROOM-1 module, S3 revision v0.2, 8 MB flash.
PSRAM variant/capacity is **unconfirmed and unused**. Use a data-capable micro-USB
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

Before any future approved flash, run the [offline image gate](IMAGE_GATE.md)
on the exact normal-build ELF/application image. The local DROM linker remedy
passes offline and in [two corrected catalogue-memory captures](../docs/evaluations/s3-drom-diagnostic.md#corrected-normal-build-target-verification);
the corrected kernel binary has not been boot-tested. Stress-test artifacts are
not flash candidates. The gate does not itself authorize flashing.

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

From `firmware/`, **after approval** and checking the port:

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
