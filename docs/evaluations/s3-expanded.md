# ESP32-S3 expanded kernel costs — 2026-10-10 UTC

## Scope and evidence

Completed the remaining prepared V2 target suites: baseline, 64-slot scaling,
four signed-age sweeps, and four search-setting sweeps. **Ten captures, 120
workloads × three samples**, one untimed warm-up per workload. Every capture
passes strict validation against its host-generated expected-work manifest;
no failures, retries, or excluded samples. Together with the earlier
[16-slot run](s3-scaling-16.md), all 122 prepared workloads now have target evidence.
This is still four historical satellites, not broader orbital sampling or a
supported catalogue size.

- [Full timing/work summary](s3-expanded-summary.csv): min/median/max integer
  microseconds for every workload, settings, and evaluation/search/pass counts.
- [Artifact index](s3-expanded-artifacts.json): per-suite ELF, raw-capture, and
  expected-manifest SHA-256 hashes, filenames, and selected ELF section sizes.
- Raw logs: `s3-costs-run-v2-<suite>-1.log`; host manifests:
  `s3-v2-<suite>-expected.txt`, alongside this report. UART bytes are preserved.
- Build/flash/capture commands and input/timing contract:
  [firmware README](../../firmware/README.md). Use each listed suite with
  `BENCH_SAMPLES=3`; pass its matching manifest to capture validation.

These are observed costs, not worst-case bounds, independent target numeric
verification, pass-detection completeness, or production settings. Matching
work counts does not prove positions or crossing timestamps agree numerically.

## Build and hardware context

- Clean source revision `eb86769ce786d9a1bd35a96ae9b370b435b613a0` throughout;
  no engine, harness, fixture, dependency, lockfile, or toolchain changes.
- Same confirmed S3-DevKitC-1/WROOM-1, S3 v0.2, 40 MHz crystal, 8 MB flash;
  PSRAM capacity unconfirmed and unused. User explicitly authorized all ten
  flashes/captures. Direct CP2102N UART `/dev/cu.usbserial-110` at 115200 baud.
- CPU 240 MHz, first core only, bare-metal f64/libm. No display, allocator,
  wireless, PSRAM, or concurrent application tasks. Display untouched.
- Xtensa rustc 1.97.0-nightly `8ea53bcd7` (toolchain 1.97.0.0), GCC 16.2.0
  (`esp-16.2.0_20260914`), espflash 4.6.0. Release opt-level 3, debug info 2,
  no LTO override, debug assertions off; no extra RUSTFLAGS/HAL overrides.
- Flash DIO/40 MHz/8 MB, existing bundled bootloader; application image 654,992
  bytes for each suite. Existing RWX LOAD linker warning remains.
- Firmware lockfile SHA-256:
  `abd11c0adcad8d4b28e492a3fe6cfff5280aefe4d27502a391a1b1a74387782c`.
- Sequential runs from 02:20 to 03:12 UTC on October 10 (October 9 local PDT).
  Board is left with `search-gnss`, three samples; resetting reruns that suite.
  No monitor is left running.

ELFs, build/flash logs, section/symbol inventories, runner, and offline summary
script are retained locally in
`firmware/target/measurements/2026-10-10-remaining/`. This ignored build directory
is **not a durable archive**; checked-in evidence excludes the ELFs themselves.

## Near-epoch mixed scaling

Tracking includes propagation, ECEF rotation, and observer look angles, but not
satellite geodetic altitude, rendering, or the rest of an application tick.
The per-tick values divide the invocation median by 1,441: amortized averages,
not individual-tick maxima. Search settings: 24h, 60s detection, 5s tolerance,
10° threshold, same Colorado observer. These are experiment inputs.

| Repeated slots | Tracking median (s) | Tracking per tick (ms) | Search median (s) | Search evaluations / pass records |
|---|---:|---:|---:|---:|
| 4, V2 baseline | 6.959246 | 4.8295 | 6.841015 | 5,836 / 9 |
| 16, earlier V2 run | 27.659558 | 19.1947 | 27.343044 | 23,344 / 36 |
| 64 | 110.504830 | 76.6862 | 109.365837 | 93,376 / 144 |

The 64-slot tracking samples span 110.503525–110.554367s; search spans
109.364388–109.370164s. Relative to the V2 four-slot medians, costs are 15.88×
tracking and 15.99× prediction for exactly 16× the work. This is a **same-harness,
separate-image comparison**, not one binary with runtime-selected sizes.
The old V1 baseline remains historical evidence, not the scaling denominator.

