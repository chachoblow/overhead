# M4 coastline — host-only rejection / sample-reuse spike

2026-10-23. **No-go as a stand-alone device remedy; retain as experimental evidence.**
This follows the [performance review](m4-coastline-performance-review.md) at baseline
`b8dbae8` plus the test-only review/spike changes. Production coastline,
asset, shared projection, firmware and frozen M2 evidence remain unchanged. No
target build, hardware access, toolchain change or new dependency.

## Question and outcome

Can conservative segment rejection and reuse of the existing geographic samples
plausibly rescue the current comparison without changing its pixels?

The prototype preserves **exact coast pixels and On-write counts for all 1,734
isolated segments in each of eleven scenes**, and for the complete coast layer.
It reduces tight-view work strongly. However, polar 20° still evaluates **19,653**
full `Camera::coast_point` samples (from 101,847) and retains **15,890** fine
intervals. The two host runs show only about **2.5×** polar elapsed speedup. Limb
scenes retain **20,446** evaluations. Conservative culling and reuse alone are
therefore not a credible stand-alone route across the baseline's roughly
**1,956× S3 elapsed gap**.

This review recommends **not promoting this prototype or requesting its target build**;
this is not a measured S3 speed or proof that coastline rendering is impossible.
The rejection/reuse techniques may be useful within a different geometry path.
No lower-precision representation, coarser coast, persistent cache, revised budget
or production algorithm was adopted.

## Prototype boundary

`render/src/coast/candidate_spike.rs` is included only under `cfg(test)`:

1. Validate the unchanged asset on every invocation, then create the same camera.
2. Evaluate one midpoint per original segment. Use loose analytic travel/facing
   envelopes over the entire shortest-longitude / linear-latitude interpolant to
   reject only definitely hidden or wholly one-sided off-viewport segments.
   For half-span `h = (|Δlon| + |Δlat|)/2` radians, position travel is bounded by
   `7000 h` km and facing change by `107000 h` km for zero-height observers.
   Added f64 margins are **pragmatic, not directed-rounding certification**.
   Projection error encloses the continuous surface curve; an envelope wholly
   outside one viewport half-plane also excludes its endpoint raster chords.
3. Retain unknowns. Keep the original ≤0.2° sampling, 24-step bisection, clipping,
   line raster and part breaks for retained segments. Reuse adjacent endpoints
   and evaluated stroke endpoints. Skip midpoint visibility tests only when the
   entire segment's facing envelope is strictly positive.
4. Consume only a rolling set of samples; no map cache, extra asset or persistent
   bitmap. Nonzero observer altitude bypasses coarse rejection/front shortcuts;
   it is not silently treated as a certified zero-height case.

The envelope shape is adapted from the existing finite-scene visibility study;
this is a work-reduction experiment, **not a restart of the all-view proof**.
The test-only candidate duplicates some successful-path interval/raster logic so
production code need not be refactored to host an unselected experiment. It uses
an infallible test screen and assertions; it is **not** a new production failure,
rollback or draw-target error contract. Unknown bounds never drop an interval.

## Reproducible results

Run on arm64 Darwin, rustc 1.99.0, release profile. Each row reports five warm
complete-coast samples for each path, alternating baseline/candidate order.
Validation, camera construction, geometry, clipping, raster and counters are timed;
fixture construction, blanking and output comparisons are not. The target is the
existing host byte-per-pixel `Screen`, not the S3 packed 1-bit framebuffer.

| Scene | Hidden / outside segments | Retained intervals | Baseline evaluations | Cull only¹ | Cull + reuse | Host baseline / candidate median µs² |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| SF 0.1° | 799 / 934 | 27 | 95,397 | 1,869 | 1,762 | 4,012.3 / 113.0 |
| SF 1° | 799 / 926 | 176 | 95,397 | 2,614 | 1,919 | 4,060.8 / 124.5 |
| SF 5° | 799 / 555 | 4,955 | 95,397 | 26,415 | 7,116 | 4,286.2 / 652.0 |
| SF 100° | 799 / 0 | 12,883 | 95,397 | 65,649 | 16,388 | 4,758.4 / 1,697.4 |
| Colorado 20° | 790 / 0 | 13,268 | 96,415 | 67,822 | 17,076 | 4,810.5 / 1,790.9 |
| Polar 20° | 531 / 0 | 15,890 | 101,847 | 81,120 | 19,653 | 5,205.8 / 2,098.2 |
| Dateline 5° | 960 / 571 | 1,996 | 88,551 | 11,722 | 3,980 | 3,801.0 / 323.3 |
| Limb 20° | 793 / 0 | 12,726 | 92,481 | 62,262 | 20,446 | 4,498.2 / 1,738.0 |
| Limb 100° | 793 / 0 | 12,726 | 92,481 | 62,262 | 20,446 | 4,507.3 / 1,724.5 |
| Viewport 0.1° | 907 / 820 | 73 | 89,821 | 2,099 | 1,814 | 3,744.8 / 117.2 |
| ISS ground 20° | 955 / 0 | 11,765 | 91,815 | 58,713 | 17,878 | 4,469.5 / 1,607.5 |

