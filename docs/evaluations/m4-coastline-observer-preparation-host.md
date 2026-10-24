# M4 observer-space coastline preparation — first host feasibility pass

Status: **host-only measurement, not a device design or acceptance**. The
[scope and open contract](m4-coastline-location-preparation-scope.md) records the
chosen first path and full loading-screen transition. This pass implements only
an isolated `#[cfg(test)]` seam in
`render/src/coast/representation_spike/preparation_trial.rs`, reusing the
[measurement candidate](m4-coastline-representation-spike.md). It does not
change the production renderer, runtime location API, loading screen or firmware.

## What was actually tried

Validate the existing 32,211 B packed fixture and observer; decode its 2,031
quadratic ECEF pieces. For that observer, test facing and cut roots once, then
retain independent visible camera-space subcurves in a host `Vec`. At draw time
only, apply the current lens to those curves and reuse the candidate's
screen-space flattening and clipping. There is no per-zoom bitmap; no curves are
bridged across part breaks. The draw seam refuses a different observer before
writing pixels. This is an intentionally *uncompressed*, allocating host
baseline, not a candidate for the device's zero-incremental-heap policy.

The active test compares fresh and prepared raster pixels across four FOVs at
one site and a separately prepared changed site. It rejects a truncated fixture
and invalid observer, refuses a stale site without drawing, and demonstrates
that host-side staging can discard a forced *late* invalid-curve draw while the
previous output remains unchanged. That is **not** a device rollback or UI test.

Reproduce the explicit measurement with:

```sh
cargo test -p overhead-render --release coast_preparation_trial -- --ignored --nocapture
```

One local release-profile host run used the eleven existing target-input scenes
and five draw invocations per site (scene FOV plus 0.1°, 5°, 20°, 100°; some
repeat). **All 55 complete geography rasters exactly matched** a fresh
candidate draw (zero differing pixels). One preparation per site was reused at
all its FOVs. Observed single-sample wall times: **34.4–64.7 µs prepare**,
**16.2–163.0 µs prepared draw**, **65.2–234.1 µs fresh candidate draw**.
These are order-of-magnitude host diagnostics, not medians, maxima over all
conditions, S3 forecasts, or a promised setup duration. Draw times include the
per-view lens calculation, flattening and pixel writes; blanking the screen is
outside both draw timers. Preparation times include structure validation, camera
setup, visibility cutting and host allocation, but not UI or transfer.

For these sites the retained geometry held **840–1,382** camera curves;
`Vec` capacity was **1,024–2,048** curves. The measured retained *capacity*
plus `Prepared` struct size was **37,000–73,864 B per observer** (36 B/curve).
The 32,211 B packed fixture remains a separate asset. `Vec` growth can briefly
hold old and new allocations concurrently: **peak temporary bytes were not
measured**; the capacity figure is not a peak memory bound. There is no heap
instrumentation, nested stack inventory, device free-RAM accounting or
full-frame scheduling evidence in this pass. In particular, the large retained
allocation is not approved by the provisional memory screens.

## Second host-only pass: lossy compact baseline

After separate authorization to continue host work, a **single** quantization
trial was added to the same test-only module. Each camera-space control is
encoded as three signed 16-bit offsets with a 0.25 km step (depth offset from
the fixed 75,000 km camera plane), for **18 B/curve**, instead of 36 B/curve.
Encoding checks finite/range values and refuses overflow rather than clipping.
The trial first builds the uncompressed result and then allocates an exact-size
compact `Vec`; this **does not** demonstrate bounded peak setup memory or
zero-heap operation. Reproduce with:

```sh
cargo test -p overhead-render --release coast_compact_trial -- --ignored --nocapture
```

