# First ESP32-S3 kernel costs — 2026-10-09

## Scope and reproduction

First target-device M2 cost baseline, **not supported catalogue capacity,
worst-case latency, numeric accuracy certification, or operating policy**.
Build/flash/capture procedure and measurement contract:
[firmware/README.md](../../firmware/README.md).

[Raw final-build run](s3-costs-run-1.log): 14 workloads × 5 samples, one untimed
warm-up per workload, one invocation per sample. Every search completed; all
sample work counts match the corresponding [host baseline](host-costs.md).
This is one complete recorded run, not a confidence interval or a CI timing gate.
Earlier exploratory captures (including a timeout of the larger matrix) are not
included in the reported results. No outliers were removed.

### Recorded context

- User-confirmed ESP32-S3-DevKitC-1, metal module marked ESP32-S3-WROOM-1.
  `espflash board-info`: S3 revision v0.2, 40 MHz crystal, 8 MB flash.
  **N8R8/PSRAM capacity not confirmed**; PSRAM is not initialized or used.
- UART via CP2102N at `/dev/cu.usbserial-110`, direct laptop connection;
  115200-baud diagnostics. Dock connection failed to enumerate; cause unresolved.
  User approved toolchain installation and benchmark flashing. Display untouched.
- CPU explicitly set to 240 MHz; only the first core runs workloads. No RTOS,
  wireless stack, display, allocator, or concurrent application tasks.
- Xtensa Rust 1.97.0.0: rustc 1.97.0-nightly `8ea53bcd7` (2026-07-08), LLVM 21.1.3;
  GCC 16.2.0 (`esp-16.2.0_20260914`), espup 0.18.0, espflash 4.6.0.
  Host workspace still uses its original stable toolchain.
- Base revision `4e39ce2` plus this session's firmware changes; separate locked
  firmware workspace. `esp-hal` 1.2.2, `esp-println` 0.18.0,
  `esp-bootloader-esp-idf` 0.6.0; sgp4 2.4.0, libm 0.2.16, chrono 0.4.45.
  Default core features (no alloc/OMM/catalogue on target), f64 + libm math.
- `xtensa-esp32s3-none-elf`, release opt-level 3, debug info 2, debug assertions
  off, no LTO override; linker flag `-Tlinkall.x`. No RUSTFLAGS or HAL environment
  overrides. HAL defaults: 32 KiB instruction cache, 64 KiB data cache,
  32-byte lines, 8 ways each. Code/constant data mainly mapped from flash;
  satellite/search state on internal RAM/stack, timestamps in flash rodata.
- Flash DIO at 40 MHz, 8 MB image setting. espflash's bundled bootloader identifies
  itself as ESP-IDF v6.1-beta1-497-g14f663f003e; the application is bare-metal Rust,
  **not** an ESP-IDF/FreeRTOS application. App image: 265,488 bytes.
- Timer: `esp_hal::time::Instant`, microsecond resolution. No output inside timed
  sections. See firmware README for warm-up, UART drain, preparation, and exclusions.

SHA-256 of the measured build and raw capture (UART line endings preserved by
`.gitattributes`; ELF itself is a local build artifact, not checked in):

| Artifact | SHA-256 |
|---|---|
| Firmware ELF | `ad04bd06d9e758d273ba6c17248f2b5dae928bf4bd3c4c723e276edc62c81cb0` |
| `firmware/Cargo.lock` | `abd11c0adcad8d4b28e492a3fe6cfff5280aefe4d27502a391a1b1a74387782c` |
| `s3-costs-run-1.log` | `8718998f95d9851d53cf6b7978a0300f7d0b212cbd1e7a50119d6845ff2f943a` |

## Observed results

The same four historical fixtures and observer as the host baseline; each starts
at its **own** epoch −12h, not a common-UTC catalogue. No live data or current time
is used. Fixtures and provenance are unchanged.

### Tracking — microseconds per satellite state evaluation

