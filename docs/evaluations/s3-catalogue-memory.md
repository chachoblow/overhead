# First S3 catalogue heap and written-stack measurements

Measured 2026-10-10 UTC, with explicit flash approval. **Initial bounded target
costs, not whole-device peak RAM, a supported catalogue size, or completion of M2.**
The prepared experiment ran unchanged; no source, dependency, or toolchain changes
were made during capture. Firmware now contains this three-sample experiment.

## Evidence and method

Two reset-separated runs of eight cases, three single-invocation samples per case
after one discarded warm-up: 48 recorded samples total. Both complete matrices
validate against the separately executed host manifest; all work and heap counters
agree within/between runs. The release assertions also confirm that every sample
returns requested and allocator-reported occupied bytes to its empty-heap baseline.
No failed allocation, firmware panic, partial run, or discarded timing outlier.

- Unmodified serial output: [run 1](s3-catalogue-memory-run-1.log),
  [run 2](s3-catalogue-memory-run-2.log).
- [Host work manifest](s3-catalogue-memory-expected.txt).
- [Per-run/case summary CSV](s3-catalogue-memory-summary.csv): min/median/max integer
  microseconds for both phases, heap accounting, work counts, and observed stack.
  Medians are the middle of the three samples; no calibration/batching/filtering.
