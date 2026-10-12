# Frozen M2 S3 harness

Implemented/offline-gated and captured with approval 2026-10-11 UTC.
**All four frozen target captures pass**: [results/evidence](../docs/evaluations/s3-m2.md).
Any future flash/reset still requires approval.
The [frozen manifest/representation](../docs/evaluations/m2-frozen-manifest.md)
and [capture contract](../docs/evaluations/m2-next-evidence.md) remain authoritative.
No source refresh, added workload dimension, dependency, toolchain change or
production default. Historical suites/protocols remain separately runnable.

## Sources and build

**Use the historical source revision** from the
[pinned export workflow](../docs/evaluations/s3-m2/README.md#offline-validation-and-table-regeneration)
for this frozen harness and its gates. The current M3 workspace has a changed
root lockfile and intentionally fails the freeze guard. Do not renew old evidence
or bypass its hashes; current simulator checks are a separate verification scope.

`generate_m2.py --check` checks the entire frozen manifest, then compares
`src/m2_fixtures.rs` with a lossless conversion of archived document strings.
Each distinct document is embedded once; repeated groups still parse/count it
independently. Static group names, keys and source references accompany the cases.
No manifest JSON parser or source conversion runs on target.

The new binaries are auto-discovered Cargo **examples**, so pinned `Cargo.toml`
and `build.rs` stay unchanged. `src/m2.rs` implements the exact checked pipeline;
`examples/m2_support/target.rs` owns instrumentation and UART, not orbit logic.
The examples' host mains are instructions, not simulated target measurements.

Fresh sessions need only tracked sources plus the existing installed toolchains,
cached Cargo dependencies, Python 3.11+ and Homebrew SDL2 described in the
[host prerequisites](../tools/fixtures/m2-cases/README.md#offline-regeneration).
Old ignored `target/` directories are not required: the exact staged-source export
passes all host/evidence checks and builds/gates both normal images from scratch.
Freshly built image hashes may differ; never substitute them for the archived
capture identities or treat an offline rebuild as flash approval.

From repository root, run the [existing host/preflight gates](../tools/fixtures/m2-cases/README.md#offline-regeneration), then:

```sh
python3 firmware/generate_m2.py --check
python3 -m unittest discover -s firmware -p 'test_*.py'
cargo +stable test --offline --locked --manifest-path firmware/Cargo.toml --lib \
  --features catalogue-memory,numerical-reference
cargo +stable clippy --offline --locked --manifest-path firmware/Cargo.toml --lib \
  --tests --examples --features catalogue-memory,numerical-reference -- -D warnings
cargo fmt --manifest-path firmware/Cargo.toml -- --check
. "$HOME/export-esp.sh"
python3 firmware/prepare_m2_images.py firmware/target/m2-normal-NEW
```

Use a **new** output directory. The runner builds normal `m2_measure` and
`m2_diagnostic` images with the installed toolchain; it never opens a port.
It retains exact ELF/application images, source/manifest/tool/build hashes and
commands, allocated-section/symbol inventories, disassembly and symbol-start
ENTRY frame inventories. Both ELF/image pairs must pass the [image gate](IMAGE_GATE.md).
A rebuild needs a fresh gate; older or stress artifacts are not substitutes.

Manual target builds, from `firmware/` after sourcing the existing exports:

```sh
cargo build --offline --release --locked --example m2_measure --features catalogue-memory
cargo build --offline --release --locked --example m2_diagnostic \
  --features catalogue-memory,numerical-reference
```

These manual output paths are `target/xtensa-esp32s3-none-elf/release/examples/`;
manual builds alone do not archive/gate images. `BENCH_SAMPLES` is historical:
M2 always uses the frozen one warmup/three measured samples, not that variable.

## What is measured

Both binaries reserve the same 65,536-byte internal heap; one CPU, no task
switching, PSRAM, display or network. `m2_measure` runs all 20 cases, one warmup
then three samples. No stack painting or numerical sweep occurs in timed runs.
`m2_diagnostic` executes the unchanged 75-row numerical checker, then all cases
once for each required stack pattern. Its phase timing fields are zero and are
**not timing evidence**. Capture each image on two separately reset boots.

The fixed four-slot RAM-copy table is on stack. Copy allocation starts only after
pipeline counters start; dropped inputs are released after initialization,
retained inputs after aggregation. Owned caller errors/core diagnostics and
original catalogue metadata/reports/passes/arrival candidates stay alive through
the endpoint. Only case 16 omits aggregation; empty catalogue is a failure.

Input-copy, initialization, input-release, aggregation and destruction have
separate absolute requested/backend peak/live counters. Separate whole-pipeline
counters survive every phase reset. Both occupancies must return to zero, and
work/heap/collection counters must repeat; no heap-size or allocator-policy change.
Allocation instrumentation and per-record snapshots are included in phase costs.
Configuration, outcome verification, formatting and UART are outside phase timers;
endpoint output borrows still-live objects without allocating. Pipeline accounting
also catches any unexpected allocation outside the named phases.

`RAW` reports actual parser-array capacities; `RECORD` reports per-record allocation
calls/requested traffic and absolute requested/backend live occupancy. `OWN` reports
actual caller-error and arrival-vector capacities, then **lengths**, not capacities,
for core-private entries, diagnostics, memberships, reports, passes and conflict
origins. No host allocation ledger is transplanted and private capacities are not
inferred from host values. These observations expose growth/retention costs without
claiming a full target collection-capacity inventory. Protocol code/tests own exact
field ordering and rejection rules.

## Capture, after separate flash approval

Flashing replaces the existing experiment. Follow the established [flash options](README.md#flash-and-capture)
using the approved archived normal ELF, **not** an output left by stress builds.
The following capture command resets/monitors but never flashes; do not run it
before permission to access/reset the board. Replace the example archive/port.

```sh
python3 firmware/m2_capture.py capture --port /dev/cu.usbserial-110 \
  --elf firmware/target/m2-normal-NEW/m2_measure.elf \
  --image firmware/target/m2-normal-NEW/m2_measure.bin \
  --mode measure --output /tmp/m2-measure-boot-1.log
# Repeat to a new boot-2 filename. After approved diagnostic flash, use
# m2_diagnostic.{elf,bin}, --mode diagnostic, and two new diagnostic filenames.
python3 firmware/m2_capture.py validate /tmp/m2-measure-boot-1.log
python3 firmware/m2_capture.py validate-set \
  --measure /tmp/m2-measure-boot-1.log /tmp/m2-measure-boot-2.log \
  --diagnostic /tmp/m2-diagnostic-boot-1.log /tmp/m2-diagnostic-boot-2.log
```

Before reset the capture tool checks the freeze/generated tables and exact
ELF/image pair. It refuses existing output/sidecar files, records gate/command/
timeout provenance, and preserves partial logs on failure/timeout. Markers are
`OVERHEAD_S3_M2_V1`/`OVERHEAD_S3_M2_DONE`, distinct from historical protocols.
The validator requires exact ordered outcomes, owned messages/origins, sample
matrix, full release, consistent peaks, numerical rows and phase/pattern stack
observations. It compares heap/collection counters across both boots and images.
Unknown/missing/repeated/malformed records fail closed. Synthetic Python test logs
use invented memory/timings and are **not target evidence**.

Four valid files alone cannot prove separate physical resets, correct flashed
image or complete experiment provenance. Review the raw boot/capture commands,
exact build archive and frame limitations before accepting evidence. Failed/OOM
runs remain failed experiments, not graceful overload behavior.

### Timeout allowance, not a runtime bound

The frozen suite totals **219,681 search evaluations** per complete traversal.
Historical eight-object aggregation was ~14.528 s / 11,616 evaluations (~1.25 ms
each); near-epoch HEO sweeps were ~2.35–2.85 s / 1,449 (~1.62–1.97 ms each).
See [catalogue costs](../docs/evaluations/s3-catalogue-memory.md#heap-results)
and [signed-age costs](../docs/evaluations/s3-expanded.md#signed-element-ages).
Use a deliberately conservative **10 ms/evaluation capture allowance**, plus
300 s for ingestion/UART/numerics/painting, then a further 2× margin, rounded
up to a minute. Four traversals per timed boot and two per diagnostic boot give:

- timed: **18,180 s** per boot;
- diagnostic: **9,420 s** per boot.

`python3 firmware/m2_capture.py timeouts` recomputes these values. They are generous
failure deadlines, not predicted duration; capture exits immediately on DONE/failure.
Evaluation count does not bound runtime across arbitrary old resonant elements.
No timeout or historical sample maximum establishes scheduling latency.

## Offline image/frame review and remaining limits

Latest local normal archive: `target/m2-normal-implementation-final-2/report.json`.
It is ignored local output, **not a durable binary archive**. The approved
capture archive is `target/m2-capture-20261011T0423/`; raw logs, source/image hashes,
reset provenance and results are [tracked separately](../docs/evaluations/s3-m2/README.md).
Both images have
one DROM segment, 64-byte rodata alignment, and a 32-byte file-backed merge.
The separate RWX warning remains visible. The full historical image regression
(8 positive cases plus NOBITS negative control) also passes.

| Allocated section bytes | Timed | Diagnostic |
|---|---:|---:|
| `.rodata` | 129,200 | 131,952 |
| `.text` | 274,669 | 296,961 |
| `.data` | 3,724 | 3,940 |
| `.bss` | 65,704 | 65,704 |
| reserved `.stack` | 259,704 | 259,496 |

Full section/symbol inventories accompany each ELF. Alias/dummy sections must not
be double-counted. The 64 KiB heap is already inside `.bss`; occupied heap is not
additional static RAM. This is not whole-device memory measurement.

Reviewed ENTRY reservations in the exact normal builds: harness run/sample
2,128/1,920 B timed and 2,192/2,000 B diagnostic; loader 448 B, builder push/finish
1,328/864 B, B-tree insertion 2,880 B, diagnostic sorting 4,144 B. Prediction passes
through the retained catalogue search/evaluator to propagation/geometry; evaluator
240 B, SGP4 propagation-from-state 368 B and resonance integration 704 B are
individual reservations, **not a sum or a call-chain bound**. The diagnostic smoke
function retains its 8 KiB written local in an 8,240-byte frame. Paint has a 32-byte
frame and the fill loop remains a no-call assembly loop below SP minus 256 bytes;
scan also checks the unchanged HAL guard and untouched lower margin.

Painting is repeated at each applicable phase with both patterns. Observations
include live caller frames, the safety gap, probe and interrupt effects. Unwritten
reservations/pattern matches can hide deeper use; probe frames can inflate it.
ENTRY inventories exclude nonstandard/dynamic reservations; disassembly also
contains decoded literal pools. Many `callx` sites resolve through literal loads,
others need separate indirect-call analysis. No complete recursion/indirect-call,
ABI spill or interrupt bound is established. Two patterns and smoke/guard checks
still do **not** establish a safe or worst-case stack allocation.

Both patterns, smoke/guard checks and all numerical rows now pass in two target
boots; observed phase depths reach 9,248 bytes, without resolving these limitations.
Next: measured operating-budget decisions and separate bounded-scheduling
verification. Sharp bring-up remains separate.