Median invocation time divided by 1,441 precomputed timestamps. These are
amortized averages across the grid, **not** maxima for individual evaluations.

| Class | Propagation only | Propagation + ECEF + look angles |
|---|---:|---:|
| ISS LEO | 601.9 | 783.0 |
| Resonant HEO | 1,179.2 | 1,495.2 |
| GEO | 997.1 | 1,313.3 |
| GNSS | 830.2 | 1,011.0 |

Full geometry here does not include satellite geodetic altitude or rendering.

### 24-hour search — seconds per fixture

60s detection interval, 5s crossing tolerance, 10° threshold. Sample ranges are
only the observed five samples, not runtime bounds or acceptable miss rates.

| Class | Minimum | Median | Maximum | Evaluations | Pass records |
|---|---:|---:|---:|---:|---:|
| ISS LEO | 1.177959 | 1.177972 | 1.178069 | 1,497 | 7 |
| Resonant HEO | 2.142448 | 2.142458 | 2.142571 | 1,449 | 1 |
| GEO | 1.826613 | 1.826614 | 1.826694 | 1,441 | 0 |
| GNSS | 1.470778 | 1.470784 | 1.470896 | 1,449 | 1 |

### Four-slot mixed batch

| Operation | Median invocation | Work |
|---|---:|---|
| Full tracking | 6.877099 s | 5,764 state evaluations over 1,441 ticks |
| 24h prediction | 6.538675 s | 5,836 evaluations, 4 searches, 9 pass records |

Full tracking is **4.772 ms per four-satellite tick**, averaged over the grid.
Prediction samples span 6.538578–6.540383s. Mixed slots reference the same four
prepared fixtures; they do not model a larger working set or catalogue storage.

### Memory observations, not budgets

Target `size_of::<Satellite>()` is 512 bytes; the four timestamp grids occupy
69,168 bytes of flash rodata. Target build has no allocator. ELF section sizes:
`.data` 2,848 B, `.bss` 168 B, `.rwtext` 11,356 B, `.vectors` 1,024 B,
`.rodata` 95,220 B, `.text` 134,337 B. These are selected section sizes, not an
additive complete device memory budget. The linker's 326,360-byte `.stack` region
is reserved space, **not measured peak stack use**. No stack watermark, catalogue
allocation/aggregation, or PSRAM capacity measurement was performed.

## Implications and remaining work

These searches are seconds long on the S3, versus sub-millisecond single-fixture
host searches. Blocking prediction on a display update path would cause visible
stalls; evaluation allowances alone do not provide a nonblocking scheduler.
The measured four-slot tracking cost is below a 50ms/20Hz frame period, but this
is **not** an end-to-end display cadence or supported-size guarantee.

The S3 has single-precision hardware floating point; double precision is
[software-emulated](https://docs.espressif.com/projects/esp-idf/en/stable/esp32s3/api-guides/performance/speed.html).
Our f64/libm path therefore has a substantially different cost from the host.
This benchmark does not isolate how much each math routine contributes, and no
precision, propagation model, or physical-pass contract was changed to save time.

Before selecting production limits/cadence/scheduling:
- Extend target measurements to older positive/negative element ages, more orbit
  samples, and detection/window/tolerance sweeps. Near-epoch samples do not bound
  older resonant costs. 16/64-slot target samples remain unmeasured.
- Measure distinct-catalogue initialization/storage/aggregation, peak memory,
  and target numerical agreement against the existing reference suite.
- Design scheduling and cached-prediction refresh/invalidation with explicit
  partial/over-budget behavior and measured headroom for display/network work.
- Validate Sharp output separately; the display is not connected yet.

Do not coarsen the detection interval merely to fit these costs: the
[short-event detection limits](detection-intervals.md) remain independent of
crossing tolerance. No production interval, supported capacity, cadence,
scheduling method, or numeric allowance is selected here. M2 remains open.
