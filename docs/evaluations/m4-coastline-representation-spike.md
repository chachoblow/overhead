# M4 coastline — one host representation / refinement experiment

**Subsequent result:** the user accepted the appearance and separately authorized
one [S3 feasibility check](m4-coastline-representation-target.md), now complete.
It greatly improves target time but fails the current rebuild/scratch screens;
0056's next step is to scope location preparation/reuse. The report below preserves
the host-stage evidence and recommendation. Runtime extraction is now available
only under tests or an explicit measurement feature, not production rendering.

2026-10-24. **Recommendation: promising enough for the one separately authorized
target feasibility check, not production adoption.** This is the bounded experiment
agreed in [0056](../decisions/0056-bounded-coastline-architecture-trial.md), following
the [rejection/reuse no-go](m4-coastline-candidate-spike.md). Baseline
`33fcf43fc467fa0936972fac782bf8737d377809` plus the test-only files described below
(uncommitted when measured); the production renderer, checked asset,
shared camera/tracking engine and firmware are unchanged. No target build, toolchain
change, board access, flash, new dependency or frozen-M2 refresh occurred.

## Result and stop boundary

The trial replaces geographic resampling with **2,031 packed quadratic Earth-fixed
pieces**, occupying **32,211 bytes**, and uses f32 transforms, analytic facing roots
and screen-dependent subdivision. There is **no per-point runtime trigonometry or
geodetic conversion in traversal**; the existing f64 camera setup still runs once
per invocation and quadratic roots may use `sqrtf`. Nothing observer-dependent is
prepared outside the complete-rebuild timer.

Across all eleven prior target scenes, host median rebuild time is **60.1–145.3 µs**,
versus **3,892.4–5,302.5 µs** for the current renderer in run 2 (about **36–65×** faster
per scene). Polar overview now processes 1,382 flattening nodes / strokes, with
4,146 simple 3D transforms instead of the old 101,847 full geographic evaluations.
These are **different units of work**, not an evaluation-cost ratio.

This improvement is not limited to tight-view culling: it removes repeated
transcendentals and fine overview sampling. That structural change justifies a
target check; **do not divide the old 9.78 seconds by a host ratio** to predict S3
cost. This experiment neither passes nor relaxes the 5 ms screen.

The packed asset fits the provisional 32 KiB screen by **557 bytes**. Total compiled
increment and live scratch remain unmeasured and could still fail. If the separately
authorized target feasibility check is unconvincing, follow 0056's location-preparation
pivot, not another general-renderer optimization campaign. No further host algorithm
variant, precision sweep or all-view proof is recommended before that decision.

## Candidate, explicitly experimental

Implementation: `render/src/coast/representation_spike.rs` and
`render/src/coast/representation_spike/packing.rs`, reachable only through `cfg(test)`.
The host prep reads the **unchanged, hash-pinned simplified asset**. It is not a new
Natural Earth reduction or a fetch. The original 1,734 segment boundaries and 134
part breaks remain available for isolated comparisons.

### Offline representation

- Fit a quadratic Bézier in ECEF through each segment's endpoints and geographic
  midpoint. The reference interpolant is still shortest-longitude/linear-latitude
  on zero-height WGS-84, not a geodesic or a straight ECEF chord.
- Split offline until the maximum error at 15 interior fitting samples is ≤**20 m**.
  This is a trial fitting criterion, **not a certified between-sample error bound**.
- Encode endpoints as three signed 24-bit coordinates, **1/1024 km** per unit.
  Encode a control-point correction from the endpoint average as three signed
  16-bit values, **1/128 km** per unit. Fit that correction using decoded endpoints.
  Overflow fails preparation rather than saturating/truncating. Adjacent pieces
  share their endpoint bytes; each part starts independently.
- Layout: 135 little-endian u32 byte offsets (**540 B**), one 9-byte initial endpoint
  per part (**1,206 B**), then 15 bytes per piece (**30,465 B**). Total **32,211 B**.
  Source-span and geographic-parameter audit metadata are host-only, not device
  asset bytes. The timed reader consumes actual packed bytes, not host f64 tables.
- Regenerated trial SHA-256:
  `6c478669a07d8683cf842b5ee8eb197723e05dbbe1a063508601866e3c5d3d02`.
  The generated binary is disposable host output, not a bundled production asset.

A finer, 65-point-per-piece audit of the decoded quadratic observes maximum ECEF
error **0.021858 km**. A chord-only replacement between the original segment
endpoints reaches **38.955270 km** in its sampled audit. These are empirical 3D
errors against the simplified interpolant, not source-geography accuracy or screen
bounds. The chord control confirms why precomputed endpoints alone are insufficient.

### Runtime trial

- Validate packed *structure*, then construct a coast-local f32 affine transform
  from the existing public f64 camera. The geographic preparation is excluded from
  runtime, but validation and camera construction are included. This is a generated,
  trusted experiment input; its structural check is **not** a production corrupt-data
  validation/checksum policy.
