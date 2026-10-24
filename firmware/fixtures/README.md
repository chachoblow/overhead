# Measurement-only fixtures

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
