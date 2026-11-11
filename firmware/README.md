# ESP32-S3 kernel benchmark

Early M2 measurement firmware plus device diagnostics. The **device
application** is the opt-in `device` build ([below](#device-application-0076));
the crate name and default binary are historical. The default kernel
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
historical fixture image (since replaced on the board by `sharp-natural-audit`
v2), not a live clock. It keeps the same 5° real-ISS excerpt, cached period
raster, 2 MHz SPI and 500 ms display step.
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

`src/device_timing.rs` is an opt-in, no-alloc timing preflight tested under
`sharp-replay` (with sender-resync coverage under `sharp-lines`). Its diagnostic
action policy is wired only into the separate `sharp-natural` historical-fixture
S3 image, not a live device loop. A trusted/validated UTC sample must be supplied
by a future SNTP integration before live frame sampling; the historical UTC
anchor in its host test is explicit fixture data, not a trusted device clock.
Invalid time suppresses frame sampling. The 50 ms target grid uses monotonic
uptime to sample UTC at actual frame start, skipping missed deadlines rather
than bursting stale frames. A separate 500 ms idle-VCOM reminder alternates its
bit after successful transfers even without renders. A monotonic 8-second
**diagnostic** full-data reminder (matching the existing replay's interval)
counts only completed full transfers, not rendered ticks or VCOM-only commands;
a failure makes the next send require full resync. With invalid UTC, a due
refresh requires resending retained, confirmed framebuffer pixels rather than
inventing a new orbital sample. The host timeline exercises an ~82 ms full
operation overrunning the 50 ms grid, a ~36 ms partial, loss of UTC, idle VCOM,
and full resync after failed partial write. Those durations are inputs based on
short target captures, **not** measured scheduling behavior. A combined
`DiagnosticSchedule` chooses a due frame first (full if refresh is due), then a
retained-pixel full refresh without UTC, then idle VCOM. It credits only
successful completed transfers; after a failed command it refuses idle service
until a full resync. If the panel is unknown and no retained pixels or valid
frame are available, it waits rather than claiming safe service. Host tests
preflight this action policy; the fixture image exercises it on the S3, without
a guarantee during failure.

No helper preempts a blocked loop, validates time quality, bounds stack or
network latency, establishes safe VCOM polarity after a partly failed command,
or proves a still-image refresh deadline or 20 Hz. The 8-second interval is not
a production policy. A live loop still needs decisions on blocked/failure
service, retained-frame availability and full-frame overrun handling. This
preflight added no dependency; the board now runs the separate natural-speed
command-audit v2 historical-fixture image.

[0062's host-only time-loss preflight](../docs/decisions/0062-no-trusted-time-display-preflight.md)
adds `RadarApp::invalidate_frame` and an opt-in `TimeLossPreflight` wrapper,
without changing that fixture image. Polling the cold-boot/time-loss action
clears even pre-populated app tracking before requesting a full draw/send of the
existing `NO FRAME` state. A retry clears it again if it was repopulated while
the panel replacement remained unconfirmed. Only successful completion confirms
the replacement; thereafter
idle VCOM and diagnostic full-data reminders continue independently of UTC,
using the newly retained warning pixels. A failed command receives no credit;
recovery samples only a new, externally trusted UTC anchor after checking the
shared supported-year/leap-second contract. A rejected sync, poll-time monotonic
regression (including relative to the last completed transfer) or UTC/deadline
arithmetic overflow invalidates previous tracking and requests `NO FRAME`
before old tracking pixels can be refreshed; only a new sync can resume frames.
This representation check cannot verify SNTP provenance, clock age/drift or
element freshness. The wrapper does not guarantee panel service through a
blocked loop or failed SPI transaction. Host coverage (including real rendered
framebuffer and selective sender) runs with:

```sh
cargo +stable test --manifest-path firmware/Cargo.toml --lib --locked --features sharp-natural-audit
```

A separate host-only [`UtcTrustPreflight`](src/utc_trust.rs) now composes
that lifecycle with injected *screened* observations. It checks time/uncertainty
and explicit test-only drift/holdover limits on every poll (including between
frames), rejects invalid calendar values, clears old samples on resync and
prevents a pending fresh frame from resending previous tracking pixels. A
failed/no-response event cannot renew trust; a matched unsynchronized response
revokes it, as does a pending-leap reply; holdover also ends 100 ms before any
UTC month end (possible leap). [`sntp_screen`](src/sntp_screen.rs) is a pure
reply screen producing those events under
[0064](../docs/decisions/0064-prototype-sntp-screen-and-leap-boundary.md); no
socket or wireless stack. [The UTC proposal](../docs/utc-trust-loss-proposal.md)
records source, capture-bound and drift reasoning. Host tests run under
the same feature command above; neither these test parameters nor a public
struct named “screened” establish actual SNTP quality.

The 0062 lifecycle preflight itself did not build or flash a device image; the
separate opt-in live-UTC candidate below was built and flashed later.

### Live-UTC ISS composition preflight (host only)

`src/live_utc_preflight.rs` composes the pure SNTP reply screen, 0065's
`UtcTrustPreflight::prototype()`, checked-in ISS *elements*, wide cached
renderer and selective Sharp sender in host tests. It tests cold-boot `NO FRAME`,
a pending network request with independent idle-COM/retained-refresh actions,
first accepted sync, bounded holdover/expiry, fresh resync after a UTC jump,
unmatched/too-uncertain/KoD replies, pending leap revocation, and failed full
`NO FRAME` send/retry. Run:

```sh
cargo +stable test --manifest-path firmware/Cargo.toml --lib --locked \
  --features sharp-natural-audit live_utc_preflight
```

The checked-in elements are not a live fetch or a time source. These tests
inject packets at the peer-validated boundary; synthetic bracketing captures
are not measured radio capture evidence. The network-pending timeline proves
only that *polled* display actions remain available, not target COM service.
The separate opt-in image below owns SPI in an independent task; its service
under Wi-Fi contention and physical COM behavior are untested.

## Natural-speed historical diagnostic (flashed; short UART capture)

`overhead-s3-sharp-natural` (`--features sharp-natural`) is a separate opt-in
S3 image using the checked single-ISS fixture, cached period raster and
selective-line sender at 2 MHz. It repeats a 300-second **historical** excerpt
at natural elapsed speed, sampling each due frame at actual monotonic start
rather than advancing fixture time by a fixed amount per draw. It runs the
host-preflight action policy above: a 50 ms target grid with skipped overruns,
500 ms idle VCOM reminder, and 8-second diagnostic full-data reminder based on
completed transfers. A full frame can overrun the grid; it does not attempt to
catch up. Only occasional frame/full/skip lines are printed to avoid turning
every frame into a UART workload. The fixture anchor is never SNTP or live UTC.

This is not an adopted production loop. The release ELF and application image
passed the **offline** image gate, then after a separate go-ahead the port,
ELF/hash and image gate were rechecked and the image was flashed on the S3.
An 18-second bounded UART monitor captured clean boot and sample logs through
frame 320 (~16.1 historical seconds), with no reported panic. The initial
full frame took 204,741 µs (2,384 update, 150,436 render, 51,921 transfer),
then frame 2 sampled at +206,205 µs and reported **three skipped deadlines**.
Among the 18 logged frames (most frames are deliberately not printed), 16
unchanged-pixel sends used display mode; sampled steady totals were ~30.2–30.3
ms including ~25.8 ms render and ~1.05 ms idle transfer. One logged changed
frame sent six rows in 2,402 µs (~31.6 ms total). A retained-pixel full
refresh was reported at uptime 8,282,399 µs; this image does **not** log its
transfer duration. These are selective logs, not a measurement of every frame
or a verified 20 Hz output cadence. Asked whether the panel looked stable,
with gradual ISS movement and without stale rows or visible flicker, the user
answered **yes**. That is a separate physical visual check, not sustained
VCOM, stack, network or controls validation. On SPI error the diagnostic halts
visibly rather than claiming panel service;
blocked work cannot preemptively service COM. The raw 18-second UART log is
ignored local `target/sharp-natural-v1-short.log` (SHA-256
`62c5d396953e11fecee533d1e3c8fb9a2b72360d0ecc5a6ee4efd5ce71efe74b`).

A second **UART-only** monitor of the same ELF (no rebuild or flash) ran for
85 seconds after a monitor reset. The last sampled log is frame 1660 at fixture
+83.1 seconds. Ten retained-pixel full refreshes were reported at uptime
8.282–80.748 seconds, spaced ~8.052 seconds apart; the only logged skipped
frame deadlines were the same three immediately after the initial full frame.
Of 85 printed frames, 81 reported idle sends, three changed-line sends, and
one initial full send. Sampled warm idle totals were 30.169–30.301 ms. No panic
was reported. The ignored raw log is `target/sharp-natural-v1-extended.log`
(SHA-256 `8de83f74ae8618d39569ed61d2d1f7dc9c973eebc7f5d4a96e3a3082f990262f`).
This is a bounded **diagnostic** run, not a sustained panel-service or live-UTC
validation: only every twentieth frame and exceptional events were printed,
full-refresh duration and COM polarity were not logged, fixture wrap at 300
seconds was not reached, and physical appearance was not rechecked over the
longer interval. In particular, sparse nominal grid samples do not prove every
frame completed inside 50 ms or establish production 20 Hz operation.

A third **UART-only** capture of the unchanged, hash-checked ELF ran for 315
seconds (monitor reset; no rebuild or flash). It reached frame 6240 at uptime
~312.3 seconds. The printed historical offset went from +299.3 seconds on
frame 5980 to +0.3 seconds on frame 6000: the 300-second **fixture wrap**
was crossed without a reported panic. Thirty-eight retained full-refresh
completions were logged, with completion intervals of 8.051716–8.081107
seconds. Unlike the 85-second capture, this run reported **four additional
missed frame deadlines**, each immediately after a retained full refresh (at
~88.8, 96.85, 193.50 and 290.15 seconds), in addition to three after the
initial full frame. That is expected contention, not evidence that every
refresh fits between frames. Among 318 printed frames (every twentieth plus
exceptions), 311 were unchanged-pixel sends; this does not account for every
transfer or prove VCOM polarity, physical appearance through the wrap, or
sustained 20 Hz. Ignored raw log: `target/sharp-natural-v1-wrap.log`, SHA-256
`f961c4cdef0b5554ac3cb8e16eb6dcb630bb1b6ac8600c2ab9640111c7e7664b`.
No renewed visual check was requested for this interval.

### Natural-speed command audit v2 (flashed; bounded UART capture)

`--features sharp-natural-audit` adds opt-in telemetry to the same fixture
binary, without changing the production crates or the v1 source path. Every
completed command is counted and checked for alternating **issued** VCOM bits;
its maximum start-to-start gap, maximum gap between last completion and next
start, and maximum full-data transfer duration are accumulated. Every frame
contributes an end-to-end occupancy, over-50-ms count and skipped-grid count.
A compact cumulative audit is printed with the existing sampled frame logs,
and retained full refreshes report their transfer duration. These are
**command-side** observations, not a physical M1/panel-edge measurement;
failed commands halt rather than being credited. Serial output itself affects
workload. No network, live UTC, controls, clock-invalid recovery on device,
stack bound, visual check or sustained-display guarantee is implied.

Host tests: `cargo +stable test --manifest-path firmware/Cargo.toml --lib
--locked --features sharp-natural-audit` (32 passed). From `firmware/`, v2 was
built with `cargo build --release --locked --bin overhead-s3-sharp-natural
--features sharp-natural-audit`. The exact ELF was copied to
`target/sharp-natural-audit-v2.elf` before gating:

```sh
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --skip-update-check \
  target/sharp-natural-audit-v2.elf target/sharp-natural-audit-v2.bin
python3 image_gate.py target/sharp-natural-audit-v2.elf \
  target/sharp-natural-audit-v2.bin
```

The offline gate passed on the 450,992-byte application image. ELF SHA-256:
`d87028ff7d63715e0fa5ba7ed091c8da3b980a4dc10afc35b04f628fbf3b7c0c`;
image SHA-256:
`a1dd2050d0d2f72014ecbc88aa9837b29f0a1b2620fcd1221b172bec92996a44`.
Both are ignored local artifacts. After a separate go-ahead, the current port,
exact ELF/hash and image gate were rechecked, and v2 replaced v1 on the S3.
An approximately 115-second bounded UART monitor then booted v2 and logged
through frame 2260 at historical fixture +113.15 seconds, without a reported
panic. The final cumulative audit reports 2,260 completed frames, **one**
over-50-ms frame (the initial 204,940 µs full frame), four missed frame-grid
deadlines (three after that startup frame, one after a retained full refresh),
2,275 completed commands including the clear and 14 retained full refreshes,
and 15 full data transfers. Retained full transfers measured 51,732–51,738 µs;
the cumulative maximum full-data transfer was 51,914 µs (startup). The
largest completed-command start-to-start gap was 153,379 µs and the largest
prior-completion-to-next-start gap was 153,143 µs, **both dominated by startup
rendering**. The audit rejected no duplicate issued VCOM phase, but does not
measure actual panel COM edges. A retained refresh can miss a frame deadline
even though that frame's own update/render/send occupancy is under 50 ms.
The ignored raw log is `target/sharp-natural-audit-v2-110s.log`, SHA-256
`31582ceed40224a4705431b45913461782b351bf1eb360761b15bd5589ccf6e5`.
Asked separately about the v2 image, the user confirmed stable, gradual ISS
motion without stale rows or visible flicker. This is one bounded fixture
capture and a separate visual check, not endurance, per-edge VCOM or production
20 Hz validation; serial auditing changes the workload.
The exact v1 ELF remains saved locally as `target/sharp-natural-v1.elf`.

### Original v1 image reproduction

The scheduler tests and historical wrap test can be run without an S3 using
`cargo +stable test --manifest-path firmware/Cargo.toml --lib --locked --features sharp-natural`.

From `firmware/`, using the already installed toolchain, the exact checked
artifact was built with `cargo build --release --locked --bin
overhead-s3-sharp-natural --features sharp-natural`. A copy of that original
ELF was retained before building the separately instrumented v2 variant;
**the ordinary release ELF path now holds v2, not the original v1 image**.
For the original image:

```sh
ELF=target/sharp-natural-v1.elf
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --skip-update-check "$ELF" target/sharp-natural-v1.bin
python3 image_gate.py "$ELF" target/sharp-natural-v1.bin
```

The offline gate passed (447,776-byte application image). ELF SHA-256:
`153252a719d573ee35b800f1fe4252b28da1bc615ed34d94c52a8fd4b24ae1ab`;
image SHA-256:
`89fdb097d1752a18869e3ea12c1443851f7a4b16110abb8776e3a8ed6baaaa2a`.
The `.bin` and ELF are ignored local artifacts. A rebuild changes their
identity and requires a fresh gate. The existing linker RWX warning remains.

## S3 clock-drift capture (flashed; 3-hour UART run)

`overhead-s3-clock-drift` (`--features clock-drift`) prints
`DRIFT seq=N uptime_us=…` (`esp_hal::time::Instant`, 240 MHz CPU config) once
per nominal second and holds the Sharp panel white with serial VCOM alternating
~1 Hz. No network, radar or new dependency. `drift_capture.py` (stdlib, tests in
`test_drift_capture.py`) stamps each UART line with the host wall clock and fits
the rate, optionally correcting host stamps by interpolated `sntp` offsets.

```sh
BENCH_SUITE=baseline BENCH_SAMPLES=5 cargo build --release --locked \
  --bin overhead-s3-clock-drift --features clock-drift
# save-image + image_gate.py as above; flash with approval, then:
python3 drift_capture.py capture --port <UART> --elf "$E" --output target/drift.log --duration 10800
# in parallel, every 300 s: (date -u +%FT%TZ; sntp time.nist.gov; sntp time.cloudflare.com) >> target/offsets.log
python3 drift_capture.py fit target/drift.log --offsets target/offsets.log
```

2026-10-26, `/dev/cu.usbserial-21410`, ELF SHA-256
`745efe125e05d16cc43ae1876bee79928d83286fec0dbabf01814bebbc978d99`, image
`384c2fdcddb38ef9c94f42c5719329222981e0429456f26acb9b0ad9a34b50c7` (gate passed,
105,520 bytes). 10,798 consecutive samples over ~3.0 h, no reset or failure;
indoor room temperature, USB-powered, unmeasured. Result, S3 rate relative to
UTC via the corrected host clock: **+1.2 ppm** (least squares over 3 h); per-hour
thirds 0.8, 1.5 and 2.4 ppm. The *host* clock was the limiting reference: its
`sntp` offset wandered between +0.5 and +40.6 ms (several ppm over an hour), and
lower-envelope estimates scattered 2–8 ppm. Read this as "the S3 timer is within
a few ppm of UTC on this board, at this temperature, over three hours"; it is
not a temperature, voltage, aging or multi-board bound. Raw logs are ignored
local files (`target/clock-drift-v1-3h*.log`). The board was then restored to
the gated natural-audit v2 image (hashes unchanged; boot line confirmed).
[0065](../docs/decisions/0065-prototype-drift-bound-and-holdover.md) uses this
evidence for the prototype `D = 20 ppm`, `H = 3,600 s`
(`UtcTrustPreflight::prototype()`).

## Live SNTP source evidence (flashed; 1-hour UART run)

`overhead-s3-live-sntp` (`--features live-sntp`) joins Wi-Fi (WPA/WPA2), queries
`time.cloudflare.com` every 64 s (NIST fallback) under
[0066](../docs/decisions/0066-prototype-live-sntp-stack-and-source.md), screens each
reply with `sntp_screen` using bracketing uptime captures and 0065's 20 ppm, and
logs `SNTP …` and `SNTP_CHECK …` lines. The check compares each accepted
observation with UTC predicted from the previous one by monotonic time; it must
fall within both uncertainties plus drift. The panel is held white with VCOM
alternating ~1 Hz. No trust gate, radar or persistence on target.

```sh
# firmware/.wifi.env (ignored): SSID=… and PASSWORD=… lines
export SSID="$(sed -n 's/^SSID=//p' .wifi.env)" PASSWORD="$(sed -n 's/^PASSWORD=//p' .wifi.env)"
BENCH_SUITE=baseline BENCH_SAMPLES=5 cargo build --release --locked \
  --bin overhead-s3-live-sntp --features live-sntp
# save-image + image_gate.py as above; flash with approval; capture with
# espflash monitor, then:
python3 drift_capture.py sntp target/live-sntp.log
```

The ELF embeds the Wi-Fi credentials: keep it local and unshared. On
2026-10-26 (`/dev/cu.usbserial-10`, same MAC as before; 550,080-byte image,
gate passed) a ~62-minute run on a home 2.4 GHz network gave 59 consecutive
Cloudflare observations, none rejected, timed out or failed over:

| Metric | Result |
|---|---|
| RTT min / median / max | 12.9 / 26.0 / 85.8 ms |
| Initial uncertainty `U0` min / median / max | 21.3 / 27.7 / 57.7 ms |
| Consistency checks within bound | 58 / 58; worst \|diff\| 36.5 ms = 45% of its bound |
| S3 rate vs UTC (fit over 3,717 s) | −0.4 ppm (± a few ppm at this noise) |

With 0065's `D = 20 ppm`, a median `U0` leaves ~72 ms, about 3,600 s of
holdover. This is one network, one hour and one board; not a guarantee for
other networks, a WPA3-only access point, a congested link or the handheld.
Raw log and summary: ignored `target/live-sntp-v1-run.*`. The board was then
restored to the gated natural-audit v2 image (boot line confirmed).

## Historical multi-marker BOOT cycle diagnostic (flashed; short UART capture)

`overhead-s3-sharp-boot-cycle` (`--features sharp-boot-cycle`) is a **separate**
fixed-time #4694 display experiment, not a live-UTC image or a current-sky
claim. It loads the [pinned 16-object M2 pool](../tools/fixtures/m2-pool/README.md)
without fetching anything, fixes UTC at 2026-10-10T00:00:00Z and observer at
39.007° N, 104.883° W, 2.187 km, with a 100° wide view. A host test checks
that two distinct in-view IDs cycle and change the actual 1-bit render pixels.
The active-low BOOT/GPIO0 input uses the provisional 20 ms completed-press
filter; one press triggers one shared `select_next()` and redraw. Full panel
frames also repeat every 500 ms for VCOM. A 5 ms loop poll cannot see presses
hidden by blocking render/SPI, and neither input timing nor COM edges have
been measured for this candidate. No Wi-Fi, SNTP, knob or device scheduler is
included. A BOOT hold across reset can invoke the ROM downloader instead.

Offline build/gate from `firmware/` after sourcing `$HOME/export-esp.sh`:

```sh
cargo build --release --locked --bin overhead-s3-sharp-boot-cycle \
  --features sharp-boot-cycle
ELF=target/xtensa-esp32s3-none-elf/release/overhead-s3-sharp-boot-cycle
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --skip-update-check "$ELF" target/sharp-boot-cycle-build-only.bin
python3 image_gate.py "$ELF" target/sharp-boot-cycle-build-only.bin
```

Host tests, release build and exact-image offline gate passed; the existing
linker RWX warning remains. After separate approval, the MAC-confirmed board
was flashed with this exact image and a bounded 35-second reset-and-UART
capture logged `visible=4 selected=Some(19548)` at ready, then two completed
press selections `Some(21639)` and `Some(22314)`. This is physical event and
selection-state evidence for one historical scene. The user separately
reported that the selected label/marker **changed with each of the two
presses** on the Sharp panel. That visual report supports this bounded BOOT
cycle, **not** exhaustive cycling, missed-short-press bounds,
production cadence or live UTC. Raw UART is an ignored local artifact. The
board now runs this historical diagnostic, not the live-UTC image. Further
board access/reset/flash requires separate authorization.

2026-11-01 UTC (devbox flash check, moved here from HANDOFF): the board is on
`/dev/ttyUSB0` (CP2102N, MAC confirmed). A devbox-built image of this
diagnostic (ELF SHA-256
`efc0ed40fa053e8243fdaada9c2a8aea449491bef3f13abfac9c95cf408222b6`, image
`8551a20800a080451405b57ee49fd9cad45a1aaec6cb0f9f79f24728860fd0d8`; not
byte-identical to the Mac build above) passed the offline gate, flashed, and a
15 s UART capture showed a clean boot to
`BOOT_CYCLE_READY … visible=4 selected=Some(19548)`. BOOT presses were not
exercised on this build. Flashing no longer needs per-flash approval under
[0069](../docs/decisions/0069-flash-without-per-flash-approval.md).

## Interrupt-sampled BOOT capture diagnostic (flashed; user tap session)

`overhead-s3-sharp-boot-capture` (`--features sharp-boot-capture`) implements
[0070](../docs/decisions/0070-boot-input-capture-independent-of-render.md) on
the same fixed historical scene as the polling image above, which stays
unchanged for comparison. A 5 ms TIMG0 periodic interrupt samples GPIO0 into
`button_input::PressLatch` (the same `CompletedPress` filter, provisional
20 ms). The display loop drains completed presses at its boundary and applies
[0068](../docs/decisions/0068-boot-selection-gesture.md) via `apply_presses`:
each drained press advances the selection once on a valid frame. The interrupt
shares only the timer, pin and latch under a critical section and never
touches the app, renderer or panel. It needs `esp-hal/unstable` (the timer
module) and a direct `critical-section` dependency, which was already locked
transitively. TIMG0 is free because this image has no esp-rtos or Wi-Fi. The
interrupt, shared state and timer setup live in `src/boot_capture.rs`, which the
[live-UTC BOOT image](#opt-in-boot-selection-on-the-live-utc-diagnostic-interrupt-capture-user-tap-session)
also uses.

UART lines: `BOOT_CAPTURE_READY …`, one `BOOT_CAPTURE drained=N applied=N
selected=…` per non-empty drain, and every 10 s `BOOT_CAPTURE_STATS samples=N
max_gap_us=G` (expect about 2000 samples; a UART `println` holds a critical
section and can delay one sample).

```sh
cargo build --release --locked --bin overhead-s3-sharp-boot-capture \
  --features sharp-boot-capture
ELF=target/xtensa-esp32s3-none-elf/release/overhead-s3-sharp-boot-capture
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --skip-update-check "$ELF" target/sharp-boot-capture.bin
python3 image_gate.py "$ELF" target/sharp-boot-capture.bin
```

2026-11-01 UTC (devbox build, not byte-identical to any Mac build): host
tests, release build and the offline gate passed (ELF SHA-256
`f23982a6f97831361976b7c680ff920dc27fc44acd7a732516c69f798ad3eff7`, image
`ca1bb5f1ce1312db01a5af95767012b10ee4cbb16b4863de70ba7a7aaeaa0bf6`). Flashed to
the MAC-confirmed board on `/dev/ttyUSB0`. A 25 s reset capture with no presses
logged `visible=4 selected=Some(19548)`, then `samples=2000 max_gap_us=5045` and
`samples=2010 max_gap_us=5001`. Each window spanned about twenty 500 ms VCOM
transfers, so the interrupt kept sampling during blocking SPI. Same day, a 90 s
reset capture while the user pressed deliberately, quick-tapped, double-tapped
and tapped right after each visible change (plus some extra presses) logged
23 `drained=1 applied=1` lines. The selection advanced in order around the 4
visible IDs with no skip or repeat. Sampling stayed at 2000–2026 per 10 s with
`max_gap_us` ≤ 5050 during the press-triggered redraws. The user reported
**every press moved the selection**, and the 23 events were reasonable for the
presses made. The press total was not counted exactly and the log has no
timestamps, so this rules out obvious misses, not an exact one-to-one match.
No drain held more than one press, so multi-press drains remain host-tested
only. Press durations, latency and live-UTC behavior are not measured.

After the capture code moved into `src/boot_capture.rs` (same logic, now
shared), a devbox rebuild passed the gate (ELF SHA-256
`d9d0c1146f74177ad0a753f376e4681661335228ddc81e94d5e5ced6bc08fdab`, image
`506c31ad234e3cf2fc311ce1d6676367bfdad3b465fc9ee1582dabb5f82dc8c9`) and was
flashed to the MAC-confirmed board on `/dev/ttyUSB0`. A 26 s idle reset capture
logged `visible=4 selected=Some(19548)`, `samples=2000 max_gap_us=5027` and
`samples=2010 max_gap_us=5001`, matching the pre-refactor image. The board
runs this build. No presses were made on it.

## Opt-in BOOT selection on the live-UTC diagnostic (interrupt capture; user tap session)

The `live-utc-boot` feature adds active-low GPIO0 BOOT input to the existing
`overhead-s3-live-utc-radar` display owner. Since 2026-11-01 it uses the
[0070](../docs/decisions/0070-boot-input-capture-independent-of-render.md)
capture in `src/boot_capture.rs`: a 5 ms **TIMG1** periodic interrupt samples
GPIO0 into `PressLatch`, because esp-rtos owns TIMG0 timer0 and neither
esp-rtos nor esp-radio uses TIMG1. The display task drains completed presses
at its loop boundary and applies
[0068](../docs/decisions/0068-boot-selection-gesture.md) via `apply_presses`.
Each press calls shared `select_next()` once, only with a valid app frame.
Presses without a valid frame are consumed, not queued, and a startup hold is
ignored. The provisional 20 ms qualification is unchanged. Selection appears on
the next scheduled UTC frame, not an immediate redraw. The current
ISS diagnostic has only one marker, so this image cannot demonstrate multi-object
cycling on the panel; the host tests cover it. Knob, physical input timing and
whole-device latency remain unverified. GPIO0 is also a boot strap: holding
BOOT across reset can enter the ROM downloader.

UART lines: `BOOT_CAPTURE_READY live UTC sample_ms=5 timer=TIMG1` at startup,
one `BOOT_CAPTURE drained=N applied=N uptime_us=… valid=… selected=…` per
non-empty drain, and every 10 s `BOOT_CAPTURE_STATS samples=N max_gap_us=G`.
Expect about 2000 samples per window. `max_gap_us` shows whether Wi-Fi,
esp-rtos or UART critical sections delay the interrupt.

Build and gate offline with dummy credentials, from `firmware/` after sourcing
`$HOME/export-esp.sh`:

```sh
SSID=BUILD_ONLY PASSWORD=BUILD_ONLY cargo build --release --locked \
  --bin overhead-s3-live-utc-radar --features live-utc-boot
ELF=target/xtensa-esp32s3-none-elf/release/overhead-s3-live-utc-radar
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --skip-update-check "$ELF" target/live-utc-boot-build-only.bin
python3 image_gate.py "$ELF" target/live-utc-boot-build-only.bin
```

2026-11-01 UTC (devbox, interrupt capture): host suites, clippy (no new
warnings), the dummy-credential release build and the offline gate passed
(ELF SHA-256 `9f2cb931d84ea7b31597990950b3b94d96862f57fb29befc8a280d08bace8775`,
image `bca8733179d2dec8279a3aecaf1aa072a240361c0b9e664b24c730b18f5adab1`).
The plain `live-utc-radar` build is unchanged in source and still builds.

Same day, the user created the ignored `firmware/.wifi.env`. A
credential-bearing rebuild passed the gate and was flashed to the MAC-confirmed
board on `/dev/ttyUSB0`. Its hashes are not recorded because the image embeds
credentials; it and the raw UART logs are ignored local artifacts. In three
reset captures (45 s idle, 90 s with no presses, 90 s tap session), time from
Cloudflare was accepted at ~11.8–11.9 s and again at ~76 s. The first 10 s
window, while Wi-Fi associated, always showed one gap of 33.8–34.7 ms.
After that, sampling stayed at 2000–2008 per 10 s with `max_gap_us` 5.2–6.5 ms,
rising once to 9.2 ms around a later SNTP exchange. In the tap session the user
pressed deliberately, quick-tapped, double-tapped and tapped right after
presses. UART logged **44** `drained=1 applied=1 valid=true` lines from
~25.3 s to ~60.2 s, including bursts of up to six taps ~150 ms apart (closest
134 ms). The user confirmed 44 matched their presses. `selected=None`
throughout: the single ISS was not in view, so this is press-capture evidence,
not visible cycling. No drain held more than one press. Press durations,
latency and the startup association gap's effect on a real press are not
measured.

Earlier polling version: the BOOT input was sampled once per display loop,
not during blocking render/SPI or executor stalls. Its dummy-credential build
and offline exact-image gate passed locally. After
separate physical approval, a credential-bearing rebuild passed the same gate,
and was flashed to the MAC-confirmed DevKitC-1 on `/dev/cu.usbserial-21410`.
A bounded 28-second UART capture logged the diagnostic boot, `NO FRAME`
completion at uptime ~0.50 s and an accepted UTC source at ~11.78 s. In a
separately approved reset-and-36-second capture, the user operated BOOT and
UART logged **two** completed presses at uptime ~14.40 s and ~21.41 s, each
`valid=true selected=None` (no in-view ISS to cycle at that UTC). This confirms
sampled BOOT events on the physical board in this run; the user confirmed two
presses. The user's tentative recollection of `NO FRAME` on the panel was not
a focused observation at either press, and does not override UART's valid UTC
state at those moments. This does **not** establish in-view cycling,
missed-short-press bounds, bounce rejection under stress, or a visual change.
An attempted no-reset capture connected through the flash stub and yielded no
application output; it is not negative button evidence. The existing linker
RWX warning remains. The credential-bearing image and raw UART log are
ignored local artifacts; do not share them. The BOOT/GPIO0 startup role is
recorded in [Espressif's boot-mode guide](https://docs.espressif.com/projects/esptool/en/latest/esp32s3/advanced-topics/boot-mode-selection.html).

## Live-UTC ISS radar candidate (flashed; bounded UART capture)

`overhead-s3-live-utc-radar` (`--features live-utc-radar`) is a separate opt-in
image combining the existing `live-sntp` Wi-Fi/UDP adapter with
`UtcTrustPreflight::prototype()`, the checked-in single-ISS elements and the
natural-speed cached-render/selective-line diagnostic policy. A dedicated
Embassy display task alone owns app, gate, framebuffer, Sharp SPI and VCOM;
the network side checks exact peer, nonce and bracketing capture, then sends
screened outcomes through a bounded `embassy-sync` channel. This dependency
is already locked transitively at 0.8.0; it is direct only for this feature to
avoid shared mutable display/time state and permit Wi-Fi awaits without waiting
for a Sharp command. An observation is evaluated at **processing** uptime;
channel backlog cannot extend its holdover. A matched KoD stops all subsequent
queries until reboot (conservative diagnostic handling); unmatched packets,
failed DNS/send/timeout and disconnect cannot renew time. Cloudflare then NIST
is attempted only when the first yields no reply.

Use *dummy* credentials for a build-only/offline-gate check (the ELF embeds the
actual `SSID`/`PASSWORD` supplied to it). From `firmware/`, with the existing
Espressif exports, **do not flash again without a separate go-ahead**:

```sh
SSID=BUILD_ONLY PASSWORD=BUILD_ONLY cargo build --release --locked \
  --bin overhead-s3-live-utc-radar --features live-utc-radar
ELF=target/xtensa-esp32s3-none-elf/release/overhead-s3-live-utc-radar
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --skip-update-check "$ELF" target/live-utc-radar-build-only.bin
python3 image_gate.py "$ELF" target/live-utc-radar-build-only.bin
```

On 2026-10-26 the dummy-credential release build and exact-image offline gate
passed (909,984 B image; ELF SHA-256 `ed14efb2261638b773bb067f492d5dc94d026f43a73049a1b915a86258687c1e`;
image SHA-256 `2026771b8a2b29e40094d06d59d587c52b5e67a84c5ba6ff18dfbfb41af5ba13`).
That hash is **not** a usable Wi-Fi image; any rebuild must be re-gated.
The existing linker RWX warning persists. That dummy image was not flashed.

**First local-credential run (2026-10-26):** after separate user approval, a
real-credential ELF was built locally, paired with its 910,048-byte image and
passed the exact-ELF offline gate again immediately before flashing. ELF
SHA-256 `6c7f7fb07ee9f8b71fd4fca07f9ca4223d052a06d90d5892e5432a8062168b77`;
image SHA-256 `65d92fc3b72de2f5605e0b48ea5c08f8ba30b44378a55e05fd1998f2cd8ba7fa`.
Board-info confirmed the expected S3 MAC `80:45:6b:2a:45:08` on
`/dev/cu.usbserial-10`. Flash completed, replacing the historical audit v2.
The credential-bearing ELF/image and raw UART log remain **ignored local
artifacts**; do not share them.

A bounded ~90-second monitor reset the board and captured one boot and the
`NO FRAME` full-send completion at uptime 0.498 s, followed by Wi-Fi connection
and two Cloudflare `Observation` outcomes accepted by the trust gate (at
uptime ~11.75 s and ~75.83 s). Nine retained-full completion lines were logged;
the last sampled report reached frame 1,520 at UTC Unix
`1793055175026776` µs, with three logged skipped-grid counts after refreshes,
and no failure marker. Of 80 printed frame summaries,
79 reported an unchanged-frame idle command and one a full transfer; summaries
are sampled/event-driven, not every frame. The raw ignored log is
`target/live-utc-radar-first-90s.log` (SHA-256
`9a6cd973f69b15a3129e0bf36d25fee949dcf21a7e5dda2bd2f1d2a8c72eab1e`).
The user subsequently reported seeing an **empty-sky notice** on the physical
Sharp panel. That confirms a legible post-sync empty view, not an ISS marker,
visible motion, cold-boot `NO FRAME` appearance, absence of flicker/stale rows,
or physical COM edges. The UART capture remains **command-side** evidence, not
verified 20 Hz throughput or a complete frame-timing trace. The board currently
runs this candidate; no restoration flash has been made.

The 8-second full refresh, 50 ms grid, 500 ms idle reminder and 5 ms owner poll
are **diagnostic**, not selected production timing or a guaranteed physical
VCOM bound. The reported unchanged frames and historical element age cannot
establish visible ISS motion in this 90-second window. On failed SPI the image
panics with panel state unknown, so it cannot promise subsequent COM service.
The async owner does not preempt blocking renderer/SPI, Wi-Fi driver, or a
stalled executor. Neither image gate
nor host tests measure stack/heap peaks, scheduler contention, power-on safety,
or sustained panel behavior. The pinned ISS elements are historical and will
age further; a live UTC frame is **not** an accurate current ISS position claim.
No HTTPS, persistence, data-age policy, controls or provisioning is present.

**Headless empty-view diagnosis (same checked-in elements and first-capture UTC):**
`src/live_utc_preflight.rs::first_live_utc_capture_empty_view_diagnostic`
reuses the candidate's `iss-ground-20` fixture setup (observer fixed at the
ISS subpoint at element epoch, 5° wide view), then samples the captured
`1793055175026776` µs UTC and ±45 seconds. At the captured instant
(2026-10-26 22:52:55.026776 UTC), the elements from
2026-10-04 12:43:41.833056 UTC are **22 days 10:09:13.193720 old**.
The fixed observer is approximately 0.0000174° N, 85.1803573° W; the
single ISS sample propagates successfully but has about **−48.385° observer
elevation**. All three tested instants have one accepted entry, zero tracking
failures, zero above-horizon objects, zero visible markers and zero indicators.
The same observer has a visible marker at the fixture epoch (control).
With a valid trusted frame and no ingestion warning, the wide renderer's
[0048/0061 notice rules](../docs/decisions/0048-refined-empty-view-notices.md)
therefore select exactly **`NONE ABOVE HORIZON`** (not `NONE IN VIEW`,
`NO MARKERS`, `TRACK ERROR` or `NO FRAME`). This explains why an empty view
is expected near the sampled time; the user reported only an empty-sky notice,
not its text, and no framebuffer from that run was captured. This is a
propagation/classification diagnostic, **not** a claim that 22-day-old
predictions describe the real ISS or that the physical panel displayed those
exact pixels. Reproduce on the host with:

```sh
cargo +stable test --manifest-path firmware/Cargo.toml --lib --locked \
  --features sharp-natural-audit first_live_utc_capture_empty_view_diagnostic -- --nocapture
```

**Fresh-element host-only visibility proof (2026-10-26):** CelesTrak's
[ISS GP JSON](https://celestrak.org/NORAD/elements/gp.php?CATNR=25544&FORMAT=JSON)
was retrieved as a *temporary local input* (manifest fetch metadata
`2026-10-26T23:02:53Z`). The one-record JSON's SHA-256 is
`418a12a93d28234192d8b4fc8876937f381e87012d6fb747decfab6a1e3dde27`;
NORAD 25544's element epoch is `2026-10-26T05:01:03.508032Z`.
The initial input, manifest and headless PNGs live in ignored
`target/fresh-iss-host-proof/`. A separately dated copy of the OMM is now
[checked in for the opt-in diagnostic](fixtures/README.md); the live endpoint
can change, so a later fetch does **not** reproduce these exact bytes.
No on-device fetching or runtime cache was added.

Using **the same fixed observer and 5° wide view as the candidate**, an
explicit host pass search from `2026-10-26T23:03:00Z` found a modelled
above-horizon interval on 2026-10-27 approximately
`02:34:52..02:43:08Z` (30 s detection grid, 5 s crossing tolerance,
0° threshold; brackets, not real-world timing guarantees). At
`2026-10-27T02:39:00Z` (elements age 21:37:56.491968), the shared
`overhead-track` pipeline computes **+8.338° elevation**, and the headless
`overhead-sim --wide --fov-deg 5 --snapshot` reports
`catalogue=1 sky=1 visible=1 failed=0 indicators=0`. Its 400×240
black/white PNG shows the selected `ISS` diamond and strip;
PNGs at ±30 s show
its screen position changing. At the earlier board-capture sample with these
fresh elements, the host still reports `sky=0 visible=0`—freshness alone
cannot promise a visible marker at an arbitrary instant. The old fixture
also happens to yield a visible marker at 02:39, so this is a **positive
host visibility control, not an attribution of a changed position to
freshness**. The host snapshots are neither physical-panel evidence nor a
live-UTC/device-cadence test. The 02:39 sample was a *future model time* when
this host proof was run, not a captured device observation.

Reproduce from the checked-in dated OMM (host only, repo root; outputs ignored):

```sh
mkdir -p target/fresh-iss-host-proof
printf '%s\n' '{"groups":[{"name":"iss","path":"../../firmware/fixtures/live-utc-iss-20261026.json","category":"station"}]}' \
  > target/fresh-iss-host-proof/pinned-catalogue.json
cargo run -p overhead-tools --bin overhead-catalogue -- target/fresh-iss-host-proof/pinned-catalogue.json
cargo run -p overhead-tools --bin overhead-track -- \
  firmware/fixtures/live-utc-iss-20261026.json 2026-10-27T02:39:00Z \
  0.000017408882706672767 -85.18035731481179 0
cargo run -p overhead-sim -- target/fresh-iss-host-proof/pinned-catalogue.json \
  2026-10-27T02:39:00Z 0.000017408882706672767 -85.18035731481179 0 \
  --wide --fov-deg 5 --snapshot target/fresh-iss-host-proof/visible-5deg.png
```

This host proof selects no production data-age rule, fetch/persistence path,
network credentials policy or scheduler.

## Pinned-OMM live-UTC marker diagnostic (flashed; no visual observation)

After authorization to stage an image (not a flash), the [dated OMM and
provenance](fixtures/README.md) were checked in separately from the old
fixture. `overhead-s3-live-utc-fresh-iss` (`--features live-utc-fresh-iss`)
copies the existing display/network diagnostic runtime without editing the
original image's source, but builds its own app from this pinned source at the
**same fixed observer** and 5° wide view. The original candidate's source
still loads its historical elements; neither image fetches OMM or implements
an age policy. [Trial scope and completed gates](../docs/live-utc-visible-marker-proposal.md)
remain separate from 0067.

Host `pinned_fresh_iss_diagnostic_keeps_site_and_renders_marker_motion` checks
that the new input preserves the original observer/view, that cold boot clears
the epoch frame to `NO FRAME`, that screened time at the first capture's UTC
remains empty, and that 02:38:30/02:39:00/02:39:30 UTC on 2026-10-27 each
produces one selected, in-view ISS with changing marker positions and changed
selective-sender pixels. These are historical *test timestamps* after this
window passes. Re-evaluate any new capture window against the pinned OMM and
confirm it is still timely **before** seeking physical approval.

For a **build-only** check with dummy Wi-Fi credentials, from `firmware/`
with the existing Espressif exports (do **not** flash):

```sh
SSID=BUILD_ONLY PASSWORD=BUILD_ONLY cargo build --release --locked \
  --bin overhead-s3-live-utc-fresh-iss --features live-utc-fresh-iss
ELF=target/xtensa-esp32s3-none-elf/release/overhead-s3-live-utc-fresh-iss
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --skip-update-check "$ELF" target/live-utc-fresh-iss-build-only.bin
python3 image_gate.py "$ELF" target/live-utc-fresh-iss-build-only.bin
```

On 2026-10-26 this target release build and exact offline image gate passed
for **dummy** credentials: image 908,672 B, ELF SHA-256
`0fd6f2a91f4d78657db4c847a8e447dd5d5a6f5f5dfb65bc949bd598f9720a8a`,
image SHA-256
`632094ea92508e39530771aa7fdb056accc2b7c57a6f7ef53601fea78b73c2ce`.
The linker RWX warning persists. These hashes do **not** identify a usable
Wi-Fi image. No board-info, flash, UART capture or visual review was done.
A subsequent **local credential-bearing build-only** preparation on
2026-10-27 at ~01:45 UTC produced a distinct, ignored
`target/live-utc-fresh-iss-real-20261027.elf` paired with
`target/live-utc-fresh-iss-real-20261027.bin`. The exact offline gate passed:
908,672 B / 8,323,072 B partition (10.92%), ELF SHA-256
`a1fb3c4870d853b1cdc94e8b3c23d52d11879e924fdd087300961c80c5497b66`,
image SHA-256
`db12cba295d58e0299bfb54998370dfeea75f9480d613f1f898f599a3442f882`.
Credentials and binary remain local; **no board access, MAC check, flash,
monitor or physical review** was performed by this preparation. Any rebuild
invalidates these identities and requires a new exact-image gate.

At 2026-10-27 01:44 UTC an explicit **host-only** repeat of the pass search
using the checked-in OMM (not the mutable endpoint) and the original fixed
observer, 30 s detection, 5 s crossing tolerance, 0° threshold, 2 h horizon
and 3,000 evaluation allowances completed (247 evaluations). It bracketed
above-horizon start `02:34:52.500..02:34:56.250Z` and end
`02:43:03.750..02:43:07.500Z`. Headless wide 5° snapshots every 30 s from
`02:37:00` through `02:41:00Z` each reported
`catalogue=1 sky=1 visible=1 failed=0 indicators=0`. This is the **2026-10-27
UTC** predicted window, not a guarantee of physical visibility or a later-day
window; the ~21.5-hour-old elements and fixed historical observer still limit
real-world claims. The ignored host manifest and PNGs are in
`target/fresh-iss-host-proof/`. No device or visual observations follow.

**Single authorized physical trial (2026-10-27):** after explicit user
approval of this *exact* credential-bearing ELF and the 02:37–02:41 UTC
window, the expected UART bridge `/dev/cu.usbserial-10`, S3 revision v0.2,
8 MB flash and MAC `80:45:6b:2a:45:08` were confirmed. The source JSON, ELF,
image and exact-image offline gate were rechecked immediately before the
flash, with the same SHA-256 hashes above. The one flash completed at about
`02:33:18Z`; a single local monitor ran from that boot until `02:43:30Z`,
then stopped. An initial timed host wait stopped slightly early **without a
flash**; the immediate retry passed every guard before this one flash. No
restoration flash was done. The raw credential-adjacent UART log remains
**ignored and local** at `target/live-utc-fresh-iss-20261027-window.log`
(SHA-256 `8bc22d0a6c7b9e8badc952bae40570db9a2640105fe76c421f463dde91d8406d`,
62,898 bytes); do not share it or the ELF/image.

The bounded log has one diagnostic banner, one completed cold-boot `NO FRAME`
send, Wi-Fi connection, **10 screened SNTP observations accepted** by the
gate, 58 reported retained-full completions, and 645 *sampled/event-logged*
frame summaries from `02:33:32.951787Z` to `02:43:29.605559Z`: 615 idle,
17 full, 13 changed-line summaries; zero logged panics or `LIVE_UTC failure`
lines. From `02:37:00` to `02:41:01Z` the sampled summaries include seven
changed-line and seven full sends; changed-line sends total 61 rows. These
are **not all frames**, physical COM edges, a 20 Hz guarantee, marker/selection
telemetry, or proof of display contents. The live-UTC source code logs only
transfer kinds, not observed marker coordinates; the host-predicted marker
cannot be inferred from these UART lines. **The user was not watching the
panel during the predicted window. No visual marker, motion or notice was
observed or accepted.** The nearly hour-long wait between authorization and
capture was not coordinated with the user; do not repeat that unattended
scheduling approach. The device remains on the diagnostic image; this short
capture neither tests unattended service nor authorizes another flash.

Do not infer visual success from UART or reuse the expired window as permission
for a later capture. First agree a time when the user is ready to look; recheck
an upcoming window against appropriately reviewed input, then seek fresh
approval before any further board access or flash.

## HTTPS element fetch, slice 1 (host-only; no image)

[0071](../docs/decisions/0071-https-element-fetch-first-slice.md) slice 1 adds
the opt-in `element-fetch` feature and `src/element_fetch.rs`: the fixed
CelesTrak STATIONS `HTTP/1.0` GET, a bounded response parser (2 KiB head,
16 KiB body, one required `Content-Length`; any non-200 status, chunked coding,
truncation or overrun is a typed failure) and ingestion through the
`overhead-core` catalogue. It has no TLS, socket or target binary yet, and no
default or existing image changes. Tests use the pinned
[STATIONS fixture](fixtures/README.md#celestrak-stations-20261101json):

```sh
cargo +stable test --manifest-path firmware/Cargo.toml --lib --locked --features element-fetch
```

## HTTPS element fetch, slice 2 (build-only TLS image; not flashed)

The opt-in `live-fetch` feature builds `overhead-s3-live-fetch`
(`src/live_fetch_main.rs`): Wi-Fi, one screened SNTP observation within
0065's 100 ms budget, then exactly one HTTPS GET of STATIONS through
[`mbedtls-rs`](https://github.com/esp-rs/mbedtls-rs) 0.3.0, parsed and ingested
by `element_fetch`. It prints `FETCH_*` lines (TLS result and version, TCP and
handshake times, transfer bytes and time, HTTP status, catalogue summary,
heap high-water from `esp-alloc`'s `internal-heap-stats`) and never retries.
The Sharp panel is only held white with VCOM. Trust anchors are the two roots
in `certs/celestrak-roots.pem` (Sectigo Public Server Authentication Root R46,
USERTrust RSA Certification Authority, copied from devbox's system CA store);
on 2026-11-01 each alone verified CelesTrak's served chain and hostname with
`openssl s_client` (handshake only, no HTTP request).

**Certificate dates are not checked.** `mbedtls-rs-sys` uses its prebuilt
MbedTLS library only when the active features and hooks equal its default
`tls` profile. The `hook-wall-clock` feature (needed for `notBefore`/`notAfter`
checks) changes the hooks and forces an on-the-fly C build that needs `clang`,
`cmake` and `ninja`, none of which are installed. That would be a toolchain
change, so this build keeps the default profile: the chain, signatures and hostname
are verified, validity dates are not. Hardware crypto (`EspAccel`, the crate's
`esp32s3` feature) is also not enabled, so crypto runs in software.

Build and gate with dummy credentials, from `firmware/` after sourcing
`$HOME/export-esp.sh`:

```sh
SSID=BUILD_ONLY PASSWORD=BUILD_ONLY cargo build --release --locked \
  --bin overhead-s3-live-fetch --features live-fetch
ELF=target/xtensa-esp32s3-none-elf/release/overhead-s3-live-fetch
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --skip-update-check "$ELF" target/live-fetch-build-only.bin
python3 image_gate.py "$ELF" target/live-fetch-build-only.bin
```

2026-11-01 UTC (devbox): it built against the existing esp-hal `=1.2.2`,
esp-radio `=1.0.0-beta.1` and esp-rtos `=0.4.0` pins with no pin or toolchain
change, and no existing locked version changed. The lockfile adds
`mbedtls-rs`/`mbedtls-rs-sys` and host-only build-script crates (bindgen, cmake
and similar, unused on the prebuilt path). No libc shim was needed: string
functions resolve to ROM, `calloc`/`free` to esp-alloc, and `snprintf`/`printf`
to an existing prebuilt object. Only the existing RWX linker warning appeared;
clippy reported no new warnings. Image **1,185,328 B** (a same-day `live-sntp`
build is 550,640 B), 14.2% of the app partition; static internal RAM 274,308 B
including the 160 KiB heap and an 18 KiB response buffer. ELF SHA-256
`dd8e3a61032167a0890dd58b7803025216cae69b0fd6471e90f09cd39c9038cf`, image
`4b479ff4c213030af346e92112e4e5147ca6ad5a5c9c2157fa0dedc478e8592b`; the offline
gate passed. The image was **not flashed or run**; handshake time, memory peak
and the CelesTrak response are unmeasured.

## HTTPS element fetch on the board (flashed; one-boot UART capture)

Same `live-fetch` image, built with the user's ignored `.wifi.env` (see the
live-SNTP section for the export), gated as above, then flashed and captured
in **one** command so the board boots once and makes one request (a separate
`espflash monitor` resets it again and costs a second request):

```sh
timeout 150 espflash flash --port <UART> --chip esp32s3 --flash-size 8mb \
  --flash-mode dio --flash-freq 40mhz --non-interactive --skip-update-check \
  --monitor "$ELF" > target/live-fetch-run1.log 2>&1
```

Certificate validity dates are still unchecked, as accepted for this
diagnostic in [0073](../docs/decisions/0073-diagnostic-fetch-without-certificate-dates.md).
The ELF and image embed credentials: their hashes are not recorded and they,
like the raw log (which contains the LAN address), stay local.

2026-11-01 15:19 UTC, devbox, `/dev/ttyUSB0` (MAC `80:45:6b:2a:45:08`,
1,185,328 B image, gate passed), home 2.4 GHz network, one boot:

| Stage | Result |
|---|---|
| Trusted UTC | one screened SNTP observation, `U0` 27.8 ms; heap peak 54,460 B |
| TCP connect / TLS handshake | 40 ms / **1,761 ms**, TLS 1.2, chain and hostname verified |
| Transfer | 8,691 B in 476 ms; HTTP 200, 8,446 B body |
| Catalogue | 20 records, 20 accepted, 0 errors/invalid/conflicts; epochs 2026-10-31T09:58:37Z – 2026-11-01T06:40:35Z; ingest 79 ms |
| Whole fetch | 2,445 ms; heap peak **113,704 B** of the 163,840 B heap |

CelesTrak ended the connection with a TCP reset after the full response, not
a TLS `close_notify` (`FETCH_READ_END error=Io(ConnectionReset)`); the exact
`Content-Length` check is what establishes the body was complete. Crypto runs
in software (no `EspAccel`). The heap peak exceeds the live-UTC radar image's
96 KiB heap, so combining fetch with the display needs a memory decision. This
is one boot on one network: not repeated-run, failure-path, long-run or
date-check evidence.

## Live-UTC radar on fetched STATIONS (flashed; UART captures)

The opt-in `live-fetch-radar` feature builds `overhead-s3-live-fetch-radar`
(`src/live_fetch_radar_main.rs`; since 0076 a thin entry to the shared
`src/live_radar.rs` with every telemetry line, banner `…_V3`) under
[0074](../docs/decisions/0074-live-utc-radar-on-fetched-elements.md): the
live-UTC wide radar with BOOT interrupt capture (0068/0070), starting with **no
elements** (`NO FRAME`, then `NO DATA`). After the first SNTP observation
within 100 ms it makes **one** STATIONS request, checks every served
certificate's validity dates against that UTC (`tls_dates`) before sending the
GET, and hands the catalogue to the display owner. Fixed site
`LIVE_UTC_ISS_OBSERVER`, 60° vertical FOV, 160 KiB heap. No retry,
persistence or refresh; each boot is one CelesTrak request. (Superseded for
this image by 0075's refresh and home site, [below](#refresh-staleness-and-home-site-0075).) It prints the
`FETCH_*` lines of `live-fetch` plus `FETCH_CERT_DATES`, `STACK_WATERMARK`
(CPU0 stack written depth through the fetch), `DISPLAY_FETCH_GAP` (longest
display-loop gap while the fetch ran), `LIVE_FETCH loaded` and periodic
`BOOT_CAPTURE_STATS` with catalogue/sky/view counts and the selection.

Build, gate, flash and capture exactly as for `live-fetch` (one command, so
one boot), with `--bin overhead-s3-live-fetch-radar --features live-fetch-radar`.
A dummy-credential build was 1,333,392 B (16.0% of the app partition), `.bss`
250,208 B including the heap; only the existing RWX warning; the gate passed.
That leaves **~21.7 KiB of CPU0 stack** (the fetch-only image has ~65 KiB).
The first board run overflowed it (stack guard panic after the first SNTP
observation, before DNS, so no request) because `StaticCell::init([0; N])`
built the 18 KiB response buffer on the stack; the buffers are now
`ConstStaticCell` statics.

2026-11-01 15:44 UTC, devbox, `/dev/ttyUSB0` (MAC `80:45:6b:2a:45:08`, gate
passed), one boot:

| Stage | Result |
|---|---|
| Trusted UTC | first Cloudflare observation accepted, `U0` 29.1 ms, ~11.9 s after boot |
| TLS | 1.2, chain and hostname verified; TCP 46 ms, handshake **803 ms** (240 MHz CPU) |
| Certificate dates | 3 served certificates, all valid at 2026-11-01T15:44:24Z; 5,435 B recorded; leaf `notAfter` 2026-12-11T23:59:59Z |
| Transfer / catalogue | 8,691 B, HTTP 200, 20/20 accepted, 0 errors; epochs 2026-10-31T09:58:37Z – 2026-11-01T06:40:35Z |
| Whole fetch | 1,387 ms; heap peak **111,116 B** of 163,840 B; stack written **6,880 B** of 21,696 B |
| Display owner | longest loop gap during the fetch **452 ms** (8 frames skipped once), then 6–7 ms per 10 s window; BOOT sampling max gap 12.3 ms in the fetch window, ~5.2 ms after |
| Radar | `catalogue=20 valid=true`; no object above the horizon at that time (`NONE ABOVE HORIZON`) |

Steady-state frames at 60° take ~47–49 ms, so the 50 ms grid skips about one
deadline per logged frame (cadence is deferred under 0060). The 452 ms stall
is CPU-bound TLS work in the same executor: it stayed under the 500 ms idle
VCOM interval in this run but is not a bound. Before trusted UTC the loop also
stalls ~0.6 s at startup (coast preparation and Wi-Fi start), as in the other
live-UTC images.

**User-observed pass, 2026-11-01.** A host predictor (pinned STATIONS fixture,
same site and FOV) put the CSS complex (48274, 53239, 54216, 69049, 69180) in
view 16:57:30–17:07:30 UTC. A fresh boot at 16:50 UTC (CelesTrak request 3 of
that 2-hour window) fetched again: TLS 1.2, handshake **2,171 ms**, certificate
dates OK (3 certificates), HTTP 200, 20/20 accepted, whole fetch 5,431 ms, heap
peak 111,068 B, stack written 6,880 B, longest display-loop gap during the fetch
**450 ms**. The 10 s stats reported `sky=5 view=5` from about 16:57 to 17:07 UTC
(within their 10 s resolution), `sky=0` otherwise. The user saw the markers
appear and move across the view and then `NONE ABOVE HORIZON` return. Ten BOOT
presses (two full cycles) each changed the selected ID
(`48274 → 53239 → 54216 → 69049 → 69180 → 48274`, all `applied=1 valid=true`;
BOOT max sampling gap ~5.2–5.9 ms after the fetch). The user saw nothing odd.
This is the first **live-UTC visible cycling** and the first fetched elements on
the panel. It is not location accuracy (fixed historical site), repeated-run,
failure-path (certificate, HTTP or parse), long-run or physical COM-edge
evidence. Handshake time varied 0.8–2.2 s across these runs. Raw logs and
credential-bearing images stay local.

### Refresh, staleness and home site (0075)

[0075](../docs/decisions/0075-fetched-element-refresh-and-staleness.md) changes
this image: it refreshes STATIONS 8 h after each success (build-time
`REFRESH_MINUTES`, at least 120, for diagnostics), retries only failures before
the GET is sent (5/15/30/60 min backoff; everything after the GET and every
verification failure waits one interval), keeps tracking old elements with an
`OLD DATA` notice after 24 h and unloads them once the newest epoch is 7 days
old. The observer is the home site from the ignored `firmware/.site.env`
(`SITE_LAT_DEG=`, `SITE_LON_DEG=`, optional `SITE_ALT_M=`; decimal degrees,
metres), never printed. Banner `OVERHEAD_S3_LIVE_FETCH_RADAR_V2`; new lines
`FETCH_START`, `FETCH_SCHEDULE attempt= outcome= next_in_s=`, `LIVE_FETCH
loaded load= fetched_at= selected_before=` and `LIVE_FETCH stale=`.

```sh
export SSID="$(sed -n 's/^SSID=//p' .wifi.env)" PASSWORD="$(sed -n 's/^PASSWORD=//p' .wifi.env)"
export SITE_LAT_DEG="$(sed -n 's/^SITE_LAT_DEG=//p' .site.env)" \
  SITE_LON_DEG="$(sed -n 's/^SITE_LON_DEG=//p' .site.env)" \
  SITE_ALT_M="$(sed -n 's/^SITE_ALT_M=//p' .site.env)"
REFRESH_MINUTES=120 cargo build --release --locked \
  --bin overhead-s3-live-fetch-radar --features live-fetch-radar
# save-image + image_gate.py as for live-fetch; flash and capture in one command
```

Dummy build (`SITE_LAT_DEG=0 SITE_LON_DEG=0`, dummy credentials): image
1,338,608 B (16.08%), only the RWX warning, clippy no new warnings, gate passed.
Site- and credential-bearing images and raw logs stay local.

**On-board evidence, 2026-11-01/02** (devbox, `/dev/ttyUSB0`, MAC
`80:45:6b:2a:45:08`, gate passed each build, home site, `REFRESH_MINUTES=120`):

| Run | Result |
|---|---|
| 1 (21:57–02:36 UTC, 4h40m) | boot fetch + 2 refreshes, all HTTP 200 and 20/20; the second brought newer epochs. Each refresh ran on the first in-budget SNTP poll after its due time (~60 s late). Heap high-water 111,116 → 136,808 → 139,452 B |
| User pass (01:00–01:10 UTC, after refresh 1) | CSS complex (48274, 53239, 54216, 69049, 69180) in view on the refreshed catalogue; BOOT cycled it twice, all `applied=1 valid=true`; user: everything looked good. The five share one element set, so their glyphs are identical by design |
| 2 (02:40–13:10 UTC, 10h30m; adds `heap_now`) | boot fetch + 5 refreshes, all HTTP 200 and 20/20; selection kept across loads (`selected_before=selected=49271`). Live heap in idle 10 s windows **73,612 B** after fetches 1–3 and **73,800 B** after 4–6 (one +188 B step within interval 4, not per refresh): no leak. High-water plateaued at 136,808–136,996 B (~27 KB free) |
| Per fetch (both runs) | handshake 0.9–1.6 s, once 7.9 s; whole fetch 1.8–16.5 s; display-loop gap 449–464 ms per fetch; no panic, no Wi-Fi disconnect |

BOOT sampling across run 2: 3,153 of ~3,760 windows under 6 ms, 530 at 6–10 ms,
77 at 10–16 ms spread over all sky states and away from fetches, and one 41 ms
gap at boot during Wi-Fi association (already known). All are under the 20 ms
debounce, but longer than the short tap sessions showed.

Not shown on the board: the `OLD DATA` notice (24 h), expiry (7 days) and the
failure classes (no failure occurred); those are host-tested
(`element_refresh`, app and render tests). Afterwards the board was left on the
default 8-hour build (`refresh_s=28800`); its boot fetch loaded 20 records.

To read a running board's UART **without resetting it** (an `espflash monitor`
resets it and costs a request): `stty -F /dev/ttyUSB0 115200 raw -echo -hupcl
&& timeout 25 cat /dev/ttyUSB0`. `espflash monitor --no-reset --before
no-reset-no-sync` hung at "Connecting" on devbox.

### Fetch failure paths on the board (`live-fetch-faults`, 0075)

The opt-in `live-fetch-faults` feature builds the same binary with a
build-time fault plan (`src/fault_plan.rs`, host-tested). Each fault happens at
its real stage, so 0075's classification and schedule run unchanged:

| `FAULT_PLAN` word | What the attempt does | 0075 class |
|---|---|---|
| `dns` | resolves `overhead-fault.invalid` (never resolves) | Transport (backoff) |
| `tcp` | connects to TEST-NET-1 `192.0.2.1:443` (20 s timeout) | Transport (backoff) |
| `handshake` | real handshake with CelesTrak, stream reports EOF after the server's first byte | Handshake (backoff once) |
| `verify` | verifies against ISRG Root X1 (`certs/fault-wrong-root.pem`, not in CelesTrak's chain); no GET | Rejected (interval) |
| `ok` | ordinary fetch | Success |

Attempts past the plan are ordinary. `FAULT_STALE_MIN` replaces the 24 h
`OLD DATA` threshold; `FAULT_EXPIRE_AFTER_LOAD_MIN` sets expiry to the first
load's newest-epoch age plus that delay (a fixed threshold would fire at an
unpredictable time, as the newest STATIONS epoch can be hours old when
fetched); `FAULT_WIFI_DROP_AT_MIN`/`_FOR_MIN` disconnect Wi-Fi once. The image
prints `FAULT_CONFIG`, `FAULT_INJECT`, `FAULT_RESULT ... match=`,
`FAULT_FRESHNESS`, `FAULT_WIFI` and `LIVE_FETCH expired`. Without the feature
none of the plan, overrides or wrong root is built in. Never a device image.

```sh
REFRESH_MINUTES=120 FAULT_PLAN=tcp,ok,dns,handshake,verify,ok FAULT_STALE_MIN=90 \
  FAULT_EXPIRE_AFTER_LOAD_MIN=210 FAULT_WIFI_DROP_AT_MIN=60 FAULT_WIFI_DROP_FOR_MIN=10 \
  cargo build --release --locked --bin overhead-s3-live-fetch-radar --features live-fetch-faults
# plus the Wi-Fi/site exports above; save-image, gate, flash+capture in one command
```

Dummy build 1,350,768 B (16.23%), RWX warning only, clippy no new warnings,
gate passed; every flashed build was gated.

**Run 1, 2026-11-02 18:15–23:05 UTC** (devbox, `/dev/ttyUSB0`, MAC
`80:45:6b:2a:45:08`, home site, plan as in the command above). Two CelesTrak
GETs in total, both HTTP 200:

| Attempt (UTC) | Fault → result | Next |
|---|---|---|
| 1 boot | tcp → `stage=tcp_connect`, Transport; radar stays `NO DATA` | 300 s |
| 2 18:21:41 | ok → 20/20 loaded; newest epoch 10:25:21Z (~7.9 h old) | 7,200 s |
| 3 20:22 | dns → `stage=dns`, Transport (streak restarted after the success) | 300 s |
| 4 20:27 | handshake → `Io(ConnectionReset)`, `verify=0x0`, Handshake | 900 s |
| 5 20:43 | verify → MbedTLS `-0x2700`, `verify=0x8` (not trusted), Rejected, no GET | 7,200 s |
| 6 22:44:54 | ok → HTTP 200, 20/20, but newest epoch **still 10:25:21Z** | 7,200 s |

Every `FAULT_RESULT` matched. Through all failures the radar kept the 18:21
catalogue (`catalogue=20`). `OLD DATA` rose at 19:51:41Z (fetch + 90 min).
Expiry unloaded the catalogue at 21:51:41Z (load + 210 min; live heap 73,612
→ 51,456 B). Attempt 6's elements were as old as the expired set (~12.3 h, over
the 41,180 s threshold), so they were unloaded on load: correct epoch-based
behaviour, but no reload or `OLD DATA` clear could be shown in this run.
Wi-Fi dropped at 3,600 s for 600 s and reconnected without a reboot; frames
continued on holdover and the next SNTP observation was accepted 11 s after
reconnecting (SNTP made no attempts while the link was down). Other: 242 of
244 observations accepted, 1 NoRenewal, one NIST fallback; one `NO FRAME`
(boot); no panic; heap high-water 135,320 of 163,840 B; CPU0 stack 6,112 of
21,144 B; display-loop gap per attempt 22/452/19/112/243/451 ms; BOOT
sampling gap 33 ms at boot (known), ≤14 ms afterwards.

The run-1 image logged a spurious `stale=false frame_utc=None` / `stale=true`
pair on every accepted SNTP renewal: the renewal invalidates the app frame
until the next render (0062), and the stale check ran in between. The panel
always renders after `update`, so the notice was not affected; the log check
now only uses frames with UTC.

**Run 2, 2026-11-03 00:42–03:02 UTC** (same board and port, image at
`be9f080` with the log fix; `FAULT_PLAN=dns FAULT_STALE_MIN=30`, no expiry
override, no Wi-Fi drop). The boot attempt failed at `stage=dns` (Transport,
300 s) so the first GET came ≥2 h after run 1's last. Attempt 2 loaded 20/20
at 00:49:07Z (newest epoch 2026-11-02T17:17:18Z); `stale=true` at 01:19:07Z
(fetch + 30 min); attempt 3 at 02:50:01Z loaded 20/20 with a newer epoch
(20:37:14Z), kept the selection (`67683`) and logged `stale=false` on the
same frame. So a successful load clears `OLD DATA`. Two GETs, both HTTP 200;
117 of 119 SNTP observations accepted, 2 NoRenewal; no `NO FRAME` after boot;
no panic; steady live heap 73,612 B after each load (95,768 B briefly while
the old and new catalogues coexist), heap high-water 139,696 of 163,840 B;
display-loop gap per attempt 16/456/458 ms.

A reload after expiry was still not shown (the run-1 reload happened, but
its elements were already expired); it goes through the same load path, and
the unload/expiry rule is host-tested in `overhead-app`.

Not shown on the board: post-GET failures (HTTP status, truncation, parse;
each would cost a download and an interval) and the repeated-handshake rule;
both stay host-tested in `element_refresh`.

## Device application (0076)

`--features device` builds `overhead-s3-device` (`src/device_main.rs`), the
Overhead device app under [0076](../docs/decisions/0076-device-app-build.md):
the live-UTC wide radar at the home site on fetched elements (the `visual`
group since [0077](../docs/decisions/0077-m2-operating-budget-visual-group.md),
banner `OVERHEAD_DEVICE_V2`; STATIONS before), refreshed and aged under 0075,
with BOOT cycling. It runs the same runtime
(`src/live_radar.rs`) as the `live-fetch-radar` diagnostic and its fault build;
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

## BLE coexistence spike

Historical (removed 2026-11-08; Git keeps `src/ble_spike.rs` and the
`ble-spike` feature): a throwaway trouble-host GATT peripheral on the
`live-fetch-radar` image for the [giftable-device proposal](../docs/giftable-device-proposal.md),
with no pairing. It established that BLE adds ~33 KB of statics (BT
controller code +20.6 KB in shared D/IRAM, `.data` +5 KB, `.bss` +6 KB), so
the 160 KiB static heap no longer links and the bootloader-reclaimed `dram2`
region (72 KiB, `#[ram(reclaimed)]`) became a second internal heap on every
runtime image. Its service `6f760001-…` may still be in an iPhone's cache
(see the [setup service](#ble-setup-service-0087)).

**On-board evidence, 2026-11-08** (devbox, `/dev/ttyUSB0`, MAC
`80:45:6b:2a:45:08`, ELF sha256 `4f6708af…`, gate passed, home site,
`REFRESH_MINUTES=120`; phone: iPhone with nRF Connect):

| Check | Result |
|---|---|
| Controller start | 35,440 B of internal heap (free 160,880 → 125,440 of 200 KiB) |
| Boot fetch with BLE advertising | HTTP 200, 156/156, 15.0 s; internal free min 116,112 B through boot, Wi-Fi, SNTP and TLS |
| Phone | connected, wrote 10 B and 9 B, read the `received 10 bytes` reply; two earlier attempts ended `Connection Timeout` before one held |
| While connected | internal free flat at 124,632 B; `FRAME_WINDOW` 1–2 skipped of ~200, as without BLE; user: radar kept updating, knob felt normal |

Not measured: pairing/encryption, long connections, BLE during a refresh
with a phone connected.

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
  press while tracking shows the info page for 30 s.
- **Leaves** by a software reset 3 s after `Saved`, or after 10 min with no
  client when settings exist.
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
`SETUP ble_disconnected reason=`, `INFO_PAGE shown=`.

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

The board was left on release build 14 (`beta`, settings from the iOS app).
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
data and, on an empty store, waits on `NO FRAME` for setup (`SETTINGS
unconfigured`). `ota-dev` asks only the plain-HTTP LAN host `OTA_DEV_HOST`
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
holder copies `N.bin` to the Mac, checks its sha256, runs `overhead-release
sign KEY beta N VERSION N.bin OUT` and `verify`, and copies `OUT` back. Push
`OUT/ota/…` to the orphan `pages` branch (signed files, a README and GitHub's
`CNAME` for `overhead.wesklein.com`: pull first and never force-push, or the
custom domain drops and devices fall back to raw GitHub).
The device checks after each element refresh (8 h) and installs once the knob
and BOOT have been idle 2 min (`UPDATING n%`).

**Lines.** `OTA boot build= version= slot= state= outcome=Normal|Verify|RolledBack
channel= failed_build=`; `OTA_HEALTH met=… missing=0x..`, `valid`, `failed …;
resetting`; `OTA_CHECK source= build= offer=Install|Current|Failed|…` or
`failed=`/`refused=`; `OTA_INSTALL start|done|failed=`; `OTA_TLS`,
`OTA_CERT_DATES`, `OTA_HTTP`; `SETTINGS seeded|empty|unconfigured`.

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
a >1 release cycle offline, the `stable` channel.

## Release and recovery checklist

_Draft (2026-11-11, soak work item): steps 1–6 will be checked on release 15,
recovery on the board after the soak._ Commands for each step are in
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
   (`grep -c -F -- "$value" N.bin` prints `0` for each; never echo the values).
3. **Hand over:** `sha256sum N.bin`; the key holder copies it to the Mac and
   checks the hash.
4. **Sign on the Mac:** `sign KEY beta N 0.1.N N.bin OUT`, and for `stable`
   `sign KEY stable N 0.1.N N.bin OUT` into the same `OUT`; `verify` each
   manifest. Copy `OUT/ota/` back to devbox.
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

**USB recovery flash** (a unit that won't boot or update; keeps `settings`
and `elements`, so Wi-Fi, site and channel survive): with the board on the
UART port, flash the newest published image's ELF exactly as the "USB flash"
command above (bootloader, partition table, `--erase-parts otadata`, app in
`ota_0`). After a factory reset, or for a board whose store is empty, the
image's `OTA_CHANNEL` seed decides the channel: build with
`OTA_CHANNEL=beta` for the user's board. Then confirm `OTA boot … state=`,
first frame and `OTA_CHECK … offer=Current`.

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
