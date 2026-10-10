# ESP32-S3 kernel benchmark

Early M2 measurement firmware, **not the device application**. The default kernel
binary has no display, Wi-Fi, allocator, PSRAM, catalogue ingestion/aggregation,
or scheduling. A separate opt-in [catalogue memory experiment](CATALOGUE_MEMORY.md)
now has first target captures; its feature/build commands are distinct.
All orbital calculations still run in `overhead-core`. Results and limitations:
[first S3 evaluation](../docs/evaluations/s3-costs.md),
[16-slot follow-up](../docs/evaluations/s3-scaling-16.md), and
[remaining V2 suites](../docs/evaluations/s3-expanded.md). All 122 prepared
kernel workloads are host-tested and now have target captures. Broader orbital
samples, independent target numeric checks, and whole-device peak-memory budgets
remain pending; the separate memory experiment supplies only its bounded evidence.

## Hardware and permission

Ask before installing toolchains or flashing. Flashing replaces the existing
application and writes a bootloader/partition table; it is not a read-only test.
The capture command resets the device but does not flash it.

Confirmed board: ESP32-S3-DevKitC-1, WROOM-1 module, S3 revision v0.2, 8 MB flash.
PSRAM variant/capacity is **unconfirmed and unused**. Use a data-capable micro-USB
cable in the board's **UART** port (CP2102N bridge). Direct laptop connection
worked during bring-up; the Anker dock path did not enumerate. No drivers were
installed. Display remains disconnected; soldering is unnecessary for this test.

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
passes offline; corrected target boot/capture is still pending. Stress-test
artifacts are not flash candidates. The gate does not itself authorize flashing.

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
larger distinct working set. Additional orbital fixtures remain future work.

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
  orbit samples and independent target numeric checks remain pending; initial
  distinct-catalogue heap/write-watermark evidence lives in the separate experiment.
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
python3 -m unittest discover -s firmware -p 'test_*.py'
```

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