- For a true ellipsoid point P, camera-facing sign is equivalent to
  `C · diag(1/a², 1/a², 1/b²) P - 1`. Applying this plane to the approximate quadratic
  makes facing a quadratic polynomial. Reject an all-nonpositive control hull;
  otherwise solve its interior roots and render positive runs. Both hidden
  endpoints can enclose a visible run. The curve is not exactly on the ellipsoid,
  so this is an approximation choice that needs the comparisons below.
- Transform retained controls to camera coordinates. For positive depths, the
  rational projected curve lies within its projected control hull in exact
  arithmetic. Reject a hull wholly outside one viewport half-plane; do not reject
  merely because the two endpoints are outside.
- Split with de Casteljau until projected control-to-endpoint-segment distance is
  ≤**0.25 px**. Clip and rasterize the resulting line with the existing 1-bit line
  primitive. The flatness threshold is experimental, not an adopted visual budget.
- Arithmetic during traversal, projection and clipping is f32, without changing
  satellite/shared projection precision. Facing roots use the stable quadratic
  formulation, including linear/double-root cases. Neither predicates nor sampled
  fitting have formal outward-rounding bounds; this is not an all-view guarantee.
- A fixed depth-first pending array supports 16 levels; an invocation node cap is
  65,536. Exhaustion or invalid geometry returns an explicit experiment failure,
  never a successful partial coast. No observed scene exhausted either limit.
  The test target is infallible; this is **not** a production draw-target error,
  exceptional rollback, caching or scheduling implementation.

## Work and host timing

Two sequential release runs on arm64 Darwin / Apple M1 Pro, rustc 1.99.0. Five warm
samples per implementation and scene, alternating order. Full candidate traversal,
structure checks, camera setup, counters, clipping and raster are timed; offline
fitting, fixture construction, screen blanking and comparisons are outside timing.
The host target is the byte-per-pixel test `Screen`, not the S3 packed framebuffer.
Integer counts and shape metrics match in both runs. Table timings are run 2 medians.

| Scene | Simple transforms | Flattening nodes | Drawn strokes | On writes | Baseline / trial µs |
| --- | ---: | ---: | ---: | ---: | ---: |
| SF 0.1° | 3,183 | 1,067 | 3 | 237 | 4,042.9 / 63.3 |
| SF 1° | 3,183 | 1,067 | 12 | 289 | 4,094.0 / 64.5 |
| SF 5° | 3,183 | 1,061 | 333 | 1,932 | 4,327.4 / 86.7 |
| SF 100° | 3,183 | 1,061 | 1,061 | 1,249 | 4,884.8 / 108.8 |
| Colorado 20° | 3,273 | 1,091 | 1,091 | 2,204 | 4,889.1 / 121.5 |
| Polar 20° | 4,146 | 1,382 | 1,382 | 2,860 | 5,302.5 / 145.3 |
| Dateline 5° | 2,520 | 840 | 165 | 867 | 4,075.0 / 69.7 |
| Limb 20° | 2,973 | 991 | 991 | 1,892 | 4,567.9 / 115.5 |
| Limb 100° | 2,973 | 991 | 991 | 1,150 | 4,588.9 / 104.9 |
| Viewport 0.1° | 2,745 | 915 | 2 | 256 | 3,892.4 / 60.1 |
| ISS ground 20° | 2,646 | 882 | 882 | 1,758 | 4,556.1 / 101.5 |

All invocations still inspect 2,031 pieces / 6,093 facing control values; there is no
spatial index or persistent observer cache. The 10–42 mixed-facing pieces per scene
can require a quadratic-root square root. Overview scenes require no additional
runtime flattening splits after offline fitting. Tight SF needs three; the separate
midpoint sweep reaches three pending entries. Fewer On writes mainly reflect less
repeated ink, not an equivalent reduction in unique visible coastline pixels.

## Shape evidence and subsequent visual acceptance

For each of the eleven scenes, compare **every original segment in isolation** and
then the whole coast layer. Compare raster pixels in **both directions**: every
reference pixel must have same-segment candidate ink nearby and vice versa. An empty
opposite mask is a failure distance, not a vacuous match. All compared pixels were
within **one Chebyshev pixel** of the opposite mask; none were >1px or >3px away.

This is not exact parity. For example, SF 0.1° has **178 changed pixels**, SF 5° has
**601 whole-layer changed pixels**, and polar 20° has **314**. Long, slightly different
raster paths can change many pixels while remaining adjacent. The masks retain
recognizable angular shape in agent inspection. The user subsequently reviewed
the candidate comparison and **accepted its appearance on 2026-10-24 UTC**. This
acceptance is separate from the automated checks and does not authorize target
work or production adoption. A one-pixel neighborhood check can tolerate
small local discontinuities; it does not prove absence of every resolvable gap.

