# M4 coastline — offline performance / algorithm feasibility review

2026-10-23. Baseline `b8dbae8` plus the test-only work probe in
`render/src/coast/work_review.rs`. **No production renderer, asset, target build,
toolchain, hardware or frozen-M2 evidence changed.** The Stage A candidate still
fails its provisional time gate. This review recommends an investigation, not
adoption of a replacement algorithm or a different budget.

## Finding

The renderer performs a global, geometry-heavy rebuild even when almost no coast
is on-screen. Across the exact eleven Stage A inputs, successful source paths
require **88,551–101,847 `Camera::coast_point` evaluations per rebuild**. Each
performs WGS-84 conversion, camera projection and surface-normal facing; latitude
and longitude trigonometry are computed once in `to_ecef` and again for the normal.
Inspection of the **existing captured ELF** confirms that these are separate calls
and that double-precision arithmetic uses helper calls in this build.

This is a much stronger diagnosis than “too many drawn pixels” or “too many
bisection iterations”: SF 0.1° takes about **9.0 s for 242 On writes**. The same
observer at 100° has the same geometry count, about 52.5 times the writes, and
only about 2.8% longer coast time. Root searches account for at most **1.1%** of
coast-point evaluations in these scenes. A cheaper point path and substantially
less geometry work deserve investigation before raster tuning or root-limit cuts.

**Attribution limit:** source counts, static target call evidence and host timings
are not a target phase profile. They strongly implicate the repeated geometry
pipeline but do not measure its share of S3 time, individual helper latency,
validation time or the speedup of an unbuilt replacement. No “95% in trig” claim
is justified. A future separately authorized target experiment would need phase
or primitive measurements to resolve those shares.

## Reproduce and evidence

From the repo root, on the host only:

```sh
cargo test -p overhead-render --release coast_work_review -- --ignored --nocapture > /tmp/coast-work.txt 2>&1
python3 docs/evaluations/m4-coastline-work-review/validate_work.py /tmp/coast-work.txt
python3 docs/evaluations/m4-coastline-s3/validate_coast.py
```

The ignored test uses the exact rounded Stage A boundary coordinates and the same
computed ISS-ground fixture. It classifies each actual interval's endpoint/midpoint
facing signs, exercises the production interval successfully, and compares its
assembled pixels and On-write count against a complete production `coast::draw`.
An ordinary unit test covers all eight Boolean visibility branches of the count
model. The model follows `interval`, `crossing` and `stroke`; output equality alone
cannot prove that future source changes preserve its counts. It is not independent
geographic correctness evidence. The test-only target is the existing host
byte-per-pixel `Screen`, **not** the S3 packed 1-bit target.

Two sequential release runs on arm64 Darwin / Apple M1 Pro, rustc 1.99.0, are
retained as [run 1](m4-coastline-work-review/host-run1.txt) and
[run 2](m4-coastline-work-review/host-run2.txt). The
[audit](m4-coastline-work-review/validate_work.py) confirms identical counts in both
runs and matching On-write counts for all five retained S3 Coast rows in each
scene. Counts use **host-evaluated visibility signs**; matching write totals do
not independently prove identical target branch counts or byte-level geography.
No new S3 timing was captured. The existing
[Stage A protocol](m4-coastline-device-measurement-protocol.md) owns timing,
image identities, memory caveats and the STOP result.

### Successful-path work model

Every rebuild validates the static export and traverses all **1,734 segments /
23,377 intervals**. Validation and traversal each compute `steps` for every segment:
3,468 `hypot`/`ceil` evaluations total, independent of visible ink. There is no
part/segment front-face or viewport culling before interval sampling.

For `I` intervals, `R` visibility-root searches and `S` attempted strokes:

```
coast_point evaluations = 3 I + 24 R + 2 S
```

The first term samples both endpoints and midpoint, including fully hidden
intervals. Bisection samples 24 times per root. Each stroke recomputes its two
endpoint geometry samples, then performs lens projection and viewport clipping;
“stroke” in this table does not mean it survives clipping or draws any pixels.
Successful samples entail four source-level `sincos` calls and one WGS-84 square
root each, in addition to double arithmetic and finite/range checks. For polar
20°, that is **407,388 `sincos` calls** and **101,847 WGS-84 square roots**, excluding
one-time camera setup and step-validation math. These are source-path counts,
not a general instruction count or measured cycles.

