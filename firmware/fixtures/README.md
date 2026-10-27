# Experimental firmware fixtures

## `coast-quadratic-trial.bin`

The **32,211-byte** packed quadratic ECEF asset for the single authorized
[0056 target feasibility check](../../docs/evaluations/m4-coastline-representation-target.md),
not a production/device asset or a location-specific firmware requirement.
SHA-256: `6c478669a07d8683cf842b5ee8eb197723e05dbbe1a063508601866e3c5d3d02`.

It is deterministically derived from the unchanged
[`render/assets/coastline-110m-40km.bin`](../../render/assets/README.md): Natural Earth
1:110m v4.1.0, 40 km simplification, 134 parts / 1,734 original segments. Original
source provenance and public-domain terms remain in that README; no new download
or geographic reduction is performed. The experimental representation, quantization,
20 m sampled fitting criterion and limitations are documented in the
[host trial](../../docs/evaluations/m4-coastline-representation-spike.md).

Regenerate on the host from repository root (no firmware build or device access):

```sh
OVERHEAD_COAST_REP_OUTPUT=/tmp/coast-quadratic-regeneration \
  cargo test -p overhead-render --release coast_representation_spike -- --ignored --nocapture
cmp firmware/fixtures/coast-quadratic-trial.bin \
  /tmp/coast-quadratic-regeneration/quadratic-trial.bin
shasum -a 256 /tmp/coast-quadratic-regeneration/quadratic-trial.bin
```

The generator and original host-reference decoder are test-only in
`render/src/coast/representation_spike/packing.rs`. The target reads the same
packed bytes through `decode.rs`; host tests compare the decoders. The normal
renderer still uses its original asset. Actual link inspection confirmed this
candidate asset only in the on image and neither coastline asset in the off image.
Changing this file would invalidate the recorded target-image evidence; do not
regenerate it to conceal a mismatch.

## `live-utc-iss-20261026.json`

A **separate diagnostic-only** single-record ISS OMM array, not a replacement
for `core/tests/fixtures/iss-25544.json` or any frozen M2 input.

- Source: [CelesTrak GP by CATNR=25544, JSON](https://celestrak.org/NORAD/elements/gp.php?CATNR=25544&FORMAT=JSON), retrieved on 2026-10-26 UTC; host manifest recorded `2026-10-26T23:02:53Z` as fetch metadata. The endpoint is mutable.
- NORAD ID: 25544; element epoch: `2026-10-26T05:01:03.508032Z`.
- Exact checked-in bytes SHA-256: `418a12a93d28234192d8b4fc8876937f381e87012d6fb747decfab6a1e3dde27` (one JSON array line followed by LF). The original response was fetched through a web tool; the checked-in file records its returned JSON with an LF, so this hash is for **these checked-in bytes**, not an attestation of the remote HTTP response bytes.
- Consumer: only the separately named `overhead-s3-live-utc-fresh-iss` diagnostic and its host preflight. The existing `overhead-s3-live-utc-radar` still uses the old fixture. The new target image neither fetches nor refreshes this data.

This is a **dated visibility experiment**, not a lasting claim of current ISS
accuracy. The [firmware guide](../README.md#pinned-omm-live-utc-marker-diagnostic-flashed-no-visual-observation)
records host, build and bounded UART evidence; recheck input age and an upcoming in-view
window before any proposed physical capture. Never refresh this file silently
or treat a newly fetched response as the same image. No production data-age,
fetch, persistence or display policy follows.
