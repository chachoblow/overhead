# Host tools

## `overhead-catalogue`

Load local OMM groups and report a nonempty, deduplicated catalogue. No network,
wall clock, observer, propagation, or freshness cutoff is implicit.

```sh
cargo run -p overhead-tools --bin overhead-catalogue -- tools/examples/catalogue.json
```

The checked-in example references the existing historical ISS fixture without
changing it; it is a smoke test, not live data or a chosen production group set.
Fixture provenance remains in the [fixture README](../core/tests/fixtures/README.md).

### Manifest

```json
{
  "groups": [
    {
      "name": "stations",
      "path": "stations.json",
      "source_url": "https://celestrak.org/NORAD/elements/gp.php?GROUP=stations&FORMAT=JSON",
      "fetched_at": "2026-10-06T08:00:00Z"
    },
    {
      "name": "weather",
      "path": "weather.json"
    }
  ]
}
```

Save each group as a local OMM JSON array. Paths resolve relative to the manifest;
absolute paths also work. Configure at least one group, using unique nonblank
names and nonempty paths. Unknown configuration fields are rejected.

`source_url` and `fetched_at` are optional (absent/null means unknown). Source
URLs are descriptive strings, never fetched. Fetch timestamps use explicit UTC
`YYYY-MM-DDTHH:MM:SS[.fraction]Z`, 1957–2100, without leap seconds or offsets.
They are not inferred from file metadata or used to select elements.

### Report and failures

Reports are sorted by NORAD ID and include name, group memberships, selected
record location (one-based record number), element epoch, source URL, and fetch
time. Only groups contributing valid records establish membership. Equivalent
newest records use the first manifest group/record for display metadata and
provenance; orbital disagreement is never resolved by that ordering.

The [catalogue decision](../docs/decisions/0011-catalogue-ingestion-and-provenance.md)
owns merge and failure policies. Invalid records and unresolved newest-epoch
conflicts produce stderr warnings; a nonempty accepted result still exits 0.
Unreadable files, malformed documents/configuration, or an empty result exit 1
with an explanatory stderr error and no partial stdout report. Help exits 0.
Empty-result errors retain record/conflict diagnostics.

Whole-document syntax is checked before records are interpreted. Raw record
JSON preserves duplicate keys for checked OMM ingestion; converting through a
JSON object map first could silently discard them. Successful ingestion means
validation and SGP4 initialization, not guaranteed propagation or accuracy.

## `overhead-passes`

Search the accepted local catalogue for physical passes at an explicit observer
and UTC. Uses the same manifest, atomic loading, provenance, and ingestion
warnings as `overhead-catalogue`; no network, wall clock, or freshness cutoff.

```sh
cargo run -p overhead-tools --bin overhead-passes -- \
  tools/examples/catalogue.json 2026-10-04T12:43:41.833056Z \
  39.007 -104.883 2.187 60 3000 3000
```

This historical-fixture example uses **illustrative**, not measured operating
defaults, for the detection interval and allowances. Do not use it as live data.

```text
MANIFEST.json UTC LAT_DEG LON_DEG HEIGHT_KM DETECTION_S SAT_LIMIT TOTAL_LIMIT
  [LOOK_AHEAD_S MIN_ELEVATION_DEG TOLERANCE_S]
```

- UTC and observer conventions match `overhead-track` below.
- Durations are positive integer seconds. Detection interval is required and
  independent of crossing tolerance; no detection default is selected.
- Both evaluation allowances are required unsigned integers; zero is valid.
  Counts include failed evaluations and refinement, not denied calls. They are
  not wall-time, memory, or scheduling budgets.
- The optional tuning trio must be supplied together; otherwise it uses the
  accepted 24-hour look-ahead, 10° threshold, and 5-second crossing tolerance.

### Results and incomplete work

The report includes the catalogue/provenance, observer, search window, tuning,
allowances, aggregate completion, evaluation counts, and every satellite's pass
records/status. Stop details retain UTC, sampling/refinement phase, and cause;
progress is the last successful regular sample. Traversal is ascending NORAD ID,
not a fairness or background scheduling policy.

