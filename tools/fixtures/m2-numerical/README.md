# Pinned M2 numerical references

Independent numerical preparation for [0019](../../../docs/decisions/0019-bounded-m2-evidence-preparation.md),
selected offline by `tools/prepare_m2_numerical.py`. No expectations are computed
with Overhead. Historical fixtures and tolerances are unchanged.

## Original TEME sources

The `vallado/` files are byte-for-byte downloads from CelesTrak's
[fundamentals-of-astrodynamics](https://github.com/CelesTrak/fundamentals-of-astrodynamics)
commit `49df0479c950917c0a2c5ac31f2db6477bba57f6`, retrieved 2026-10-11 UTC:

- `datalib/SGP4-VER.TLE`: original verification inputs (including appended test
  ranges). Only columns 1–69 of the selected two-line records are consumed.
- `software/cpp/TestSGP4/TestSGP4/{00005,08195,24208,28129}.e`: published TEME
  position/velocity ephemerides associated with Vallado et al., *Revisiting
  Spacetrack Report #3*, AIAA 2006-6753. These are reference output **data**, not
  vendored upstream implementation code.

Exact raw URLs, byte counts and SHA-256 hashes are in
[references.json](references.json). That file archives the selected decimal
values, TLE lines and complete existing coordinate/observer rows. All original
source bytes are checked in; regeneration/build/test requires no network. The
upstream ephemeris headers' `NumberOfEphemerisPoints` is not used to infer rows.
Selection requires actual rows at **0 / 21,600 / 86,400 seconds** and converts
only their time labels to **0 / 360 / 1440 minutes**. No interpolation, generated
vectors, or header-epoch rounding is used. Runtime timestamps are TLE epoch plus
those exact whole minutes.

| NORAD | Reference path | Times (minutes) |
|---|---|---|
| 00005 | near-Earth | 0 / 360 / 1440 |
| 08195 | resonant HEO | 0 / 360 / 1440 |
| 24208 | resonant GEO | 0 / 360 / 1440 |
| 28129 | nonresonant deep-space GNSS | 0 / 360 / 1440 |

All 12 states check each TEME position axis **<1e-6 km** and velocity axis
**<1e-9 km/s**, per [0006](../../../docs/decisions/0006-sgp4-crate-and-conventions.md).

## Complete existing geometry selection

The existing [core fixture README](../../../core/tests/fixtures/README.md) owns
ERFA/pymap3d/Skyfield provenance, versions, limitations and regeneration. Consume
all rows in original order, not a smaller target subset:

| Checker family | Rows | Strict gates |
|---|---:|---|
| Rotation | 9 | Euclidean ECEF position <1e-3 km |
| Geodetic | 13 | Forward position / inverse height <1e-3 km; inverse angles <1e-6° |
| Geometry | 27 | Range <1e-8 km; angles <1e-9° |
| IssPipeline | 12 | Range <0.1 km; angles <0.01° |
| VanguardPipeline | 2 | Existing composed epoch/+360 coordinate checks, position/round-trip <1e-3 km |

Together with TEME these are **75 rows**, not 75 scalar comparisons. A geodetic
row checks both directions. Longitude is skipped only at exact poles; near-pole
longitude and canonical angle ranges remain checked. Azimuth differences wrap
north. The two Vanguard rows reuse rotation indices 3/4 and the original 00005
TLE; they are not additional reference vectors or workloads. Synthetic/analytic
edge cases, round-trip grids and invalid-input tests remain host core regression
tests, not additional independent fixture rows in this diagnostic selection.

## Shared portable checker

`firmware/src/numerical.rs` is an opt-in, allocation-free `no_std` module under
`numerical-reference`. Both host tests/example and the future target diagnostic
call the **same `run` implementation**, selection and generated constants. Each
row returns a typed success/failure; any failed/missing row invalidates a capture.
It continues checking after failures and checks finite errors with strict `<`,
not `<=`. Position norms compare squared finite differences against the squared
1-metre limit; this avoids a new diagnostic-only math dependency without changing
the Euclidean bound. Existing isolated/pipeline angular tolerances are not widened.

`firmware/build_numerical.rs` converts pinned data on the build host, uses sgp4
only to parse TLEs, and emits numeric fields as `f64::from_bits`. It never propagates
or computes expected geometry. Names/designators are omitted from diagnostic
`Elements` only (not from catalogue ingestion); no runtime JSON/TLE parsing or
allocation is added. No capture protocol, target harness, or target execution is
provided by this preparation feature.

```sh
python3 tools/prepare_m2_numerical.py --check
cargo +stable test --offline --locked --manifest-path firmware/Cargo.toml --lib \
  --features numerical-reference
cargo +stable test --offline --locked --manifest-path firmware/Cargo.toml --lib \
  --features numerical-reference,catalogue-memory
cargo +stable run --offline --locked --manifest-path firmware/Cargo.toml \
  --example numerical_expected --features numerical-reference
cargo +stable run --offline --locked --release --manifest-path firmware/Cargo.toml \
  --example numerical_expected --features numerical-reference
```

Commands run from the repo root using the installed host stable toolchain, not
firmware's local Xtensa configuration. Debug/release host output agrees at
`checked=75 failed=0`. This is **not independent target numerical evidence** or
real-world orbital accuracy. Future S3 failures must be investigated without
relaxing these tolerances.