A full 64-slot tracking update at every intended 50ms/20Hz frame cannot fit
this measured kernel workload even on average, before rendering. That does not
rule out 64 entries with a different calculation cadence or establish a supported
capacity. The 109-second synchronous search cannot share a responsive display
update path without a scheduling change. No scheduling policy is selected here.

## Signed element ages

Each age is the center of an epoch-relative ±12h grid, not the window start.
Satellites retain their original elements and have different epochs; this is
not a common-UTC catalogue. Completing old-element propagation says nothing
about real-world accuracy or acceptable freshness.

**24h search medians in seconds**, 60s detection / 5s tolerance:

| Center age (days) | LEO | Resonant HEO | GEO | GNSS |
|---|---:|---:|---:|---:|
| −30 | 1.173302 | 21.016614 | 7.521279 | 1.587784 |
| −7 | 1.176258 | 6.620326 | 3.153744 | 1.558460 |
| −1 | 1.161462 | 2.848513 | 2.087602 | 1.585369 |
| 0 | 1.178064 | 2.352106 | 1.941862 | 1.586232 |
| +1 | 1.172837 | 2.836294 | 2.048503 | 1.587378 |
| +7 | 1.159658 | 6.592782 | 3.244086 | 1.583482 |
| +30 | 1.143971 | 21.002767 | 7.773296 | 1.653485 |

HEO search is about 8.9× its same-suite near-epoch cost at ±30 days; GEO reaches
about 4.0× at +30 days. HEO uses 1,449 evaluations at every age; GEO uses 1,441.
Evaluation count alone therefore does not bound wall time across element ages.
The full tracking grids show the same broad age sensitivity: HEO grows from
2.317802s at age zero to 20.919767s at −30 days, GEO from 1.936146s to 7.680524s
at +30 days. LEO/GNSS costs vary much less for these particular fixtures.
The CSV retains all tracking results, counts, and sample ranges.

Age and search-setting suites are separate sweeps, **not their Cartesian
product**: dense detection at old resonant ages has not been measured here.
Do not extrapolate the near-epoch mixed scaling table into an older catalogue.

## Search settings

Near-epoch 24h search medians (seconds), **5s crossing tolerance**:

| Detection interval | LEO | Resonant HEO | GEO | GNSS |
|---|---:|---:|---:|---:|
| 60s | 1.178064 | 2.379523 | 1.941864 | 1.586232 |
| 30s | 2.300480 | 4.728109 | 3.881857 | 3.160576 |
| 5s | 13.602823 | 28.297177 | 23.286641 | 18.920558 |

Reducing detection spacing dominates cost in these cases. At 60s detection,
tightening crossing tolerance from 5s to 250ms gives 24h medians of 1.222093s,
2.392580s, 1.941867s, and 1.595279s respectively. Tolerance adds work only for
detected transitions; it cannot recover an event missed between grid samples.
Same-setting values in separate suites vary slightly; retain each suite's
measurements rather than averaging across separately built images.

The 1h / 60s / 5s searches take 0.054557–0.100747s across these four fixtures,
versus 0.571651–1.186948s at 5s detection. They start at epoch −12h, just like
the 24h searches, so they are not proportional cuts of the same pass population.
All 48 setting combinations, including 250ms tolerance, are in the CSV.
These results do not justify coarsening detection or selecting a production
default; the [short-event limits](detection-intervals.md) remain independent.

## Memory inventory and next work

No peak-memory measurement was added. All suites report `Satellite` 512 B,
one four-fixture time grid 69,168 B, and prepared workload 560 B. All seven age
grids remain linked (484,176 B flash data). Most images have `.rodata` 511,228 B;
`ages-gnss` has 511,236 B. Other selected sections are identical: `.data` 2,952 B,
`.bss` 168 B, `.rwtext` 11,356 B, `.vectors` 1,024 B, `.text` 128,057 B.
The reserved `.stack` region is 326,256 B, **not measured peak stack use**.
Per-suite hashes and section inventory are in the artifact index.

Next: measure distinct-catalogue initialization/storage, pass aggregation,
and peak RAM/stack; broaden orbital samples and check independent target
numerics. Then choose supported size, element-age limits, calculation/prediction
cadence, scheduling/caching, and explicit over-budget behavior with application
headroom. Minimal Sharp output remains a separate early check. No production
default or decision contract changed; M2 remains open.
