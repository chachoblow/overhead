# Pinned M2 acquisition pool

A separate real-orbit source pool for [0019](../../../docs/decisions/0019-bounded-m2-evidence-preparation.md).
**Source suitability passes; this is not the frozen capture manifest.** Historical
ISS, interval, catalogue-cost and Vallado reference fixtures are unchanged.
[Scope/remaining gates](../../../docs/evaluations/m2-next-evidence.md),
[host suitability results](../../../docs/evaluations/m2-pool-suitability.md).

## Acquisition and integrity

CelesTrak GP JSON responses were downloaded once on 2026-10-10 UTC using Python's
standard-library `urllib.request.urlopen`; response bodies are archived verbatim
in `sources/`. [acquisition.json](acquisition.json) records exact URLs, retrieval
completion timestamps in UTC, byte lengths and SHA-256 hashes. These live URLs
are provenance, **not reproducible regeneration endpoints**. Do not refresh these
files; regeneration uses only the archived bytes. `.gitattributes` disables line
ending conversion for these response bodies, preserving their original CRLF.

| Archive | Query on `https://celestrak.org/NORAD/elements/gp.php` | Records | Bytes |
|---|---|---:|---:|
| `sources/leo.json` | `GROUP=weather&FORMAT=JSON` | 73 | 30,268 |
| `sources/heo.json` | `NAME=MOLNIYA&FORMAT=JSON` | 34 | 13,992 |
| `sources/gnss.json` | `GROUP=gps-ops&FORMAT=JSON` | 32 | 13,276 |
| `sources/geo.json` | `GROUP=geo&FORMAT=JSON` | 567 | 229,258 |

The initial `GROUP=molniya` query returned an invalid-group message, not an orbital
snapshot; the current group index no longer listed that group. The documented
`NAME=MOLNIYA` query supplied the HEO candidates. No verification/error records
or synthetic orbital mutations were used to fill this pool.
[CelesTrak GP query/format documentation](https://celestrak.org/NORAD/documentation/gp-data-formats.php)
explains omitted EARTH/TEME/UTC/SGP4 metadata defaults; checked `OmmElements` is
still the Rust ingestion gate. Acquisition archives are **not case inputs**:
the 32-record/32-KiB case bounds apply to selected subsets, not these archives.

## Selection and exact bytes

Use fixed start **`2026-10-10T00:00:00Z`**, not retrieval time or the historical
cohort's `2006-06-26T00:00:00Z`. Within each source, select the lowest NORAD IDs
meeting all filters, with no search-result-based replacement:

- Absolute epoch distance from start ≤48 hours (past or future).
- LEO: 12 < mean motion < 18 rev/day, 0 ≤ eccentricity < 0.05; take eight.
- HEO: 1.89 < mean motion < 2.12 rev/day, 0.5 ≤ eccentricity < 1; take two.
- GNSS: 1.89 < mean motion < 2.12 rev/day, 0 ≤ eccentricity < 0.5; take two.
- GEO: 0.8 < mean motion < 1.2 rev/day, 0 ≤ eccentricity < 0.05; take four.

These are selection filters, **not branch-coverage assertions**. Rust separately
checks each initialized propagator's actual near-Earth/deep-space and resonance
variants. Source identity establishes the GNSS label; propagator path alone does
not establish a satellite's mission. Failed coverage/search gates require review,
not silent replacement.

| Population | Selected NORAD IDs, ascending |
|---|---|
| LEO | 28054, 29522, 32958, 35951, 37214, 37849, 38771, 39260 |
| Resonant HEO | 7376, 7780 |
| Nonresonant GNSS | 26407, 27663 |
| Resonant GEO | 19548, 20253, 21639, 22314 |

Order deep-space objects HEO₁/GNSS₁/GEO₁/HEO₂/GNSS₂/GEO₂/GEO₃/GEO₄, then
alternate LEO/deep-space objects. [selection.json](selection.json) publishes every
source's eligible IDs, selected names/epochs/order, zero-based source record
indices and raw-object hashes. It is acquisition metadata, not a capture manifest.
No names, IDs, epochs or orbital fields are edited. The LEO population is
weather-derived and predominantly polar; this is not broad inclination coverage
or a proposed production catalogue. Some HEO/GEO objects are old/inactive; this
suite tests orbit paths and costs, not operational status.

[pool.json](pool.json) copies selected JSON objects **verbatim**, wrapped in
`[\n` / `\n]\n` and separated by `,\n`. It is 6,672 bytes; SHA-256:
`64197cca475768a0d1030c504c2084fad3b865feccacb66d682a43ed36fdcec2`.
Whole documents and duplicate JSON keys are validated before extraction. The
archived lexical values are preserved, but Rust floating-point deserialization
is not a bitwise decimal/float or independent numerical accuracy contract.

## Offline regeneration and checks

From a fresh checkout, install/use the project's existing Rust toolchain and
cached Cargo dependencies; workspace simulator checks also need Homebrew SDL2
and the linker path in `.cargo/config.toml`. No ignored build artifacts, Python
packages, orbital downloads or hardware are prerequisites. Start with
`docs/HANDOFF.md`, `docs/PLAN.md` and `docs/DESIGN.md` for next-session scope.
From the repository root, these fixture checks run entirely offline:

```sh
python3 tools/prepare_m2_pool.py --check
python3 -m unittest discover -s tools -p 'test_prepare_m2_pool.py'
cargo run --offline --release --locked -p overhead-tools --bin overhead-m2-pool-preflight \
  > /tmp/m2-pool-suitability.json
cmp /tmp/m2-pool-suitability.json docs/evaluations/m2-pool-suitability.json
cargo test --offline --locked -p overhead-tools --bin overhead-m2-pool-preflight --test m2_pool_preflight
```

Omit `--check` only to reconstruct `pool.json` and `selection.json` from the
existing archives; archive hashes must pass first. The Python tests verify raw
object preservation, regeneration, hash-failure handling, document/duplicate-key
checks, and signed age/selection boundaries. Rust tests verify path/epoch/order
gates, every published search count, density tie-breaking, CLI validation and
working-directory independence. See [tool usage](../../README.md#overhead-m2-pool-preflight)
for what is intentionally not yet measured.