Arrival times remain brackets, including whether tolerance was met. Earliest
candidates follow [0016](../docs/decisions/0016-catalogue-pass-aggregation.md):
overlapping/touching timing uncertainty is not resolved by midpoint or NORAD ID.
Candidates are displayed by NORAD ID/pass number, not claimed arrival order.
In-progress and equality-boundary starts are not upcoming arrivals. Unknown ends
distinguish completed-window limits, boundary equality, and interruptions.

Incomplete searches retain usable/coarse records and identify every partial or
unsearched satellite. They cannot establish a catalogue-wide next arrival or a
complete no-pass result. Completion covers only the **accepted** catalogue and
configured procedure: ingestion warnings identify omitted records, and brief
excursions/gaps or extra crossings inside brackets can still be missed. Printed
brackets are model timing, not a guarantee of real-world orbit accuracy.

| Exit | Meaning |
|---|---|
| 0 | Complete search (subject to detection limits), or help |
| 2 | Incomplete search; **stdout contains retained results and stop details** |
| 1 | Invalid inputs or catalogue-load failure; no partial stdout report |

## `overhead-evaluate-intervals`

Run a fixed, offline detection-interval experiment over historical LEO, resonant
HEO, GNSS, and GEO inputs plus constructed short excursions/gaps and a no-pass
graze control. This is **same-model integration/detection evidence**, not an
independent timing oracle, a coverage guarantee, or a selected operating default.

```sh
cargo run --release -p overhead-tools --bin overhead-evaluate-intervals \
  > /tmp/overhead-intervals.json
```

No arguments (other than `--help`/`-h`), file loading, network, clock, or hardware.
Inputs are embedded; run from any working directory after building. JSON contains
case/observer/epoch/threshold metadata, fine-grid stability comparisons, event
bounds, and 630 search rows: explicit intervals/phases/tolerances, evaluation
counts, matched/missed/unmatched/ambiguous crossings, and missed excursions/gaps.
Evaluation counts are not device cost measurements. The 200,000-call per-search
guard is an experimental safety allowance, not an operating budget.

Exit 0 means the experiment finished, **including reported detection misses**;
exit 1 means invalid arguments, failed/incomplete evaluation, or reference-grid
instability, with stderr and no partial stdout JSON. Finite reference-grid
agreement cannot prove completeness. Precise floating-point JSON is not a
cross-platform bitwise contract.

[Method and results](../docs/evaluations/detection-intervals.md) own the experiment
interpretation; [fixture provenance](fixtures/README.md) owns the historical
inputs. `cargo test -p overhead-tools` includes the complete suite plus matching,
equality, uncertainty, short-event/phase, tolerance-independence, and CLI tests.

## `overhead-benchmark`

Measure offline **host kernel throughput** using the same embedded historical
LEO, resonant HEO, GNSS, and GEO inputs as the interval experiment. No dependencies,
network, input files, or hardware are required beyond the existing workspace.

```sh
cargo run --release -p overhead-tools --bin overhead-benchmark -- \
  --samples 5 --min-sample-ms 20 --label 'record CPU, OS, rustc, build/revision here' \
  > /tmp/overhead-benchmark.json
```

Options are optional, order-independent, and cannot repeat. `--samples` accepts
1–50 (default 5); `--min-sample-ms` accepts 1–1000 (default 20); `--label` is an
unverified descriptive string. These are measurement settings, not device policy.
`--help`/`-h` exits 0 without running. Unknown/missing/invalid options exit 1.

The 62 fixed workloads cover propagation alone, propagation plus ECEF/observer
geometry, 1h/24h pass searches at 5/30/60-second detection intervals and independent
250ms/5s crossing tolerances, and equally mixed 4/16/64-slot tracking/search batches.
Slots repeat four fixtures at their **own epoch-relative times**, not distinct
catalogue entries at a common UTC. They measure compute scaling, not supported size.

Timing uses `std::time::Instant`: one untimed warm-up, discarded doubling-batch
calibration to the minimum target, then fixed-size samples. The target is not a
maximum runtime or a promise every subsequent sample lasts that long. JSON retains
raw elapsed nanoseconds, batch iteration counts, min/median/max per invocation,
work counts, inputs/configurations, target OS/architecture, debug-assertion status,
and label. `black_box` consumes inputs/outputs; work counts must remain stable.
There are no outlier exclusions or machine-dependent speed assertions in tests.

