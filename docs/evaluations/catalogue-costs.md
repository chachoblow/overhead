# Distinct-catalogue host allocation and aggregation costs

Measured 2026-10-10 UTC. **Host requested heap, not S3 peak RAM or a supported
catalogue size.** This closes an initial host measurement slice, not M2.

## Inputs and method

`overhead-catalogue-benchmark` embeds eight distinct historical satellites from
the pinned Vallado/CelesTrak source. [Fixture provenance](../../tools/fixtures/README.md#catalogue-coststle)
owns selection/regeneration; [usage](../../tools/README.md#overhead-catalogue-benchmark)
owns the CLI. No existing fixture, core implementation, dependency, or firmware
was changed. Four-entry prefix: LEO, resonant HEO, GNSS, GEO. Eight entries add
another LEO/HEO and two GEOs; this is not a representative production population.
No fabricated IDs or repeated satellite slots.

All satellites use the **same** start, `2006-06-26T00:00:00Z`; individual epochs
are within two days. Observer: 39.007° N, −104.883° E, 2.187 km ellipsoidal height.
Experiments use 10° elevation, 60-second detection, 5-second crossing tolerance:
1h and 24h complete searches plus 24h with shared allowances of 1,000 and zero.
Per-satellite guard is 200,000 evaluations; complete-search shared guard is that
times catalogue size. These are experimental settings, not operating defaults.

Two processes, five single-invocation samples per row after one discarded
warm-up. Raw evidence: [run 1](catalogue-costs-run-1.json),
[run 2](catalogue-costs-run-2.json). No batching/calibration or outlier removal.
Each sample has two separately timed/requested-heap-instrumented phases:

1. **Initialization:** whole OMM-array syntax parsing into borrowed raw records,
   checked record deserialization, `CatalogueBuilder` SGP4 initialization/merge,
   and finish. One group; no duplicates, conflicts, or rejected records.
2. **Aggregation:** synchronous `search_catalogue` and `earliest_candidates`,
   retaining all reports, pass records, and candidates. Catalogue remains live.
   This includes orbital search, not just the incremental cost of collecting it.

Preparation (TLE parsing, TLE-to-OMM serialization, input JSON, config), output
formatting, validation, and destruction are outside both phases. Input JSON byte
length is reported separately; its heap capacity is **not** included. This is
not the full manifest/file/provenance-reporting tool path. The standard existing
Serde conversion may round binary floats slightly; fixture tests bound the nine
orbital fields' conversion error to two `f64::EPSILON` relative and require exact
identity/epoch/other metadata. No float-parser feature was changed.

The executable alone wraps `System` with allocation-free atomic counters:
- Live requested bytes and logical peak; successful alloc/zeroed/realloc counts;
  sum of requested sizes (full new size for realloc).
- Realloc replaces the old logical request; transient old+new storage inside
  System, allocation headers/alignment overhead/fragmentation are not visible.
- Each phase excludes already-live allocations. Combined peak is
  `max(initialization_peak, catalogue_retained + aggregation_peak)`.
- All measured catalogue/results are dropped before confirming live bytes return
  to the pre-sample baseline. The process is single-threaded; these snapshots
  must not be used with concurrent allocations or freeing preexisting objects.

Timing includes atomic instrumentation overhead and normal kernel validation.
Neither timing nor heap counts are a cross-build/architecture contract. Tiny
zero-budget timings are especially dominated by timer/instrumentation overhead.

## Results

Every sample in both processes agrees on work and allocation counts. Untimed
checks compare **full retained reports and pass records** with direct streaming
single-satellite searches sharing the same budget. This is same-model accounting
validation, **not independent position/crossing accuracy**.

| Distinct satellites | Init peak bytes | Catalogue retained bytes | 24h aggregation peak/retained bytes | Combined retained bytes | Combined peak bytes |
|---:|---:|---:|---:|---:|---:|
| 4 | 11,320 | 2,968 | 1,376 | 4,344 | 11,320 |
| 8 | 14,279 | 5,927 | 2,560 | 8,487 | 14,279 |

Initialization: 35/69 successful allocation/reallocation calls for 4/8 entries;
26,744/48,103 total requested bytes across calls. Peak is not total allocation
traffic, and retained size is not peak. Collection capacities/growth matter;
do not extrapolate these two points as a linear capacity formula.

| Satellites | Window / shared allowance | Evaluations | Stored passes | Searched satellites | Aggregation retained bytes | Aggregation median ms, run 1 / run 2 |
|---:|---|---:|---:|---:|---:|---:|
| 4 | 1h / 800,000 | 252 | 1 | 4 | 928 | 0.135 / 0.131 |
| 4 | 24h / 800,000 | 5,812 | 6 | 4 | 1,376 | 5.014 / 3.227 |
| 4 | 24h / 1,000 | 1,000 | 2 | 1 | 928 | 0.525 / 0.478 |
| 4 | 24h / 0 | 0 | 0 | 0 | 512 | 0.000083 / 0.000083 |
| 8 | 1h / 1,600,000 | 500 | 3 | 8 | 1,888 | 0.277 / 0.265 |
| 8 | 24h / 1,600,000 | 11,616 | 13 | 8 | 2,560 | 6.505 / 6.578 |
| 8 | 24h / 1,000 | 1,000 | 2 | 1 | 1,440 | 0.483 / 0.472 |
| 8 | 24h / 0 | 0 | 0 | 0 | 1,024 | 0.000125 / 0.000125 |

All nonzero-budget rows retain one earliest candidate; limited searches remain
incomplete and cannot establish a global next arrival. Zero allowance still
allocates a report for every unsearched satellite. Full 24h-row initialization
medians were 21.750/13.458 µs (4 entries) and 20.625/28.000 µs (8 entries).
Host timings vary substantially; sample maxima are not worst-case bounds.

## Reproduction/build context

Apple M1 Pro, aarch64 macOS, Darwin 25.6.0. `rustc 1.99.0
(b940084d7 2026-09-28)`, LLVM 23.1.1. Cargo release defaults (`opt-level=3`, no LTO
override), debug assertions off, 64-bit pointers. Existing `.cargo/config.toml`
adds `-L /opt/homebrew/lib` on this target. No `RUSTFLAGS`, encoded flags,
or release opt-level/LTO environment overrides. Base revision
`86e9d37c6580fe8700e414d6204e0d5739c7dfb4` plus this benchmark change; hashes
identify measured sources rather than claiming the base revision contains them.

```sh
cargo run --release --locked -p overhead-tools --bin overhead-catalogue-benchmark -- \
  --samples 5 --label 'record CPU, OS, rustc, revision/profile/flags here' \
  > /tmp/catalogue-costs.json
# Repeat the built executable in another process for a second sample set.
```

SHA-256:

| Artifact | Hash |
|---|---|
| `Cargo.lock` (unchanged) | `58f907720e3e528a3881fb3c884150111e41e843be6ef309123af44150beaa47` |
| `tools/src/bin/overhead-catalogue-benchmark.rs` | `4fcfe811fad5ccdaae8314215ae5ce685190709ebb5945d3311cad782d3f6b19` |
| `tools/src/bin/catalogue_benchmark/memory.rs` | `caeda5f28fcca59e25d4a2942345fad75bc5c982559b5c30e484b838ff734768` |
| Local measured release executable (not archived) | `d21358e65e85f7af995dd7d51d4e059dc7564eeba37668cc7d06dd1ad4a61dd1` |
| `catalogue-costs-run-1.json` | `9a9224c2876db86f5adad743da5eee69191c41aff731deae92c64976c4f986bd` |
| `catalogue-costs-run-2.json` | `60f75e762ac72954cc563b0c1afa11423e59fc406cc301cb182de3600b9856b1` |

## Remaining limits / next slice

No stack high-water mark, static RAM, RSS, physical allocator footprint, PSRAM,
S3 catalogue measurements, supported size, cadence, scheduler, or over-budget
policy has been established. Firmware currently has no allocator or catalogue
feature; reproducing this path on S3 requires an explicit memory/instrumentation
plan, not multiplying these host bytes by a pointer-width ratio. Ask before
flashing or changing toolchains.

Next: choose a bounded target catalogue/allocator and stack-high-water experiment,
including input/storage lifetimes. Broaden distinct orbital populations and
observer/pass-density cases, duplicates/diagnostics and larger collection growth;
add independent target numeric checks before selecting operating limits. Eight
historical objects are neither a curated product catalogue nor a worst-case
population. Sharp output remains a separate early hardware check.

Verification: workspace check/tests (150 tests + 6 doctests), targeted strict
Clippy, workspace fmt, source-line comparison with pinned upstream, and two
release runs pass. New tests exercise allocator zeroing/growth/shrink/release,
checked ingestion/conversion, CLI errors, all eight workloads, incomplete and
unsearched reports, stable heap/work accounting, and phase peak arithmetic.
Firmware and hardware were untouched; their checks/captures were not rerun.
