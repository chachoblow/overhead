# Bounded S3 catalogue memory experiment

**First two target runs captured:** [results, raw evidence and limits](../docs/evaluations/s3-catalogue-memory.md).
This is an opt-in M2 experiment, not device firmware or a supported catalogue size.
The default [kernel benchmark](README.md) remains allocation-free. Scope/rationale:
[0017](../docs/decisions/0017-target-catalogue-memory-experiment.md).

## Measurement contract

Eight cases reproduce the existing [host experiment](../docs/evaluations/catalogue-costs.md):
4/8 distinct historical objects, common UTC 2006-06-26T00:00:00Z; Colorado observer,
10° threshold, 60s detection, 5s tolerance; 1h/24h complete searches and 24h shared
allowances of 1,000/zero. Per-satellite allowance is 200,000. No new orbital samples,
observer densities, duplicate/conflict cases, or operating defaults are introduced.
Input provenance remains in the [fixture README](../tools/fixtures/README.md#catalogue-coststle).

- Host build script parses original TLEs and emits the same Serde OMM conversion
  used by the host experiment. Both 4/8 JSON arrays live in flash rodata. Neither
  TLE conversion nor an input RAM/network buffer is measured. Each case reports
  its selected JSON byte length; ELF sections account for both linked arrays.
- `catalogue-memory` enables core catalogue/OMM and no_std Serde JSON only for the
  optional experiment. `esp-alloc` 0.11.0 supplies the fixed 65,536-byte internal
  heap; `.cargo/config.toml` pins its LLFF algorithm. No C allocator exports,
  PSRAM, second core, tasks, display, wireless stack, or live data.
- Initialization times whole-array raw-record parsing, checked deserialization,
  SGP4 initialization, catalogue merge and finish. Array records/build temporaries
  die before the phase ends; accepted catalogue remains live.
- Aggregation times synchronous `search_catalogue` plus `earliest_candidates`;
  all reports/passes/candidates remain live at its endpoint. Configuration and
  validation/reporting are outside both timers. All retained objects are then
  dropped; both requested and allocator occupancy must return to baseline.
- One discarded warm-up, then 1–5 invocations per case. Timer is HAL `Instant` at
  240 MHz. Counters/allocator instrumentation are inside timing; UART is outside.
  Work and heap accounting must repeat; no timing or stack-repeatability assertion.

## What the counters mean

`PHASE` reports **absolute** requested peak/live bytes and backend-reported
occupied peak/live bytes, plus successful allocation calls, total requested bytes,
failed calls, and elapsed microseconds. Aggregation values include the retained
catalogue, unlike the host report's phase-extra values. Take `max(init, aggregate)`
for the instrumented pipeline heap peak; do not add those absolute peaks.

The wrapper uses Rust's allocate-copy-free reallocation, so old/new blocks coexist
and count toward both peaks. This differs from the host System wrapper's logical
reallocation accounting. LLFF occupancy includes its block rounding/minimum sizes;
it is an allocator estimate, not total RAM. Free-list metadata lives inside the
reserved heap. The entire 64 KiB heap is already in `.bss`: do not add its occupied
bytes to `.bss` again. No allocator policy is selected for the future device app.

The CPU0 stack probe paints only below current SP minus 256 bytes, above the HAL
stack guard. A no-call assembly loop prevents painting its own live frames. Scan
occurs after validation and destruction, before reporting; it includes harness,
probe, and drop costs but excludes earlier boot/init history. A startup `PROBE`
smoke check must observe at least 4 KiB more depth with a known written 8 KiB local.
Each sample needs at least 256 untouched bytes above the guard. These checks can
invalidate a run; they do not prove absence of every kind of stack overflow.

`STACK` records linker bottom/top, painted bounds, untouched bytes, and observed
write depth (including the unpainted live-frame/safety margin). Unwritten stack
reservations and coincidental pattern matches can hide use. **This is not maximum
reserved stack or a worst-case bound.** Inspection of the compiled probe is part
of preparation; host scanner tests alone cannot verify Xtensa stack behavior.
No simultaneous whole-device RAM peak is inferred by summing separate maxima.

OOM, panic, incomplete expected work, failed probe, unreleased storage, or changed
work/heap counters invalidate the experiment. Preserve partial logs; never treat
them as a smaller supported catalogue or graceful product over-budget behavior.

API/source review: [esp-alloc 0.11.0](https://docs.rs/crate/esp-alloc/0.11.0/source/src/lib.rs)
and the pinned [esp-hal 1.2.2 stack layout](https://docs.rs/crate/esp-hal/1.2.2/source/ld/sections/stack.x),
also inspected locally alongside the installed Xtensa runtime.

## Build and capture

Use the installed toolchain; ask before any toolchain change or flash. Always
select the new binary explicitly with this feature (do not feature-enable the
old kernel binary). `BENCH_SUITE` still belongs to the old harness; the new matrix
is fixed and only `BENCH_SAMPLES` selects its sample count.

From repository root, prepare external same-model work expectations:

```sh
cargo +stable run --release --locked --manifest-path firmware/Cargo.toml \
  --features catalogue-memory --example catalogue_expected -- 3 \
  > /tmp/overhead-catalogue-expected.txt

cd firmware
. "$HOME/export-esp.sh"
BENCH_SUITE=baseline BENCH_SAMPLES=3 cargo build --release --locked \
  --features catalogue-memory --bin overhead-s3-catalogue-memory
```

Retain source revision/diff, lockfile and generated JSON hashes, compiler/profile,
ELF/hash, `xtensa-esp32s3-elf-size -A` output, and probe disassembly with each run.
The `.stack` section is available linker space, **not measured stack use**. Inspect
`stack_watermark::target::paint` for its bounded no-call loop and `exercise_stack`
for the retained 8 KiB local. The existing linker RWX warning remains unsuppressed.

Before requesting flash approval, run the [offline image gate](IMAGE_GATE.md)
on the exact normal-build ELF/application image. Do not use stress-runner Cargo
outputs as measurement builds. The local source remedy passes offline; a corrected
target boot is not yet captured.

From `firmware/`, **only after flash approval and checking the current port**:

```sh
espflash flash --port /dev/cu.usbserial-110 --chip esp32s3 \
  --flash-size 8mb --flash-mode dio --flash-freq 40mhz \
  --non-interactive --skip-update-check \
  target/xtensa-esp32s3-none-elf/release/overhead-s3-catalogue-memory

python3 capture.py --port /dev/cu.usbserial-110 \
  --elf target/xtensa-esp32s3-none-elf/release/overhead-s3-catalogue-memory \
  --expected /tmp/overhead-catalogue-expected.txt \
  --output /tmp/overhead-catalogue-run-1.log --timeout 900
```

Repeat capture with a new filename; it resets but never flashes. The 900s timeout
is an initial allowance, not measured runtime. On completion firmware spins;
resetting reruns the experiment. `capture.py` validates the manifest before reset,
requires the ordered complete case/sample matrix, compares host work counts,
checks heap/stack consistency and the probe smoke check, and preserves raw/partial
logs without overwriting. Synthetic Python test logs are not target evidence.
Matching work counts is not independent target numerical accuracy.

## Offline verification / next step

From repository root, in addition to the default checks in [README](README.md):

```sh
cargo +stable test --manifest-path firmware/Cargo.toml --lib --locked --features catalogue-memory
cargo +stable clippy --manifest-path firmware/Cargo.toml --lib --tests --examples \
  --locked --features catalogue-memory -- -D warnings
python3 -m unittest discover -s firmware -p 'test_*.py'
```

Preparation passed workspace check/tests (150 + 6 doctests), default/feature
firmware host tests (6/10), default/feature strict host Clippy, strict target Clippy
for the new binary, formatting, 11 Python tests, a generated/parsed release host
manifest, and both target release builds. Probe disassembly retains its no-call
paint loop and known stack local. That preparation did not access hardware; the
subsequent approved captures and their verification are in the linked results.
Both historical captured boots report a multiple-DROM mapping diagnostic despite
completing. The [source remedy and image gate](IMAGE_GATE.md) now pass offline,
but no corrected target boot/capture exists yet; historical results are unchanged.

Next: approved corrected boot/capture, then refine/freeze the
[bounded evidence proposal](../docs/evaluations/m2-next-evidence.md) before broader
measurements or selecting M2 capacity/cadence/scheduling policies.
