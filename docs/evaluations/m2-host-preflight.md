# M2 candidate host preflight — ingestion, work, and requested memory

2026-10-10 UTC. Host-only continuation of [source suitability](m2-pool-suitability.md)
under [0019](../decisions/0019-bounded-m2-evidence-preparation.md).
**Historical candidate-stage report:** adopted unchanged by the 2026-10-11
[frozen annex](m2-frozen-manifest.md), which closes numerical/representation
preparation. The original candidate output/schema remains unchanged. No target harness, firmware, toolchain, dependency, hardware, or
production-default change. The fixed 64 KiB target heap is unchanged.

## Inputs and retained outcomes

The [fixture README](../../tools/fixtures/m2-cases/README.md) owns exact document
bytes, hashes, input order/epochs, mutations, provenance, and regeneration.
[m2-host-preflight.json](m2-host-preflight.json) publishes all 20 candidate cases:
UTC/site/window/budgets/lifetime, per-group byte/record/hash data, accepted sorted
IDs/epochs/memberships/origins, diagnostics, per-satellite work/status, capacities,
and requested-memory observations. No case exceeds the contract ceilings.
Malformed group 1 has no parseable record count; its one byte is still counted.

The runner validates each whole document into `Vec<&RawValue>` before parsing
checked `OmmElements`, preserving duplicate keys. Caller diagnostics are retained
as `Vec<ParseError>` (zero-based `RecordOrigin` plus owned `String`); core diagnostics
remain the original `CatalogueDiagnostic` vector, including conflict-origin vectors.
A failed document retains its group index/message and any prior parse diagnostics,
but drops the builder and never calls aggregation. This is separate from the old
firmware helper that aborts on a checked record error. CLI formatting, input
preparation, hashes, and external group/source metadata are outside measurement.

| Cases | Verified outcome |
|---|---|
| 01–02 | Historical eight-object 1h/24h controls: 500/11,616 evaluations, 3/13 stored passes; unchanged historical input/UTC |
| 03–10 | Initial suitability's complete growth/density populations: unchanged pass/work counts and sites |
| 11 | Case 07 with allowance 1,000: NORAD 7376 incomplete in sampling at 16:28 UTC, two retained passes; nine unsearched reports |
| 12 | Case 07 with zero allowance: ten unsearched reports, no evaluations/passes |
| 13 | Four equivalent groups: eight survivors, all four memberships, no diagnostics, same search as 02 |
| 14 | NORAD 6251 omitted, seven survivors; one newest-epoch conflict with origins `(0,0)` and `(1,1)`, no older fallback |
| 15 | One checked OMM rejection `(1,0)` for MARS, one core `EccentricityOutOfRange(1.0)` rejection `(1,1)`; eight original survivors, no rejected membership, same search as 02 |
| 16 | Malformed group 1 fails the whole load; no catalogue/search published; retained error only, then full release |
| 17–20 | Cases 02/07 with copied RAM input, dropped after initialization or retained through aggregation; catalogue/diagnostics/all search output identical to flash controls |

Every search report and pass is cross-checked against direct streaming searches.
RAM and equivalent/invalid variants are also compared to their controls using full
stored reports/passes, not only totals. This is same-model consistency, **not**
independent numerical evidence or proof of detection of every physical pass.

## Collection observations and case contributions

A bounded, nonallocating allocator event ledger observes the live requested block
backing each exposed whole-Vec slice, divided by element size. It does not cast
private fields, dereference allocation pointers, infer capacity from allocator
rounding, or change core APIs. Missing allocations or ledger overflow fail closed.
Parser-array capacity is read directly; snapshots around every record preserve
allocation-call/request totals and live bytes. These are installed-toolchain
observations, not stable Rust container promises or target capacities.

| Case | Accepted | Parser array capacity | Catalogue entries capacity | Satellite-report capacity | Distinct contribution |
|---|---:|---:|---:|---:|---|
| 03 | 5 | 8 | 8 | 5 | Small expanded mixed population; baseline spare entry capacity |
| 04 | 9 | 16 | 16 | 9 | Larger parser and accepted-entry backing storage than 03 |
| 05 | 12 | 16 | 16 | 12 | Staging allocation jump at the 12th distinct record: 10 calls/20,565 requested bytes versus 8 calls/~3,900 on preceding records |
| 06 | 16 | 16 | 16 | 16 | Full distinct pool/experimental population ceiling, more reports and passes; not another claimed catalogue Vec boundary |