¹ Cull-only is a **source-work count**, not another timed renderer: the original
successful interval path on retained segments, plus 1,734 bound evaluations.
The candidate counter records actual calls to its sample wrapper, including
bounds, retained endpoints/midpoints and roots. Each full sample still uses the
unchanged double-precision conversion/projection and duplicate trigonometry.

² Medians from [run 2](m4-coastline-candidate-spike/host-run2.txt).
[Run 1](m4-coastline-candidate-spike/host-run1.txt) is also retained; all integer
counts match. Neither sample set is a target forecast or timing bound. In
particular, **do not multiply baseline S3 seconds by the host speedup** or divide
by the sample-count reduction to claim candidate S3 time.

### Resource accounting

- Export is still **15,484 B**; no extra table or exported representation. Hash
  remains `951ed542a445201c24241bfb263b81567cef20641854be281778788a7a7a6796`.
- Candidate traversal uses no allocation or recursion. One `Evaluated` value is
  six f64 scalars (48 B); only a bounded rolling set is needed, not 23,377 stored
  intervals. This is source-level accounting, **not** a measured stack increment,
  proof of the ≤1 KiB screen or a target ABI/compiler layout result. The host
  fixture/screen/test allocations are not a proposed device memory layout.
- No new image was built. Additional rejection code and duplicated test code are
  not a measured compiled increment; the old image's 516 B margin under 64 KiB
  gives no permission to add code unchecked.
- Wide scenes still have thousands of expensive retained samples and fine
  strokes. At polar 20°, 19,653 evaluations still imply 78,612 source-level
  `sincos` calls, excluding camera setup. Tight-view success does not address this
  overview workload. No partitioning/indexing tweak alone is demonstrated to do so.

## Controls, limitations and reproduction

```sh
cargo test -p overhead-render candidate_spike_retains_unknown_and_crossing_segments
cargo test -p overhead-render --release coast_candidate_spike -- --ignored --nocapture > /tmp/coast-spike.txt 2>&1
python3 docs/evaluations/m4-coastline-candidate-spike/validate_spike.py /tmp/coast-spike.txt
```

The ordinary test covers cross-viewport strokes with outside endpoints, a wrapped
dateline stroke, limb crossing, polar geometry and nonzero-height bypass. The
ignored full probe checks every exported segment in isolation at the exact eleven
Stage A inputs, then checks the complete layer and each timed result. Neither
nearby strokes nor matching total writes alone can conceal a segment pixel loss
in those checks. The audit cross-checks old work counts and On-write controls,
new sample-count identities and both retained runs; it does not infer pixel
identity from the serial write counts.

This is parity with the existing simplified renderer, not independent geographic
completeness, all-view correctness, target branch identity or visual acceptance of
a changed appearance. There were **no newly observed gaps**. The existing
[0053](../decisions/0053-no-resolvable-coastline-omissions.md) known-gap rule,
[0054](../decisions/0054-first-executable-coastline-comparison.md) exceptional
failure treatment and [0055](../decisions/0055-defer-all-view-coastline-proof-for-m4.md)
deferred proof remain intact.

Workspace check/tests, renderer measurement-feature tests, app/render no-default
check and formatting passed. The new release probe passed twice; the old work
probe was rerun after sharing its scene helper and reproduced its retained counts.
Both host evidence audits and the existing S3 raw-log validator passed. No firmware
host/target test or hardware operation was needed for this test-only change.

## Next boundary

Stop tuning this expensive fixed-sampling path as a stand-alone rescue. A next
bounded **host-only representation/interpolation experiment** should test whether
precomputed endpoint geometry plus screen-scale-aware refinement can remove both
runtime conversion/trigonometry and fine sampling of already subpixel overview
curves. It must report total storage/code risks, isolated shape/pixel differences
at tight zoom and limb/viewport cases, and retained work on overview scenes. An
ECEF chord is **not** the currently accepted latitude/longitude curve; investigate
that difference explicitly rather than silently replacing it. No encoding,
precision, approximation tolerance or production replacement is selected here.

The failed Stage A candidate remains unmodified. Its target timing gate, selected
FrameOff cost, Sharp bring-up and whole-device budgets remain open; this host
experiment does not authorize a new target build or flash.
