# Overhead — Handoff

_Last updated: 2026-10-20 UTC. Next: compare pixel-stable coastline strokes offline; no renderer implementation by default._

## State
[M3 complete](PLAN.md#m3--minimal-real-data-radar-complete). M4's
[0048 notices](decisions/0048-refined-empty-view-notices.md) and
[0049 wide selection](decisions/0049-stable-wide-selection.md) are implemented
and visually accepted; square/circular stay visible-only. Selection continuity
needs [interactive replay](../sim/README.md#wide-radar-comparison).
[0050](decisions/0050-keep-range-in-debug-and-defer-display-prediction.md) defers display prediction; range stays in debug. **No coastline runtime exists.**

## This session
- [0051](decisions/0051-angular-coastline-appearance-direction.md) accepts the
  angular **5°–100° appearance**; below 5° is evidence-gated exploration.
  [0052](decisions/0052-natural-earth-coastline-candidate.md) accepts pinned Natural Earth 1:110m v4.1.0 as a **conditional candidate**, not an asset.
  [Contract proposal](mockups/0055-coastline-contract-proposal.md) and implementation remain unapproved.
- [Offline tools/usage](../tools/README.md#offline-m4-coastline-source-preflight)
  and [source](evaluations/m4-coastline-source-preflight.md), [reduction](evaluations/m4-coastline-reduction-study.md), [40 km](evaluations/m4-coastline-40km-refinement.md) studies check provenance,
  trial bytes and sampled clipping/culling. User chose **40 km for study only**,
  not an export tolerance. [0056 sheets](mockups/0056-coastline-reduction-comparison.md)
  are offline overlays over headless source frames.
- [Sampling study](evaluations/m4-coastline-sampling-study.md) and [0057 sheet](mockups/0057-coastline-sampling-comparison.md)
  show 0.2° vs 0.01° changes 108 mask pixels at San Francisco 0.1° despite
  <0.011px *eligible-point* chord deviation. Finer samples cost more and alter
  raster ink without proving curve/tangent accuracy.

## Next / limits
Compare one **offline pixel-stable clipped-stroke raster** with 0057 and
synthetic tangent/viewport-edge cases; distinguish aliasing from missed visible
runs and target-float error. No sampler, asset, budget, tight-zoom appearance or
implementation is approved. Preserve horizon, limb, notices, selection and
square baseline. M2 budgets/scheduling, Sharp, RAM/safe stack and RWX remain
open; [frozen M2 evidence](decisions/0023-frozen-m2-revision-isolation.md) is isolated.

## Verification
Fresh workspace check/tests and eight Python tests passed; archive/report hashes,
1-bit sheet bounds, links and whitespace checked. Offline OR composites are
**not executable coastlines** or physical/S3 evidence. No interactive replay,
hardware, firmware or frozen-M2 capture.
