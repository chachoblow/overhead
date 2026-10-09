# ESP32-S3 kernel benchmark

Early M2 measurement firmware, **not the device application**. No display,
Wi-Fi, allocator, PSRAM, catalogue ingestion/aggregation, or scheduling. All
orbital calculations still run in `overhead-core`. Results and limitations:
[S3 evaluation](../docs/evaluations/s3-costs.md).

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
cargo build --release --locked
```

Default Cargo release optimization (`opt-level=3`), debug information retained,
no LTO override. The initial GCC linker emits a LOAD-segment RWX warning for the
embedded image; build/flash/boot succeed. No warning suppression is configured.

## Flash and capture

From `firmware/`, after approval and checking the port:

```sh
espflash flash --port /dev/cu.usbserial-110 --chip esp32s3 \
  --flash-size 8mb --flash-mode dio --flash-freq 40mhz \
  --non-interactive --skip-update-check \
  target/xtensa-esp32s3-none-elf/release/overhead-s3-benchmark

python3 capture.py --port /dev/cu.usbserial-110 \
  --elf target/xtensa-esp32s3-none-elf/release/overhead-s3-benchmark \
  --output /tmp/overhead-s3-run.log
```

`capture.py` uses only the Python standard library plus installed `espflash`.
It resets the board, saves unmodified serial/monitor output, stops at the DONE
marker, and validates 14 workloads × 5 samples and expected work counts. It
refuses to overwrite a result and retains partial logs on timeout/failure.
Allow roughly four minutes per run; default timeout is 600 seconds. Repeat with
a different output filename for another run. After DONE the firmware spins;
resetting reruns the benchmark. The display and second CPU core are unused.

## Measurement contract

The build script reads the **original** [ISS OMM](../core/tests/fixtures/iss-25544.json)
and [HEO/GEO/GNSS TLEs](../tools/fixtures/pass-intervals.tle), emits lossless numeric
element fields, and precomputes the same timestamp grids as the
[host benchmark](../tools/README.md#overhead-benchmark). Fixture provenance stays
in [tools/fixtures/README.md](../tools/fixtures/README.md). No live data is fetched.

- Four orbit classes × propagation-only, propagation/ECEF/look angles, and pass
  search; plus four repeated fixture slots for full tracking and pass search.
- Each tracking invocation evaluates 1,441 timestamps at 60s spacing, from each
  fixture's own epoch −12h to +12h. Mixed slots iterate times then satellites.
- Search: 24h, 60s detection, 5s crossing tolerance, 10° threshold, Colorado
  observer (39.007°, −104.883°, 2.187 km). These are **experiment settings**, not
  production defaults or an accuracy guarantee. Each fixture has a different
  epoch; this is not a common-UTC catalogue.
- Each row has one untimed warm-up, then five samples of one invocation each.
  `esp_hal::time::Instant` measures elapsed microseconds at 240 MHz CPU clock.
  Inputs/outputs use `black_box`; UART printing is outside timed sections.
  All searches must complete and repeated work counts must agree.
- Initialization, parsing, timestamp/config preparation, and reporting are not
  timed. Timestamp arrays occupy flash rodata; four satellite instances and
  search state use internal RAM/stack. No heap or PSRAM is used.
- The host's calibration/batching is not reproduced: device invocations are
  already long. Workload operations/counts match 14 selected host rows, but
  compiler, CPU, pointer width, and memory placement differ.
- Full geometry means ECEF rotation and observer look angles, not geodetic
  satellite altitude, rendering, or a complete application tick.
- 16/64-slot batches, interval/tolerance sweeps, older element ages, broader
  orbit samples, and distinct-catalogue/peak-memory measurements remain pending.
  The first larger-matrix capture timed out; partial exploratory output is not
  part of the published final-build run.

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
cargo +stable clippy --manifest-path firmware/Cargo.toml --lib --tests -- -D warnings
cargo fmt --manifest-path firmware/Cargo.toml -- --check
python3 -m unittest discover -s firmware -p 'test_*.py'
```

Firmware library tests check fixture IDs/time grids and all baseline work counts,
including host-only repeated-slot accounting at 16/64 slots. Capture tests check
complete and malformed matrices, missing/duplicate records, and failure markers.
