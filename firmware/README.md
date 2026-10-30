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

## Opt-in BOOT selection on the live-UTC diagnostic (flashed; event capture)

The `live-utc-boot` feature adds active-low GPIO0 BOOT sampling to the existing
`overhead-s3-live-utc-radar` display owner. A provisional 20 ms press **and**
release qualification calls shared `select_next()` only with a valid app frame;
invalid-frame presses are consumed, not queued, and a startup hold is ignored.
Sampling occurs at the display loop boundary (nominal 5 ms wait), **not** during
blocking render/SPI or executor stalls: short presses can be missed. Selection
appears on the next scheduled UTC frame, not an immediate redraw. The current
ISS diagnostic has only one marker, so this image cannot demonstrate multi-object
cycling on the panel; the host tests cover it. Knob, physical input timing and
whole-device latency remain unverified. GPIO0 is also a boot strap: holding
BOOT across reset can enter the ROM downloader.

Build and gate offline with dummy credentials, from `firmware/` after sourcing
`$HOME/export-esp.sh`; this does **not** authorize flashing or board access:

```sh
SSID=BUILD_ONLY PASSWORD=BUILD_ONLY cargo build --release --locked \
  --bin overhead-s3-live-utc-radar --features live-utc-boot
ELF=target/xtensa-esp32s3-none-elf/release/overhead-s3-live-utc-radar
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --skip-update-check "$ELF" target/live-utc-boot-build-only.bin
python3 image_gate.py "$ELF" target/live-utc-boot-build-only.bin
```

The dummy-credential build and offline exact-image gate passed locally. After
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
