# ESP32-S3 16-slot kernel costs — 2026-10-09

## Scope and evidence

First expanded-suite target capture: **16 repeated fixture slots, not 16 distinct
catalogue entries or a supported capacity**. One recorded run, two workloads ×
three samples, with one untimed warm-up each. All six samples match the
[host-generated work manifest](s3-scaling-16-expected.txt).
[Raw capture](s3-costs-run-scaling-16-1.log) retains UART bytes unchanged.
No samples were removed. This is not a worst-case bound or accuracy certification.

Harness selection, input definitions, timing exclusions, and reproduction commands
live in [firmware/README.md](../../firmware/README.md). Use
`BENCH_SUITE=scaling-16 BENCH_SAMPLES=3` and the corresponding host manifest.
The [first four-slot baseline](s3-costs.md) remains unchanged.

## Build and hardware context

- Base revision `ee2a3cb` plus this session's firmware changes. V2 harness adds
  separate age/search/scaling suites and external host-manifest validation;
  core, historical fixture bytes, dependencies, and lockfile are unchanged.
- Same confirmed S3-DevKitC-1/WROOM-1, revision v0.2, 8 MB flash; PSRAM capacity
  remains unconfirmed and unused. User explicitly approved this 16-slot flash
  and capture only. Direct UART `/dev/cu.usbserial-110`, CP2102N, 115200 baud.
- CPU 240 MHz, first core only; bare-metal, no allocator, display, wireless,
  concurrent app tasks, or PSRAM. f64/libm calculations unchanged.
- Xtensa rustc 1.97.0-nightly `8ea53bcd7` (2026-07-08), toolchain 1.97.0.0;
  GCC 16.2.0 (`esp-16.2.0_20260914`), espflash 4.6.0. Release opt-level 3,
  debug info 2, no LTO override, debug assertions off. Existing target config
  and HAL defaults unchanged from the first baseline; no extra RUSTFLAGS.
- Flash DIO/40 MHz/8 MB; same espflash-bundled ESP-IDF-format bootloader.
  Application image 654,992 bytes. The known RWX LOAD linker warning remains.
- Historical LEO/HEO/GEO/GNSS instances repeat four times; each uses its own
  epoch-relative timestamps, not common UTC. Full tracking evaluates 1,441
  ticks from epoch −12h to +12h at 60s spacing. Search uses 24h, 60s detection,
  5s crossing tolerance, 10° elevation, and the same Colorado observer.
  These remain experiment settings, not production defaults.

| Artifact | SHA-256 |
|---|---|
| Measured ELF | `d3afd346aadaeae591a72e5608c36684cf18fa79db0933cc0d580edc7879bb75` |
| `firmware/Cargo.lock` | `abd11c0adcad8d4b28e492a3fe6cfff5280aefe4d27502a391a1b1a74387782c` |
| Expected-work manifest | `ea4b1beacafd5c24d1e0d9d63471be4a703b30eec6fe666488c702fb02f46b40` |
| Raw capture | `b269bd2c5ad5ad8d2a192d207a1b8171de9e9a661306d408b809c68d33bcaac7` |

The ELF is a local artifact, not checked in. During this session it and the
original capture/manifest were retained in `/tmp/overhead-scaling-16.ubSBP9/`;
that temporary directory is not a durable archive.

## Observed timings

Seconds per invocation; each sample is one invocation, not a calibrated batch.

| Operation | Minimum | Median | Maximum | Work per invocation |
|---|---:|---:|---:|---|
| Propagation + ECEF + look angles | 27.659121 | 27.659558 | 27.671874 | 23,056 state evaluations |
| 24h pass search | 27.341320 | 27.343044 | 27.345529 | 23,344 evaluations, 16 searches, 36 pass records |

Tracking is **19.1947 ms per 16-slot tick**, dividing the median by 1,441.
This is an amortized average, not an individual-tick maximum. It excludes
satellite geodetic altitude, rendering, and the rest of a device update.

The medians are about 4.02× (tracking) and 4.18× (prediction) the old four-slot
medians. Work counts are exactly 4×. However, V2 changes the harness and code/data
layout, including additional flash grids; the old/new ratio is **not a controlled
same-build scaling comparison**. Do not attribute the difference to slot count
alone. Rerun the V2 baseline alongside larger suites for that comparison.

## Memory inventory, not peak-memory measurements

Target-reported sizes: `Satellite` 512 B; one four-fixture timestamp grid 69,168 B;
prepared workload 560 B. ELF symbols confirm **all seven age grids remain linked**,
484,176 B of flash data even though this suite uses only the zero-age grid.
This is experiment infrastructure, not catalogue storage or RAM consumption.

Selected ELF section sizes: `.data` 2,952 B, `.bss` 168 B, `.rwtext` 11,356 B,
`.vectors` 1,024 B, `.rodata` 511,228 B, `.text` 128,057 B. Do not add these into
an end-to-end device memory budget. The reserved `.stack` region is 326,256 B,
**not measured peak stack use**. No distinct-catalogue initialization,
allocation/aggregation, stack watermark, or PSRAM measurement was performed.

## Implications and next measurements

The amortized tracking cost is below the intended 50ms/20Hz frame period, but
there is no end-to-end cadence/capacity guarantee or measured application headroom.
A synchronous 27-second prediction would visibly stall a display path. Scheduling,
cached prediction refresh, and explicit partial/over-budget behavior still need
to be chosen; numeric evaluation allowances alone are not a scheduler.

All 122 workloads across the expanded suites complete/repeat on the host, but
**only scaling-16 has a new target capture**. V2 baseline, 64-slot scaling,
signed-age and search-setting sweeps still need hardware measurements; further
orbit samples, target numeric checks, and real catalogue/peak-memory costs remain
pending. Ask before another flash. No precision/model/detection-policy changes
or production defaults were selected. M2 stays open.
