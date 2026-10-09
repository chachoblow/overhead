# Host experiment fixtures

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