Additional controls:

- **1,810 isolated tight-view scenes:** all 1,734 original segments with the observer
  at their geographic midpoint, FOV 0.1°, plus 76 stratified repeats at 2.187 km
  observer height. All reference masks contain ink. Bidirectional maximum distance
  is one pixel; no >1px differences. These are not 1,810 whole-world render tests.
- A separate same-parameter projection audit samples each decoded piece 65 times
  in each of the eleven scenes. The maximum sampled displacement is **0.013164 px**
  for reference-front-facing, in-viewport points. It includes trial f32 transform
  effects but excludes rasterization and does not bound visibility at all tangencies.
- Ordinary tests cover signed packing/reproducibility, malformed structure,
  camera/facing agreement away from numerical boundaries, viewport crossings,
  dateline, pole, nonzero observer height, limb crossing, hidden-endpoint visible
  runs, separate part breaks, quadratic/double roots and explicit work-limit errors.

Retained **native 400×240 coast-only test masks**, not integrated simulator frames,
physical-panel evidence or production snapshots:

- [Main comparison](m4-coastline-representation-spike/native-review.png): six scenes,
  existing renderer / trial / black changed pixels.
- [Other five scenes](m4-coastline-representation-spike/other-scenes.png).
- [Isolated worst-segment examples](m4-coastline-representation-spike/isolated-review.png).
  Exact segment identifiers are the final `worst` rows for each scene in the logs.

Do not equate the same simplified-asset reference with independent cartography.
[0053](../decisions/0053-no-resolvable-coastline-omissions.md) remains in force and
[0055](../decisions/0055-defer-all-view-coastline-proof-for-m4.md) still defers an
all-observer theorem. This finite comparison does not restart that proof effort.

## Resource risks and next gate at the host-stage close

- Asset increases by **16,727 B** versus the current 15,484-byte export. Passing the
  asset screen alone says nothing about fitting code plus data into 64 KiB. The old
  measurement image had only 516 B of compiled-increment margin. Replacement code,
  linked numeric support and instrumentation must be measured together, not added
  to or subtracted from the old ELF by assumption.
- The host pending-array layout is **768 B**, excluding the current curve, camera,
  roots, projection temporaries, callers, raster state and counts. Runtime traversal
  has no explicit heap allocation or recursion, but this does **not** demonstrate
  ≤1 KiB target scratch or safe stack. Host preparation/audit allocate freely.
- The timed path still performs thousands of f32 transforms, divisions, checks and
  pixel operations. No new Xtensa code generation or timing was inspected. There
  is no promise that f32, raster cost or the remaining setup fits 5 ms.
- Production `COAST ERROR` rollback and foreground tests remain unchanged, but this
  candidate has not been integrated with them or a fallible device target. Any
  target extraction must remain measurement-only and preserve existing production
  behavior; do not promote the test renderer merely to make it measurable.
- Selected FrameOff and Sharp/whole-device 20 Hz remain separate open blockers.

**Recommendation at the host-stage close:** seek separate approval to prepare/gate the single target feasibility
check for this candidate, including true code/data increment, full rebuild time,
scratch and representative output comparison. No further numerical/appearance
variant is proposed. If size/time/scratch or shape is unconvincing, stop this path
and scope observer-specific preparation under 0056. The location configuration,
preparation delay, retained representation and persistence would be separate choices.

## Reproduce and verification

From the repository root, host only:

```sh
OVERHEAD_COAST_REP_OUTPUT=/tmp/overhead-coast-representation \
  cargo test -p overhead-render --release coast_representation_spike -- --ignored --nocapture \
  > /tmp/coast-representation.txt 2>&1
python3 docs/evaluations/m4-coastline-representation-spike/validate_spike.py \
  /tmp/coast-representation.txt
# Optional: compose the retained 1-bit sheets; uses existing host Pillow.
python3 docs/evaluations/m4-coastline-representation-spike/validate_spike.py \
  /tmp/coast-representation.txt --images /tmp/overhead-coast-representation
```

[Run 1](m4-coastline-representation-spike/host-run1.txt),
[run 2](m4-coastline-representation-spike/host-run2.txt) and
[review input hashes](m4-coastline-representation-spike/review-manifest.json) are retained.
The default validator checks both logs and the unchanged production asset hash.
It checks metrics; the Rust probe performs the actual isolated pixel comparisons.
Review composition verifies input hashes before replacing generated sheets.

Workspace check/tests, renderer measurement-feature tests, app/render no-default
check and formatting passed. The release experiment passed twice with matching
counts/shape metrics, and its evidence audit and retained prior candidate/work/S3
log audits passed. No firmware host test, target build/run or hardware operation
was performed. Passing host tests is not device or user visual acceptance.
