# 0010 — Finite propagation and checked OMM metadata

Date: 2026-10-06
Status: accepted

## Decision
`Satellite::state_at` returns `PropagateError::NonFinite` for any NaN/infinite
position or velocity component, even after upstream success. Finite does not
mean physically accurate.

Deserialize OMM through `OmmElements` under the existing `omm` feature:

| Metadata | Required explicit value / default if omitted |
|---|---|
| `CENTER_NAME` | `EARTH` |
| `REF_FRAME` | `TEME` |
| `TIME_SYSTEM` | `UTC` |
| `MEAN_ELEMENT_THEORY` | `SGP4` |

Values must match exactly. Reject nulls, non-strings, duplicates, and conflicting
values; allow unrelated extra fields. This is not full CCSDS schema validation.

## Why
Review probes found finite corrupt elements yielding successful NaN predictions.
Raw `sgp4::Elements` also discards physical metadata, silently accepting TAI,
GCRF, Mars, or DSST while consumers assume UTC/TEME/Earth/SGP4. Check both at
reusable core boundaries, not only in host code or downstream geometry.

## Consequences
- Pass `.elements()` (borrowed) or `.into_elements()` (owned) to numeric/epoch
  ingestion. Raw element construction cannot validate discarded metadata.
- CLI uses the adapter; successful reports stay unchanged.
- Optional direct `serde` alloc/derive was already transitive: no new library
  or version. OMM remains no_std + alloc; default core is allocation-free.
- No freshness, catalogue conflict, or arbitrary orbit-limit policy is added.

References: [CelesTrak GP formats](https://celestrak.org/NORAD/documentation/gp-data-formats.php),
[Serde container](https://serde.rs/container-attrs.html) and
[field attributes](https://serde.rs/field-attrs.html).
