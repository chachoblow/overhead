# Host experiment fixtures

## `m2-pool/`
Separate pinned 16-object real-orbit acquisition set for bounded M2 preparation.
Source bytes/URLs/retrieval timestamps/hashes, deterministic selection rules and
offline regeneration live in its [fixture README](m2-pool/README.md). Initial
[source suitability](../../docs/evaluations/m2-pool-suitability.md) passes; the full
host preflight/capture manifest is not frozen. Historical files below are unchanged.

## `m2-cases/`
Candidate document archive for all 20 bounded M2 host cases, including explicitly
synthetic historical ingestion variants. Exact bytes/hashes/order and offline
regeneration live in its [README](m2-cases/README.md); [host evidence](../../docs/evaluations/m2-host-preflight.md)
covers retained diagnostics, work, collection capacities, RAM lifetimes, requested
peaks and full release. Numerical preparation/final manifest freeze remain open.

## `catalogue-costs.tle`
Eight distinct historical records from the same pinned [Vallado/CelesTrak
SGP4 verification source](https://github.com/CelesTrak/fundamentals-of-astrodynamics/blob/98e731d390150d31defc205eff84615a5ff894f0/datalib/SGP4-VER.TLE),
retrieved 2026-10-10 UTC. This is a new input file; `pass-intervals.tle` and the
ISS fixture remain unchanged.

| Order | NORAD | Source name label | Coverage |
|---:|---:|---|---|
| 1 | 06251 | DELTA 1 DEB | Near-Earth LEO, moderate drag |
| 2 | 08195 | MOLNIYA 2-14 | Resonant HEO |
| 3 | 28129 | NAVSTAR 53 (USA 175) | Nonresonant GNSS |
| 4 | 24208 | ITALSAT 2 | Inclined resonant GEO |
| 5 | 28057 | CBERS 2 | Near-Earth low-eccentricity LEO |
| 6 | 09880 | MOLNIYA 1-36 | Another resonant HEO |
| 7 | 14128 | EUTELSAT 1-F1 (ECS1) | Inclined near-geosynchronous orbit |
| 8 | 28626 | XM-3 | Low-inclination resonant GEO |

Chosen for distinct identities, mixed propagation paths, and epochs within two
days of a common `2006-06-26T00:00:00Z`. The first four form the smaller subset;
the full eight are not an equally mixed population or production catalogue.
No synthetic error-case IDs, deliberately decayed records, or epoch mutations.

Regenerate by downloading the pinned source, selecting the listed IDs in this
order, taking exactly the first 69 characters of each TLE line (including its
checksum), prepending the listed name line, and writing LF-terminated UTF-8.
All 16 orbital lines were compared with the pinned source. SHA-256:
`2cb96da57983e5d6f2f145be410bdab9919caec417b6cba5535edb939ae55b99`.

The benchmark parses with the existing sgp4 TLE parser and serializes those
values to OMM JSON in memory before measurements; the measured path uses checked
`OmmElements` and `CatalogueBuilder`. No orbital values are manually changed.
Existing JSON floating-point parsing can introduce tiny binary rounding (tests
bound it); this is not bitwise identity with parsed TLE floats. Identity, epoch,
and other metadata are preserved exactly. No generated OMM file is authoritative
or independently verified. Runs/tests are offline and use no live UTC/data.
These inputs broaden cost coverage, not independent numerical accuracy coverage.
Usage and interpretation: [tool](../README.md#overhead-catalogue-benchmark),
[evaluation](../../docs/evaluations/catalogue-costs.md).

## `pass-intervals.tle`
Three historical records from Vallado et al.'s SGP4 verification set, retrieved
2026-10-09 from CelesTrak's `fundamentals-of-astrodynamics` repository:
[SGP4-VER.TLE, commit 98e731d](https://github.com/CelesTrak/fundamentals-of-astrodynamics/blob/98e731d390150d31defc205eff84615a5ff894f0/datalib/SGP4-VER.TLE).

| NORAD | Name | Coverage |
|---|---|---|
| 08195 | MOLNIYA 2-14 | High-eccentricity, 12-hour resonant HEO |
| 24208 | ITALSAT 2 | Inclined, 24-hour resonant GEO |
| 28129 | NAVSTAR 53 (USA 175) | GPS/GNSS, nonresonant deep-space path |

TLE lines are the source's first 69 characters, preserving checksums; trailing
verification-run time ranges are omitted. Name lines are labels from source
comments. No orbital fields were changed. SHA-256 of the LF-terminated local file:
`e2e27b0eb631dd4e172e661cfc7aea04797cabc2ee25f7d814a7caf8e697eced`.

To reconstruct, download the pinned source, select these three NORAD records in
the order above, retain the first 69 characters of each element line, and prepend
the listed name plus newline. Do not update elements to current epochs. These
are experiment inputs, not new independent expected-state or timing fixtures.

The interval and host benchmark tools share these inputs through
[historical_orbits.rs](../src/historical_orbits.rs) and also embed the existing
ISS OMM unchanged; its separate
[provenance](../../core/tests/fixtures/README.md#iss-25544json) remains authoritative.
All runs/tests are offline after Cargo dependencies are installed. No Python
packages, live fetch, current UTC, or hardware are needed; only the benchmark
uses a monotonic clock for elapsed host timing. Usage lives in
[tools/README.md](../README.md); methods and results in the
[detection evaluation](../../docs/evaluations/detection-intervals.md) and
[host cost evaluation](../../docs/evaluations/host-costs.md).