The 12th-record jump accompanies BTreeMap staging growth, not another final-entry
Vec capacity change. Do not treat every case size as a Vec boundary. Case 06's
contribution is the full allowed distinct population, not a new tree boundary.
Density cases hold population size fixed while varying retained passes (68 vs 26
for LEO-heavy, 28 vs 11 for deep-space-heavy). Partial cases retain incomplete/
unsearched metadata; diagnostic and RAM cases isolate their named storage changes.
No replacement sizes, extra combinations, or case omissions are introduced.

## Requested-memory method and limits

Two measured executions per case must repeat every normalized memory observation.
The process is single-threaded. A separate requested-byte meter uses default
`GlobalAlloc` allocate-copy-free reallocation, so **both old and new blocks overlap**
in phase and independently maintained pipeline peaks, including shrinking realloc.
The event ledger itself is fixed instrumentation storage, not measured dynamic
application memory. It is host-only and must not be transplanted into the device.

Pipeline counters start **before** RAM copy. Phase boundaries are input copy,
initialization, input release, aggregation (including retained arrival candidates),
and destruction. All reported occupancies use one common process-baseline offset;
aggregation includes retained catalogue, diagnostics, and any retained input. Phase
resets never reset the independent pipeline peak. Peaks are not summed. Counters
are cumulative within the pipeline; differences in each phase give its allocation
calls/requested-byte totals. No elapsed timings are reported in this host gate.

The borrowed-input control models flash lifetime on host; prepared source text and
an untimed reference catalogue/search are outside the baseline. It does not claim
host strings physically reside in flash or measure live refresh overlap. RAM uses
owned `String` buffers with a fixed four-slot table. Dropped buffers are released
only after initialization. Catalogue, owned parse/core diagnostics, reports, passes,
and arrival candidates remain live through their endpoint, then destruction must
restore baseline with zero allocation failures, including document-failure case 16.

| Case | Requested pipeline peak bytes | Requested retained at aggregation endpoint |
|---:|---:|---:|
| 02, historical flash control | 17,127 | 8,487 |
| 06, full mixed pool | 34,377 | 17,705 |
| 07, densest flash control | 25,725 | 17,661 |
| 17, historical RAM dropped | 20,458 | 8,487 |
| 18, historical RAM retained | 20,458 | 11,818 |
| 19, densest RAM dropped | 29,937 | 17,661 |
| 20, densest RAM retained | 29,937 | 21,873 |

RAM initialization peaks include exactly the extra 3,331/4,212 input bytes. The
retained variants keep those bytes at aggregation; dropped variants return to the
flash control's endpoint. In this matrix initialization dominates the whole-pipeline
peak, so dropped/retained variants have equal peaks but different live endpoints.
This is a measured distinction, not grounds to replace either lifetime experiment.
Case 16 retains 48 requested bytes for its error message after releasing staging;
final destruction restores zero pipeline-relative occupancy for **all 20 cases**.

Requested bytes are **not backend occupancy**, allocator overhead, static RAM,
stack, RSS, or S3 fit. No comparison to the reserved 64 KiB heap establishes target
capacity; the target's pointer widths, allocator, math library and toolchain differ.
The future harness still needs absolute requested/backend peaks, separate phase
timings, independent pipeline counters and full release, plus the required numerical
and two-pattern stack captures. No safe stack/catalogue size is established here.

## Verification and remaining preparation

Generated on Darwin arm64, rustc `1.99.0 (b940084d7 2026-09-28)`, base revision
`6162c3a7b55bdd8d505a3ee80a21635f71f8c9b2` plus this preparation change. Debug and
release reports match byte-for-byte. Offline workspace check/default/all-feature
tests, no_std core check, strict all-target/all-feature Clippy, formatting, and
eight pool/case Python tests pass. CLI integration tests run measurement in an
isolated subprocess, check exact published evidence on this host, and exercise
working-directory independence. No ignored firmware artifacts are prerequisites.
[Reproduction](../../tools/fixtures/m2-cases/README.md#offline-regeneration).

The following were the remaining gates at publication; steps 1–2 are now closed
by the [frozen annex](m2-frozen-manifest.md). Step 3 remains next:


1. Pin the 12 original TEME references/provenance and the complete existing
   coordinate/observer selection with unchanged tolerances; prepare the identical
   host/target numerical checks. Work-count agreement is not a substitute.
2. Review this candidate annex against the full contract, settle the exact target
   metadata/diagnostic representations and fixture consumption, then publish the
   frozen ≤20-case manifest with numerical references. Changes require re-preflight.
3. Only after freeze implement target timing/backend/release/stack/numerical capture
   and validation, derive timeouts from work and prior S3 costs, and gate exact normal
   images before requesting flash approval. Scheduling latency remains separate.