| Scene | Fully hidden intervals | Roots | Attempted strokes | Coast-point evaluations | On writes | Retained S3 Coast median (µs) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| SF 0.1° | 10,864 | 10 | 12,513 | 95,397 | 242 | 9,001,494 |
| SF 1° | 10,864 | 10 | 12,513 | 95,397 | 362 | 9,032,787 |
| SF 5° | 10,864 | 10 | 12,513 | 95,397 | 5,180 | 9,134,181 |
| SF 100° | 10,864 | 10 | 12,513 | 95,397 | 12,714 | 9,255,295 |
| Colorado 20° | 10,427 | 16 | 12,950 | 96,415 | 14,393 | 9,338,142 |
| Polar 20° | 7,687 | 14 | 15,690 | 101,847 | 17,613 | 9,781,125 |
| Dateline 5° | 14,383 | 18 | 8,994 | 88,551 | 2,112 | 8,210,533 |
| Limb 20° | 12,706 | 42 | 10,671 | 92,481 | 11,827 | 8,896,160 |
| Limb 100° | 12,706 | 42 | 10,671 | 92,481 | 10,839 | 8,902,380 |
| Viewport 0.1° | 13,796 | 22 | 9,581 | 89,821 | 263 | 8,375,401 |
| ISS ground 20° | 12,847 | 26 | 10,530 | 91,815 | 11,629 | 8,808,277 |

Each target median is five samples from the **existing one-boot on capture**.
The maximum remains **9,781,141 µs**, about **1,956×** the provisional 5 ms screen.
Dividing full target elapsed time by evaluation count gives 92.7–97.0 µs/evaluation
across these scenes, but includes every other operation: **not a primitive timing**.

### Existing target code, not a new build

The ignored local captured on ELF was available and its SHA-256 was rechecked:
`24d4a4e652980736ed33c4ae5bca40dad1f1f3bb4943510c5deca40da598f7a7`, exactly the
captured ELF recorded in the protocol. The asset hash also still matches
`951ed542a445201c24241bfb263b81567cef20641854be281778788a7a7a6796`.
The installed `xtensa-esp32-elf-objdump` was invoked directly to **read** that ELF;
no environment/toolchain change, target compiler invocation or hardware access
was involved.
Retained function-range disassemblies:

- [`Camera::coast_point`](m4-coastline-work-review/coast-point-disassembly.txt),
  `0x4203d514..0x4203d7c0`: loads/calls `to_ecef`, then two separate `sincos`
  calls; `__muldf3`, `__adddf3`, `__subdf3` and `Camera::project` are also visible.
- [`GeodeticPosition::to_ecef`](m4-coastline-work-review/ecef-disassembly.txt),
  `0x420453dc..0x420455de`: two further `sincos` calls, `sqrt`, and `__divdf3`.

Recreate the excerpts, if that exact local artifact remains available, with
`xtensa-esp32-elf-objdump -d -C --start-address=<start> --stop-address=<end> <ELF>`.
The full ELF is still an ignored local artifact, not newly bundled evidence.
Helper calls establish the compiled arithmetic path, not their execution cost.

### Host timing cross-check

The probe warms the renderer, then records five complete coast samples and five
100-call validation batches per scene, with setup/blanking outside timing.
Run 2 has validation medians **17.0–17.9 µs**, versus complete coast medians
**3,756–5,253 µs**: roughly 0.3–0.5%. Run 1's first scene was noisier (26 µs /
5,840 µs); it is retained, not discarded. These are host observations, not a
measured S3 validation fraction. Moving asset validation out of the draw may be
worthwhile eventually, but the available evidence does not support treating it
as the primary remedy. There is no new production validation lifetime contract.

The target timer brackets `invoke`; app construction, coast-only blanking,
watermark paint/scan, checksum and UART are outside its elapsed region. The
per-pixel target bounds checks/write accounting and the normal rendering work
are inside. Thus retained seconds are not app ingestion, serial printing or a
full framebuffer checksum mistakenly charged to coast. This source audit does
not independently calibrate the hardware timer.

## Candidate feasibility, ranked — none adopted

### 1. Preferred next investigation: remove global expensive resampling