- [Artifact/source/input hashes and ELF section inventory](s3-catalogue-memory-artifacts.json).
- [Measurement contract, lifetimes, commands, and interpretation](../../firmware/CATALOGUE_MEMORY.md);
  [original input provenance](../../tools/fixtures/README.md#catalogue-coststle).

The same 4/8 distinct historical objects and common UTC as the
[host experiment](catalogue-costs.md), not repeated references to four satellites.
Flash-resident converted OMM inputs are 1,679/3,331 bytes. One group; no rejected,
duplicate, or conflicting records. Observer is Colorado, threshold 10°, detection
60s, tolerance 5s. Per-satellite guard is 200,000; complete shared allowances are
800,000/1,600,000. These remain experiment settings, not production defaults.

## Heap results

Bytes below are **absolute requested / allocator-reported occupied** values.
Aggregation retains the catalogue; its columns must not be added to catalogue
storage again. Initialization dominates the instrumented heap peak in all cases.

| Distinct objects | Init peak requested / occupied | Catalogue retained requested / occupied | Catalogue + 24h results retained requested / occupied | Additional 24h results occupied |
|---:|---:|---:|---:|---:|
| 4 | 10,488 / 10,520 | 2,792 / 2,824 | 4,088 / 4,120 | 1,296 |
| 8 | 15,959 / 16,024 | 5,575 / 5,640 | 8,023 / 8,088 | 2,448 |

Initialization makes 35/69 successful allocation calls requesting 18,200/31,447
bytes in total for 4/8 entries. Traffic is not peak. Four/eight-entry 24h
initialization medians are 19,901/38,894 µs in both runs. Eight entries fit this
experiment's 65,536-byte region; that is not a capacity limit or safety bound.

The wrapper includes allocate-copy-free overlap during growth and reports LLFF
block occupancy separately. The host System wrapper measured logical reallocation
instead. Eight-entry target requested peak (15,959) exceeds the earlier host
logical peak (14,279) despite smaller pointers: these results must not be compared
as a pointer-width scaling formula or attributed solely to architecture.

| Objects | Window / shared allowance | Evaluations | Stored passes | Searched objects | Combined retained occupied bytes | Aggregation median seconds, run 1 / run 2 |
|---:|---|---:|---:|---:|---:|---:|
| 4 | 1h / 800,000 | 252 | 1 | 4 | 3,672 | 0.291554 / 0.291553 |
| 4 | 24h / 800,000 | 5,812 | 6 | 4 | 4,120 | 7.126651 / 7.126652 |
| 4 | 24h / 1,000 | 1,000 | 2 | 1 | 3,672 | 0.817618 / 0.817617 |
| 4 | 24h / 0 | 0 | 0 | 0 | 3,304 | 0.000190 / 0.000190 |
| 8 | 1h / 1,600,000 | 500 | 3 | 8 | 7,416 | 0.585946 / 0.585945 |
| 8 | 24h / 1,600,000 | 11,616 | 13 | 8 | 8,088 | 14.527805 / 14.527804 |
| 8 | 24h / 1,000 | 1,000 | 2 | 1 | 6,968 | 0.817568 / 0.817568 |
| 8 | 24h / 0 | 0 | 0 | 0 | 6,600 | 0.000195 / 0.000195 |

Nonzero-budget cases retain one earliest candidate. Limited/zero cases remain
incomplete; retained candidates do not establish a global next arrival. Zero
budget still allocates unsearched reports. Timing includes instrumentation and
orbital prediction, not merely storage overhead, and is not a worst-case bound.

## Written-stack observation and static inventory

The known-local probe reports 1,120 bytes at baseline and 9,068 with a written
8 KiB local (7,948 more), satisfying its 4 KiB response check in both boots.
Every measured sample reports an observed write depth of **5,824 bytes**, including
live-frame/safety margin and initialization/aggregation/validation/drop/probe costs.
Even zero-budget cases report that depth; this aggregate mark cannot be assigned
to the orbital kernel or a particular phase. No per-phase stack maximum measured.

CPU0 linker bounds are `0x3fc9bea8..0x3fcdb700` (260,184 bytes available).
Painting begins above the HAL guard at `0x3fc9bee8`; all samples retain 254,296
untouched bytes there. This is a write watermark, **not maximum reserved stack**:
unwritten reservations/pattern matches can hide use, probe frames can inflate it,
and boot history is excluded. Never use 5,824 as a safe stack allocation.

Selected ELF sections: `.data` 3,244, `.bss` 65,704 (including the entire 65,536-byte
heap), `.rwtext` 11,588, `.vectors` 1,024, flash `.rodata` 83,256, `.text` 237,689.
Full inventory is in the artifact JSON. `.stack` is available space, not actual
usage; alias/dummy sections must not be double-counted. Occupied heap is already
inside reserved `.bss`; do not add it again. No simultaneous whole-device RAM peak
is inferred by adding separate heap/stack maxima. Application image is 368,848 bytes.

## Build / boot context and warnings

Confirmed S3-DevKitC-1/WROOM-1, chip v0.2, 8 MB flash, direct CP2102N UART;
PSRAM unconfirmed/unused, display disconnected, one CPU at 240 MHz. Installed
Xtensa Rust 1.97.0.0, LLVM 21.1.3, release opt-level 3/debug 2, no LTO override,
`-Tlinkall.x`, f64/libm, esp-alloc 0.11.0 LLFF, espflash 4.6.0 with DIO/40 MHz.
No environment compiler/profile overrides. Base revision plus dirty preparation
is identified by per-source hashes; it is not claimed to be a committed build.
ELF, full source snapshot, build/flash logs, and disassembly remain local ignored
artifacts, not a durable binary archive. Generated JSON hashes were checked against
all cached target copies and the exact input bytes were found in the flashed ELF.

The existing linker RWX warning remains. **Both boots additionally report**:
`Image contains multiple DROM segments. Only the last one will be mapped.`
This message is absent from the earlier V2 kernel baseline capture. The new logs
show the 256-byte app descriptor and rodata as separate DROM image segments, though
ELF program headers group them together. Both boots complete all checked work;
that does not establish the diagnostic is harmless for every image/use. It has
not been fixed or suppressed, and no linker/toolchain change was attempted.
[Upstream discussion of similar splitting](https://github.com/esp-rs/espflash/issues/927)
is context, not an independent diagnosis of this build.

## Verification and next

Workspace check/tests (150 + 6 doctests), ten feature-enabled firmware host tests,
11 Python tests, both saved-log validations, all summary/source/input/stack
cross-checks, and the approved flash/captures pass. Earlier preparation owns the
strict Clippy/fmt checks; source code was unchanged for these runs. No monitor is
left running; resetting the board reruns this three-sample catalogue experiment.

M2 remains open. Investigate the boot image-mapping diagnostic; broaden distinct
populations, observer/pass densities, duplicate/conflict diagnostics, collection
growth and input RAM lifetimes; add independent target numerical checks and stronger
stack evidence. Only then select supported catalogue size, cadence, scheduling/
caching, and explicit over-budget behavior. A synchronous ~14.53s 24h batch is not
a display-loop tick; no production scheduling policy has been selected here.
