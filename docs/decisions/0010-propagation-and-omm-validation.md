# 0010 — Reject non-finite propagation and incompatible OMM metadata

Date: 2026-10-06
Status: accepted

## Decision
`Satellite::state_at` returns `PropagateError::NonFinite` if any propagated
position or velocity component is NaN or infinite, even if upstream returns
success. Successful states guarantee finite components, not physical accuracy.

Use `OmmElements` (behind the existing `omm` feature) to deserialize OMM before
numeric/epoch ingestion. Omitted physical metadata uses CelesTrak GP defaults;
explicit `CENTER_NAME`, `REF_FRAME`, `TIME_SYSTEM`, and `MEAN_ELEMENT_THEORY`
must be exactly `EARTH`, `TEME`, `UTC`, and `SGP4`, respectively. Reject nulls,
non-strings, and duplicate declarations; unrelated extra fields remain allowed.

## Why
Review probes reproduced finite but corrupt elements producing successful
SGP4 predictions containing NaNs. Checking only input finiteness is insufficient;
rejecting output at the propagation boundary protects consumers that do not
immediately perform the already-checked coordinate conversion.

Direct deserialization into `sgp4::Elements` discards these OMM metadata fields.
The CLI consequently accepted explicit TAI, GCRF, Mars, or DSST declarations
while reporting UTC/TEME/Earth/SGP4 calculations. Missing CelesTrak defaults are
valid; explicit contradictory metadata is not. Validation belongs in a reusable
adapter, not a host-only JSON check or a new general OMM framework.

## Consequences
- The adapter exposes borrowed/owned `sgp4::Elements`; callers still use
  `Satellite::from_elements` for numeric and epoch validation. Direct raw
  element construction remains available and cannot validate discarded metadata.
- The CLI uses the adapter and preserves successful reports for existing data.
  Errors still produce stderr/exit 1 with no partial report.
- Add optional direct `serde` with alloc/derive, already transitive through
  `sgp4/serde`. No new library/version; default core stays allocation-free.
  OMM remains no_std + alloc, with file I/O outside core.
- No arbitrary orbit limits, freshness policy, physical-model changes, fixture
  changes, or catalogue conflict policy are introduced.

References:
- https://celestrak.org/NORAD/documentation/gp-data-formats.php
- https://serde.rs/container-attrs.html
- https://serde.rs/field-attrs.html