Scope a **host-only candidate feasibility spike** around conservative part/segment
visibility/viewport rejection and cheaper reusable geometry. Measure how much work
can be avoided on *all eleven* scenes, especially polar and overview, rather than
optimizing only SF tight zoom. For retained geometry, investigate sharing ECEF /
normal / camera samples and moving observer-independent preparation offline.
Keep uncertain segments for refinement; do not discard them.

The hard problem is replacing tens of thousands of runtime transcendental-heavy
samples with a bounded representation/evaluation path **without changing the
accepted coast in ordinary views**. The existing 0.2° interpolant is linear in
longitude/latitude on WGS-84, not an ECEF straight chord. A precomputed chord-only
replacement, coarser sampling or lower precision is a new numerical candidate,
not a harmless refactor; tight-zoom curvature, limb crossings, dateline and polar
cases need comparison. Single precision is not selected merely because the
current target uses double helpers. Do not alter the verified tracking engine
or shared satellite projection to accelerate coast.

Storage constrains naive precomputation. At 1,868 vertices, three `f32` coordinates
plus the existing 540-byte part table would be **22,956 B** if replacing the
current representation; adding three normal components makes **45,372 B**, already
over the 32 KiB asset screen. Even just one 12-byte position per interpolation
interval is **280,524 B**, before endpoints, parts, normals or indexing. These are
layout arithmetic examples, **not approved encodings**, precision guarantees or
compiled-image estimates. The existing image has only 516 B margin under its
provisional 64 KiB increment; measure replacement code/data together later.

**Exit from a follow-up spike:** a compact candidate sketch, per-scene work/storage
counts and correctness comparisons showing whether a large reduction is plausible,
or an explicit no-go. Host speed alone cannot pass the S3 gate. Do not begin
production replacement until its numerical representation and tradeoffs are
reviewed; any changed appearance or persistent buffer/budget needs a decision.

### 2. Lower-risk reuse is a useful control, not a sufficient rescue

Retain evaluated endpoints for stroke projection, share sine/cosine values within
a coast-specific conversion, and reuse adjacent interval endpoints. Keeping the
same endpoints and midpoints across 1,734 segments needs roughly
`2 × 23,377 + 1,734 = 48,488` point evaluations before roots—only about a twofold
reduction from current polar work. Sharing trig can remove two of four `sincos`
calls per evaluation, not all conversion/projection arithmetic. These counts are
not additive speedup promises, and neither is close to the roughly 1,956× elapsed
gap on its own. Avoid spending multiple sessions on micro-optimizations without
an architecture-level work reduction and a target measurement plan.

### 3. Cache-only / policy work is not a pass for this gate

A stable observer and FOV permit reuse in principle, but a persistent wide coast
bitmap alone is **10,560 B**, explicitly outside the current default scratch
allowance. Caching does not fix a nine-second cold rebuild or interactive zoom,
and amortized redraw cost does not satisfy the complete-rebuild gate. Geometry
caches have their own storage and invalidation costs. No cache, slower coastline
cadence, new scheduler or widened budget is selected here.

Do not cut bisection iterations, drop uncertain fragments, suppress coast at tight
zoom, or invoke `COAST ERROR` routinely as a performance remedy. Preserve
[0053](../decisions/0053-no-resolvable-coastline-omissions.md) and the exceptional
[0054](../decisions/0054-first-executable-coastline-comparison.md) failure policy.
[0055](../decisions/0055-defer-all-view-coastline-proof-for-m4.md) still defers an
all-view theorem for this iteration; this review found no new omission and does
not restart that proof effort.

## Separate blocker and verification

Selected FrameOff already reached **136,789 µs** in the on image, before Sharp
transfer or representative device load. That non-coast renderer cost will need
its own bounded review before any 20 Hz claim; fixing coast does not fix it.
No glyph or other foreground optimization is part of this review.

The work probe and count-model unit test are test-only. Existing finite-scene
visibility, foreground, rollback and measurement-feature regressions remain the
controls for a future candidate; no new independent geography oracle is claimed.
The session ran workspace check/tests, renderer measurement-feature tests,
app/render no-default-feature check, formatting, both offline evidence validators
and whitespace checks. No firmware source changed or target was built/run.
The next step from this review was a **separately scoped host candidate
investigation**, not Sharp bring-up or another target flash. The subsequent
[rejection/reuse spike](m4-coastline-candidate-spike.md) records that investigation's
finite-scene parity and no-go as a stand-alone device remedy; it does not alter
this review's measurements.