Use release builds for measurements; debug builds are allowed for test coverage.
Record compiler version, CPU, OS, revision, Cargo.lock, profile and flags alongside
results: debug-assertion status alone cannot identify optimization settings.
Timing includes kernel validation, loops, error checks, budgets, and streamed pass
counting; it excludes ingestion/initialization, prepared timestamps/configurations,
JSON, pass storage/catalogue aggregation, UI, and network. No peak-memory measure,
worst-case latency bound, or scheduler is supplied.

All searches must complete under the experimental 200,000-evaluation per-slot
guard. Any failure, incomplete search, inconsistent work count, or calibration
failure exits 1 with stderr and no partial stdout JSON. Success exits 0. Timing
values are inherently variable; workloads/counts are reproducible on this host.

[Method, measured results, and device follow-up](../docs/evaluations/host-costs.md).
[Fixture provenance](fixtures/README.md). Tests cover the complete matrix, mixed
accounting, batch normalization, workload failures, and CLI validation.

## `overhead-catalogue-benchmark`

Measure offline **distinct-catalogue host initialization, retained pass aggregation,
and requested heap**. Eight embedded historical satellites, 4/8-entry subsets,
one common UTC. No files, network, hardware, new dependencies, or implicit clock.

```sh
cargo run --release --locked -p overhead-tools --bin overhead-catalogue-benchmark -- \
  --samples 5 --label 'record CPU, OS, rustc, revision/profile/flags here' \
  > /tmp/overhead-catalogue-costs.json
```

Optional order-independent options cannot repeat: `--samples` is 1–50 (default
5), `--label` is unverified descriptive context. `--help`/`-h` exits 0. Invalid
options or failed validation exit 1 with stderr and no partial stdout JSON.
Successful experiments exit 0 **including deliberately incomplete budget cases**.

Eight rows: each size × complete 1h/24h, plus 24h with 1,000/zero shared
allowance. Fixed UTC/observer/tuning and input identities/epochs/ages are in JSON.
These are experimental settings, not defaults. No duplicated/fabricated IDs.
One discarded warm-up, then one invocation per sample (no calibration/batching).
Two phases retain separate elapsed nanoseconds and requested-heap accounting:
checked OMM array ingestion/SGP4 initialization/merge; catalogue pass search plus
earliest-candidate collection. Catalogue remains live during aggregation.
Untimed streaming searches validate every retained report/pass, and work/heap
counts must match warm-up. All measured objects must release their allocations.

The binary's single-threaded counting allocator reports logical requested-byte
peaks, retained bytes, successful allocation/reallocation calls, and requested
traffic. It does not measure allocator overhead/fragmentation, internal realloc
transients, stack, RSS, static memory, or S3 RAM. Timing includes instrumentation.
Prepared input JSON (length reported), TLE-to-OMM conversion, configs, I/O,
manifest/provenance reporting, validation, destruction, and JSON output are
excluded. Existing Serde float parsing is not a bitwise TLE-to-OMM round-trip
contract; tests bound conversion rounding without changing dependency features.