One local release-profile host run of the same 11 sites × 5 FOV draws retained
**15,240–24,996 B** per observer including struct size, separate from the
32,211 B packed asset. Build-*plus*-pack took **45.0–208.2 µs**, compact draw
**13.8–202.0 µs** (single noisy host samples, not device predictions). Compared
bidirectionally against uncompressed complete masks, **964 differing pixel
instances** across 55 draws (some scene FOVs repeated); at most **252** in one
draw. No differing pixel was farther than **1 pixel** from the opposite mask
in these scenes. A separate ignored host test, `coast_compact_isolated_trial`,
then checked every one of the 1,734 original geographic source segments at its
own tight-zoom midpoint, plus stratified polar/dateline/limb/viewport scenes at
tight and overview zoom (2,342 isolated scenes total; 1,896 had reference ink).
Uncompressed preparation reproduced each isolated candidate raster exactly.
The compact result had **66,457 differing pixel instances**, **none beyond
1 pixel** from the opposite isolated mask. This is **not** exact parity, an
approved error tolerance, or proof that ordinarily visible coastline gaps
cannot occur. Reproduce with:

```sh
cargo test -p overhead-render --release coast_compact_isolated_trial -- --ignored --nocapture
```

The encoding's 16-bit representable range depends on the fixed
camera depth and tested observer domain; extreme site/altitude and numerical
boundary behavior remain untested. It is **not selected for production**.

## Third host-only pass: requested heap and stack inventory

A test-only thread-local `System` allocator wrapper meters requested bytes for
the same 11 site setups. It starts **after** building the immutable fixture,
then records uncompressed preparation, construction of the compact `Vec` while
the original remains alive, release of the original, one compact draw, and
release of the compact result. Reproduce with:

```sh
cargo test -p overhead-render --release coast_preparation_memory_trial -- --ignored --nocapture
```

Observed requested **live heap peak: 51,984–98,604 B**, with both prepared
vectors alive. The compact vector alone requested **15,120–24,876 B** after
release of the original; add its 120 B struct to compare with the preceding
15,240–24,996 B retained-capacity figure. Because `realloc` may temporarily
hold old and new blocks, a conservative **requested-byte transient envelope**
for this allocation sequence is **55,296–110,592 B**. It is not measured
physical peak/RSS, allocator overhead, DMA memory, or a proven bound on a
future implementation. Eleven sites had 10–11 successful allocation/reallocation
calls in preparation; the single compact draw had **zero** recorded calls.
These are thread-local host observations and require same-thread allocation and
free; the global wrapper does not count the preexisting source fixture or test
harness allocations. A `Vec`-free production design would have a different
peak, but **none has been implemented**.

The host `size_of` inventory is: **768 B** flatten pending array, **36 B**
curve, **68 B** context, **72 B** counts, **136 B** original struct, **120 B**
compact struct. These are individual values, not nested call frames or live
stack high-water. The earlier S3 candidate's **640 B** pending array and
**>1 KiB** nested frames remain the only target-specific stack evidence;
this host experiment does not supersede them or pass the provisional screen.

## Implemented slice (0058)

The wide view now uses a different, exact form instead of either retained
format above: a heap-free observer context plus front-facing piece mask
(**360 B** host `CoastLayer`), drawing through the candidate's per-piece
arithmetic. See [0058](../decisions/0058-prepared-wide-coastline-slice.md).
One release host run (`coast_prepared_layer_report`, ignored test) gave
**29–199 µs** preparation and prepared draws only ~10–25% faster than the
per-call candidate on most scenes; single noisy samples, not an S3 forecast.
[0059](../decisions/0059-current-view-coastline-raster-cache.md) then added a
10,560 B current-view raster cache (layer 10,960 B): cached draws averaged
roughly an order of magnitude below a fresh raster in two noisy host runs.

## Next gate (not automatically authorized)

Before choosing a retained format or any device check, examine streaming or
fixed-capacity preparation with an explicit peak-resource policy; the allocating
host path is not viable evidence for zero incremental heap. Obtain live nested
stack inventory for the *chosen implementation* rather than summing host
`size_of` values. Extend the isolated and tight limb, viewport, polar and dateline checks
to other heights/sites and zoom boundaries; add wider observer coverage, repeated timing,
zoom churn, and a whole-coast staging/rollback integration. Check ordinary
visible gaps under [0053](../decisions/0053-no-resolvable-coastline-omissions.md);
exact agreement with the accepted candidate on 55 masks does not prove all
observers or all zooms, nor make the candidate production-ready. Screen wording,
location setting, persistence, setup-duration ceiling, resource policy and any
S3 check require further decisions/authorization. Keep the frozen M2 record
untouched.
