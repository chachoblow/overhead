# Overhead — Handoff

_Last updated: 2026-10-19 UTC. Next: draft an M4 coastline contract for review; no implementation by default._

## State
[M3 complete](PLAN.md#m3--minimal-real-data-radar-complete). Opt-in wide radar
implements [0048's empty-view notices](decisions/0048-refined-empty-view-notices.md)
and [0049's stable selection](decisions/0049-stable-wide-selection.md) with
current true-state strip/debug values off-screen; square/circular remain
visible-only. [Interactive replay](../sim/README.md#wide-radar-comparison), not
a snapshot, proves continuity. [0050](decisions/0050-keep-range-in-debug-and-defer-display-prediction.md)
keeps range in debug and defers display prediction; the pass engine/tool remains.

## This session
- On `main` from `f03bf3e`, evaluated provisional Natural Earth 1:110m v4.1.0
  offline. [Source/hash/licence and initial 1-bit comparison](mockups/0051-coastline-legibility-comparison.md),
  [5° local](mockups/0052-local-coastline-comparison.md), [5°–20°](mockups/0053-coastline-zoom-progression.md)
  and [40°–100°](mockups/0054-coastline-overview-comparison.md) retain selected/
  empty and full/simplified/no-coast scenes at Colorado and San Francisco.
- User accepted [0051's angular appearance direction](decisions/0051-angular-coastline-appearance-direction.md)
  throughout the reviewed **5°–100° range**. Rank/2.5px RDP/0.2° sampling
  were illustrative only; source adoption, clipping/raster, below-5° behavior
  and implementation are unapproved. [Unapproved proposal](mockups/0051-coastline-rules-proposal.md)
  lists gates. PLAN coastline checkbox stays open; no Rust/firmware change.

## Next bounded step / limits
Propose a reviewable **coastline source/geometry/raster contract**: bounded
preprocessing, camera-facing tangent/viewport segment clipping, zoom below 5°,
layering and memory/work evidence. Ask separately before implementation; preserve
observer-horizon eligibility, limb, notices, selection and square baseline.
Do not reopen 0050 or treat static composites as executable output. M2 budgets/
scheduling, target RAM/safe stack, Sharp output and RWX remain open; keep
[0023's frozen evidence](decisions/0023-frozen-m2-revision-isolation.md) isolated.

## Verification / limits
Docs/PNG-only diff: rebuilt simulator and generated headless historical sources;
retained 1-bit overlays are **not** renderer output or physical validation.
Wrap-up checked binary cells, links, whitespace and staged diff; Rust check/tests
not rerun (no code/build inputs changed). No interactive replay, hardware,
firmware or frozen-M2 capture this session.
