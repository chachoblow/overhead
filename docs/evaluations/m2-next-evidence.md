# M2 next evidence — bounded preparation contract

Preparation scope accepted 2026-10-10 UTC in [0019](../decisions/0019-bounded-m2-evidence-preparation.md).
**The separate snapshot/source-suitability gate and 20 candidate host ingestion/
work/requested-memory cases are prepared; numerical preparation, final contract
review, exact capture manifest and expanded target suite are not.**
[Source suitability](m2-pool-suitability.md),
[candidate host evidence/remaining gates](m2-host-preflight.md).
This contract freezes scope, not a capture-ready matrix, production policy, or
supported catalogue size. [PLAN](../PLAN.md) owns status;
decisions 0011–0018 remain authoritative for existing behavior and evidence.

The goal is an initial supported catalogue and prediction operating budget, not
an unbounded benchmark. Keep the installed toolchain, 65,536-byte internal heap,
and single CPU; no PSRAM/display/network on target. Ask before flashing. The
[DROM remedy and corrected boots](s3-drom-diagnostic.md#corrected-normal-build-target-verification)
pass for the existing normal build; re-gate every future exact normal ELF/image.
The separate RWX warning remains open.

## Preparation gate: freeze inputs before extending the target harness

Preserve the existing eight-object reference, its bytes and UTC unchanged. Prepare
a **separate pinned acquisition set** for expanded populations: eight distinct
near-Earth LEOs and eight distinct deep-space objects (two resonant HEO, two
nonresonant GNSS, four resonant GEO). Source labels alone do not establish SGP4
path coverage; verify it during preparation. No renamed or epoch-shifted orbits
count as independent populations. The existing Vallado verification inputs remain
numerical references, not a source to pad this pool with decaying/error records.

Archive source bytes/URLs, retrieval UTC, hashes, identities, epochs and selection
rules in the fixture README. Pick one explicit UTC for the expanded pool, with all
16 source epochs within 48 hours of it. All complete expanded cases must initialize
and finish the configured searches on host without record rejection or propagation
failure. The historical reference keeps `2006-06-26T00:00:00Z`; do not describe the
two cohorts as sharing one UTC or attribute differences solely to catalogue size.
Snapshot acquisition is preparation only; checked-in fixtures and subsequent
builds/tests/captures must work offline. Do not refresh existing fixtures.

Order LEOs by NORAD ID. Order deep-space objects by alternating the lowest-ID HEO,
GNSS and GEO, then the second of each, then the remaining GEOs in ID order. Alternate
LEO/deep-space records to form a nested 16-object mixed pool; its first 5/9/12/16
records define the growth populations. Density populations each contain ten objects:
all eight LEOs plus the first two deep-space objects, or all eight deep-space objects
plus the first two LEOs. Ingestion order is not search order: 0016 still requires
ascending NORAD traversal.

Each case is bounded to **4 groups, 32 input records, 32,768 aggregate JSON bytes,
and 16 accepted satellites**. These are experimental ceilings, not ingestion
limits implemented in core or evidence of target fit. Count duplicates/rejections
and malformed-document bytes, not just accepted records. Freeze actual counts,
per-group bytes, accepted identities and input hashes in the manifest. If source
coverage, caps or preflight cannot be satisfied, stop and review the contract;
do not silently relax it, enlarge the heap, or add combinations.

## At most 20 memory/work cases

Retain 10° threshold, 60s detection, 5s crossing tolerance and per-satellite
allowance 200,000 as experimental controls, not defaults. Complete shared allowance
is 200,000 × accepted satellites. Growth/reference/error cases use Colorado
(39.007°, -104.883°, 2.187 km); density and derived cases use their selected site.
Inputs are flash-resident unless explicitly marked RAM.

| IDs | Cases | Inputs and purpose |
|---|---:|---|
| 01–02 | 2 | Historical eight-object reference, 1h and 24h; bridge to prior evidence |
| 03–06 | 4 | New nested mixed populations of 5, 9, 12 and 16, 24h; collection growth |
| 07–10 | 4 | Ten-object LEO-heavy/deep-space-heavy populations, each at its high/low-density site, 24h |
| 11–12 | 2 | Densest expanded complete case, shared allowances 1,000 and zero; partial/unsearched retention |
| 13–16 | 4 | Historical reference variants: equivalent duplicates, newest-epoch conflict, invalid records, malformed document |
| 17–20 | 4 | Reference case 02 and densest expanded case, each with RAM input dropped after initialization versus retained through aggregation |

Choose density sites from the 15-point grid: latitudes -60/-30/0/30/60°, longitudes
-120/0/120°, altitude zero. For each ten-object population, run complete 24h searches
at every candidate. Choose the maximum-pass site, ties by ascending (latitude,
longitude); then the minimum-pass site among the remaining sites, same tie-break.
Publish every candidate's pass/work counts before target capture. “Densest” means
most stored passes among cases 03–10, ties by lowest case ID; it does not assert
maximum heap or worst-case orbital cost.

Instrument host collection capacities and allocation changes rather than assuming
5/9/12/16 cross particular Vec/tree boundaries. Explain each sample's distinct
contribution. Redundant cases may be omitted; replacement sizes require a documented
contract revision before manifest freeze. Never exceed 20 cases by retaining old
and replacement samples together. The historical eight-case suite stays historical,
not eight additional cases in every expanded capture.

### Ingestion variants: observable outcomes, not manufactured orbit evidence

Use historical case 02 as the base, with explicitly labeled synthetic mutations:

- **13, equivalents:** all eight original records in each of four groups: 32
  records, eight accepted identities, all four memberships, no rejection/conflict.
- **14, conflict:** original eight in group 0; in group 1 an older copy of NORAD
  6251 (epoch minus one day) and a copy at its original epoch with mean anomaly
  increased by 1° modulo 360. Omit 6251 for newest-epoch disagreement, with no
  older fallback; seven survivors, one conflict with exact newest-record origins.
- **15, invalid records:** original eight in group 0; group 1 contains two copies
  of 6251, one with `CENTER_NAME=MARS`, one with eccentricity 1. Require one checked
  OMM rejection and one core orbital rejection, original eight survivors, and no
  membership from rejected records. Confirm the rejection stages on host.
- **16, malformed document:** valid group 0 followed by group 1 containing `[`. Fail
  the entire load, publish no catalogue, perform no aggregation, and release all
  staging storage. A missing aggregation phase is valid only for this named case.

Preserve 0011's distinction between invalid records and failed documents. The
existing firmware helper aborts on checked OMM errors and cannot be reused unchanged
for case 15. Retain caller-owned parse diagnostics and core diagnostics through the
measured endpoint; freeze their representations/counts/origins on host. Group/source
metadata may remain in flash but its storage/lifetime must be explicit; these cases
do not measure dynamically allocated URLs, CLI formatting, or network ingestion.

## Memory and timing boundaries

Measure RAM allocation/copy **after** starting pipeline counters and before parsing.
Dropped-input cases release the buffer only after initialization; retained-input
cases keep it through aggregation. Both include its overlap with parsing/catalogue
storage. Report input-copy, initialization and aggregation timings separately;
report absolute requested/backend peaks and an independently maintained whole-pipeline
peak. Never sum phase peaks or reset away an input-allocation peak. Both RAM variants
must produce the same accepted catalogue, diagnostics and search output as their
flash control.

Keep catalogue, diagnostics, reports, passes and arrival candidates alive at the
relevant endpoint; then require full destruction to restore requested and backend
occupancy baselines, including expected document-failure paths. Host preflight must
account for allocate-copy-free overlap and retained input, but host fit is not proof
of target fit. The 64 KiB reserved heap already belongs to `.bss`; occupied heap is
not additional static RAM. UART, validation and fixture conversion stay outside
phase timers. Panic/OOM, unexpected rejection/work or unreleased storage invalidates
a run, never establishes graceful product overload behavior.

## Separate numerical and stack gates

- Prepare 12 published TEME state checks: NORAD 00005, 08195, 24208 and 28129 at
  epoch/+360/+1440 minutes, spanning near-Earth, resonant HEO/GEO and nonresonant
  paths. Pin the original reference vectors and provenance before execution; do
  not generate expectations with Overhead. Preserve 0006's per-axis strict
  tolerances: position <1e-6 km, velocity <1e-9 km/s.
- Port all existing independent coordinate and observer fixture cases with their
  [existing tolerances/provenance](../../core/tests/fixtures/README.md). Host and
  target must run the same selected reference checks. No tolerance loosening to
  make the S3 pass. Work-count agreement is not independent numerical evidence;
  physical-pass detection evidence remains a separate gate.
- In separate diagnostic executions, observe input-copy, initialization,
  aggregation and destruction stack writes for every applicable case, repainting
  safely at phase boundaries. Use patterns `0xA5A5A5A5` and `0x5A5A5A5A`, each with
  the known-local smoke/guard checks. Take one observation per case/pattern in each
  of two reset-separated diagnostic captures; do not call these timed samples.
  Inspect compiled frame reservations/call paths for parsing and prediction;
  record unwritten-frame blind spots, indirect calls and interrupt limitations.
  Neither pattern agreement nor a watermark proves a safe/worst-case stack size.

## Manifest freeze and capture acceptance

Before target-harness implementation, publish the pinned fixtures and a host
preflight annex with exact IDs/order/epochs/UTC/sites, group/record/byte counts,
mutations, expected diagnostics/failure phases, work/completion counts, collection
observations and input lifetimes. Freeze numerical reference values/tolerances too.
Any subsequent change invalidates that manifest and requires renewed preflight.
The final frozen annex is **not yet available**. The [source-suitability subset](m2-pool-suitability.md)
and [candidate host annex](m2-host-preflight.md) cover source paths/searches, retained
error/partial outcomes, collection observations and requested input-memory overlap/
release. Numerical references and identical host/target check selection, final target
metadata/representation review, and manifest freeze remain required. Host requested
bytes do not substitute for future target backend occupancy, timings or stack gates.

Extend strict capture validation to that manifest, including retained/released
storage and all expected outcomes. Keep protocol versions distinguishable from
historical V1 evidence. For memory/work captures use one warm-up per case per boot,
three samples per case and two reset-separated captures: at most 120 measured
samples. Work and heap counters must repeat; timings and stack depths need not.
Numerical/stack diagnostic runs are separate and must all pass before accepting
the expanded evidence. Derive and record capture timeouts from frozen work counts
and prior S3 costs with explicit margin; do not assume host wall time predicts S3
time or reuse 900 seconds without justification. Preserve partial/failed logs.

Prepare/gate exact normal measurement and diagnostic images before requesting
flash approval. Archive source/input/manifest/build/image hashes, section inventories
and probe/frame inspection with captures. Stress and negative-control image fixtures
are never flash candidates. No approval in this contract authorizes flashing.

## Stop rule and decisions after capture

No additional matrix without a specific unanswered decision. Write an operating-
budget decision separating input bytes/records/groups from accepted satellites and
stored passes/diagnostics. Account for fixed RAM reservations and future application/
display headroom; this suite does not measure whole-device or network/refresh peaks.

Use short/long prediction costs and detection evidence to choose lookahead, accuracy,
recomputation cadence and explicit partial/over-budget behavior. Then scope the
smallest scheduling experiment for display responsiveness: synchronous search cannot
simply run in a 20 Hz loop. If resumable work is selected, verify bounded slice
latency and unchanged results before closing M2. No executor, second core, allocator
policy, cache architecture or production default is selected by this contract.
