# M2 frozen preparation manifest — bounded 20-case suite

2026-10-11 UTC. [0020](../decisions/0020-m2-capture-manifest-freeze.md) closes the
preparation gate under [0019](../decisions/0019-bounded-m2-evidence-preparation.md)
and the [capture contract](m2-next-evidence.md). **Preparation only: no expanded
target harness/capture, supported size, scheduling architecture or device policy.**

## Frozen annex and review

[`manifest.json`](../../tools/fixtures/m2-cases/manifest.json), ID
`m2-bounded-20-v1`, freezes all **20 cases in ID order**, without replacements,
omissions, new dimensions, source refreshes or altered historical UTCs. It pins:

- Exact per-group document bytes/hashes, record counts/order/epochs (through the
  [input archive](../../tools/fixtures/m2-cases/inputs.json)); case UTC/site/window,
  allowances, RAM lifetimes and control relationships.
- Accepted IDs/epochs/memberships/origins, retained parse/core diagnostics, document
  failure and per-satellite work/completion counts from the unchanged
  [host annex](m2-host-preflight.json). Case 16 alone has no catalogue/aggregation.
- Host collection/allocation observations by hashing that complete annex, **not**
  by treating host capacities/requested bytes as expected target occupancy.
- The [numerical sources/selection/tolerances](../../tools/fixtures/m2-numerical/README.md),
  portable checker/converter, relevant preflight/core sources and dependency locks.
- Two timed boots with one warmup and three samples per case (120 measured samples),
  plus separate numerical and two-pattern/two-boot stack diagnostic requirements.

Review found no need to revise the 0019 matrix. All cases obey 4 groups / 32 input
records / 32,768 aggregate JSON bytes / 16 accepted satellites. Duplicates count
in full; the malformed document contributes its byte, not a fictitious record.
The [density-site selection](m2-pool-suitability.md) and
[distinct contributions of growth/error/partial/RAM cases](m2-host-preflight.md)
remain unchanged. Case 07 is still the densest complete case. Historical controls
use `2006-06-26T00:00:00Z`; expanded cases use `2026-10-10T00:00:00Z`.

## Target representation and fixture consumption

This fixes the representation to be implemented after freeze; it does **not**
claim a target parser or instrumentation already exists.

| Storage | Required representation and lifetime |
|---|---|
| Case/group metadata | Static tables in flash: case ID/configuration, zero-based group index, existing `m2-{id}-group-{index}` name, document key and source-fixture references. No owned URLs/names, manifest JSON parser, hashes or CLI formatting in the measured pipeline. Metadata/static reservations still belong in the exact image inventory. |
| Input documents | Build host decodes each archived `text` string losslessly and verifies bytes/hash/counts. Embed each distinct document once as a flash `&str`; repeated groups borrow the same bytes but are parsed/count independently. Never regenerate target case JSON through generic numeric reserialization. |
| RAM variants | A fixed four-slot `[Option<String>; 4]` table; copy selected documents only after pipeline counters start. Drop only after initialization or retain through aggregation, as frozen per case. Count allocate-copy-free overlap; table/stack/static storage is not falsely reported as heap allocation. |
| Document parsing | Validate the whole document into `Vec<&RawValue>` before checked per-record `OmmElements` parsing. Preserve duplicate-key detection. Parse each group in order; borrowed parser-array storage is released after that group. No streaming replacement or pre-parsed `Elements` substitution for the measured pipeline. |
| Caller record errors | `Vec<ParseError>` with original `RecordOrigin { group: usize, record: usize }` and owned `String` from the parser error. Retain through aggregation/destruction. Exact frozen message/origin/count checked, not a preformatted static string or compressed error code. |
| Core errors/conflicts | Original `Vec<CatalogueDiagnostic>`, including owned conflict-origin vectors, retained with the catalogue. Keep invalid-record and document-failure paths distinct. Case 15 cannot use the historical firmware helper unchanged: it aborts on checked OMM errors. |
| Document failure | Failed group index plus owned parser-message `String`, retaining any prior caller errors. Drop builder/staging storage; publish no catalogue, do not aggregate. Case 16 is the sole expected failure and must fully release on destruction. Empty-catalogue failure remains unexpected and invalidates the run. |
| Catalogue/results | Original core catalogue, accepted element metadata (including names/designators), membership/provenance, reports and pass vectors; materialize and retain `earliest_candidates()` through the aggregation endpoint. No custom compact replacements or preallocation policy introduced. |
| Destruction | Release arrival candidates, reports/passes, catalogue/diagnostics and retained RAM input. Both independently metered requested and backend occupancy return to baseline, including document failure. |

These choices mirror the existing host preflight's measured storage. They do not
promise identical Rust sizes/capacities on 32-bit Xtensa. Group metadata is external
to dynamic measurements, but accepted element metadata is **not** stripped. The
host-only allocation ledger must not be transplanted to the S3. Use the fixed
65,536-byte internal heap, single CPU, no PSRAM/display/network. OOM or unexpected
rejection is a failed experiment, not graceful product overload behavior.

## Reproduction and verification

Base revision `7fef068` plus this preparation change; Darwin arm64, rustc
`1.99.0 (b940084d7 2026-09-28)`. No dependency/toolchain/source-snapshot refresh.
The unchanged candidate host report reproduces exactly in release; shared numerical
checks pass all 75 rows in debug/release and with/without `catalogue-memory`.

Start with the [existing offline gates](../../tools/fixtures/m2-cases/README.md#offline-regeneration),
then add:

```sh
python3 tools/prepare_m2_numerical.py --check
python3 tools/prepare_m2_manifest.py --check
python3 -m unittest discover -s tools -p 'test_prepare_m2*.py'
cargo +stable test --offline --locked --manifest-path firmware/Cargo.toml --lib \
  --features numerical-reference
cargo +stable test --offline --locked --manifest-path firmware/Cargo.toml --lib \
  --features numerical-reference,catalogue-memory
cargo +stable clippy --offline --locked --manifest-path firmware/Cargo.toml --lib \
  --tests --features numerical-reference,catalogue-memory -- -D warnings
```

The manifest check recomputes file hashes and joins group/input/report references;
it does not rerun Rust propagation or memory measurement. Run **both** the existing
preflight gates and manifest check before use. `--freeze` is explicit reconstruction
only after reviewed, renewed preflight; it is not a fix for a failed hash gate.
Target implementation has its own build/image hashes; changed pinned preparation
inputs/checker/semantics require renewed preflight and manifest review.

## Next block — implement and capture, not another preparation matrix

Implement the exact expanded measurement/diagnostic harness and strict capture
validation against this annex. Establish requested/backend whole-pipeline peaks,
phase timings, full release and collection observations, independent target
numerics and two-pattern per-phase stack observations. Derive capture timeouts
from frozen work counts and prior S3 costs, with explicit margin. Inspect compiled
frames and document watermark/interrupt/indirect-call blind spots.

Gate every exact **normal** image under [0018](../decisions/0018-source-level-drom-remedy-and-image-gate.md)
before asking for flash approval. No target image was built or flashed for this
freeze. Corrected historical boots do not gate a new image; RWX remains open.
After accepted evidence, choose conservative operating limits/cadence/over-budget
behavior and verify scheduling latency separately. Neither safe stack nor
whole-device RAM nor display responsiveness is established by preparation.