[Method, evidence, limitations, and target follow-up](../docs/evaluations/catalogue-costs.md).
[Fixture provenance](fixtures/README.md#catalogue-coststle). Tests cover allocation
hooks, conversion/ingestion, CLI validation, complete/partial/unsearched work,
stable heap accounting, and phase arithmetic—not machine-dependent speed.

## `overhead-m2-pool-preflight`

Verify the separately pinned 16-object M2 acquisition pool and publish complete
host search counts, without changing historical controls or fetching data.

```sh
python3 tools/prepare_m2_pool.py --check
cargo run --release --locked -p overhead-tools --bin overhead-m2-pool-preflight \
  > /tmp/m2-pool-suitability.json
cmp /tmp/m2-pool-suitability.json docs/evaluations/m2-pool-suitability.json
```

No arguments except `--help`/`-h`; inputs/UTC are embedded. Exit 0 means the source
suitability gate passed (or help); invalid arguments or failed gates exit 1 with
stderr and no partial stdout JSON. No files, wall clock, network or hardware are
used at runtime. The Python integrity/regeneration tool is also entirely offline;
its standard library is sufficient. No dependencies were added.

The gate checks all 16 identities/epochs/order, actual initialized sgp4 2.4
method/resonance variants, rejection-free ingestion, and complete 24h searches.
JSON contains four Colorado growth searches and all 30 density-site candidates,
per-satellite counts, deterministic high/low site choices and the densest selected
case. Every retained report/pass is cross-checked against a direct streaming search.
The 34 suitability searches are not an expanded 34-case target matrix.

**Not the full M2 preflight or capture manifest.** No allocations/capacities, RAM
lifetimes, stack, timing or independent numerical checks are measured. Historical
reference/error/partial/RAM cases remain to be preflighted. No supported size or
production policy is selected. Source integrity is verified by the Python check;
the Rust binary embeds the selected pool and publishes its recorded hash, rather
than implementing another hash function. Run both checks for evidence regeneration.

[Fixture provenance/regeneration](fixtures/m2-pool/README.md),
[results and remaining gate](../docs/evaluations/m2-pool-suitability.md).
Tests: `cargo test -p overhead-tools --bin overhead-m2-pool-preflight --test m2_pool_preflight`
and `python3 -m unittest discover -s tools -p 'test_prepare_m2_pool.py'`.

## `overhead-track`

Single-satellite measurements from explicit local inputs; no network, wall
clock, implicit location, or UI. From the workspace root:

```sh
cargo run -p overhead-tools --bin overhead-track -- \
  core/tests/fixtures/iss-25544.json 2026-10-04T12:43:41.833056Z \
  39.007 -104.883 2.187
```

| Argument | Contract |
|---|---|
| OMM JSON file | Array of **exactly one** satellite |
| UTC | `YYYY-MM-DDTHH:MM:SS[.fraction]Z`; 1957–2100; no leap seconds or offsets |
| Latitude | Degrees, [-90, 90] |
| Longitude | East-positive degrees, [-180, 180] |
| Height | km above WGS-84 ellipsoid, **not MSL**; may be negative |

Coordinates must be finite. OMM metadata is checked before numeric/epoch
validation ([0010](../docs/decisions/0010-propagation-and-omm-validation.md)).
Use `--help` / `-h` for usage; the default `overhead-tools` binary is a placeholder.

### Report
Includes identity, element epoch, requested time, signed elapsed minutes,
observer coordinates, TEME position/velocity, ECEF position, geodetic position,
and range/azimuth/elevation. Units are explicit. Vertical azimuth prints
`undefined`; negative elevation is valid geometry, not a visibility decision.

Success/help exit 0. Errors exit 1 with stderr and no partial stdout report.
Velocity is **TEME**, not Earth-fixed. Printed precision is not orbit accuracy;
there is no freshness cutoff. Do not use the fixed historical fixture as live
orbit data. Physical conventions: [0006](../docs/decisions/0006-sgp4-crate-and-conventions.md).

The example above reports approximately:

```text
Satellite WGS-84 ellipsoidal altitude (km): 424.668829114
Slant range (km): 4825.772604378
Azimuth (deg clockwise from true north): 150.234162834
Elevation (deg): -16.803612339
```

These are runner outputs, not independent references. For T0/T1/T2 timestamps,
reference sites, and provenance, see the
[fixture README](../core/tests/fixtures/README.md); do not refresh the ISS file.

### Verification
```sh
cargo test -p overhead-tools
```

Tracking tests launch the executable for all 12 Skyfield cases (range within
0.1 km, angles within 0.01°), deterministic output, and invalid-input/error paths.
Catalogue tests cover manifests, relative paths, provenance, merge/conflict
behavior, raw-key preservation, invalid records, and whole-load failure.
Pass-reporting tests cover deterministic ambiguous arrivals, configurable tuning,
no-arrival/in-progress distinctions, coarse refinement brackets, orbital failures,
local/shared allowances, unsearched satellites, exit codes, and input/load errors.
Shared-core tests additionally cover interval ordering and retained typed reports.
Synthetic record mutations test catalogue/aggregation policy, not physical accuracy.
Tests need no Python, network, SDL window, or hardware. Workspace commands are
in [AGENTS.md](../AGENTS.md).
