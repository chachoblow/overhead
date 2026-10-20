# 0053 — Do not omit resolvable coastlines as normal behavior

Date: 2026-10-20
Status: accepted product constraint; implementation and exceptional failure policy remain open

## Decision

For the supported coastline input and view domain, a resolvable visible coastline must not acquire a local gap or disappear for a frame merely because an illustrative numeric-uncertainty or work threshold was reached. If a candidate renderer cannot resolve ordinary valid scenes within its finite limits, fix or replace it before shipping; do not adopt local-gap/whole-geography fallback as routine presentation. Retain unknown as unknown while investigating, never silently classify it as invisible.

## Why

The user wants coastlines, and explicitly rejected gaps or whole-layer dropout **if the geography is resolvable**. The [offline local-gap and two-pass trials](../evaluations/m4-coastline-atomic-fallback-study.md) expose how an incomplete or over-budget candidate can hide otherwise visible ink; they do not establish permission to ship that appearance. The [first executable-slice proposal](../mockups/0060-coastline-first-executable-slice-proposal.md) now treats such failures as a design-gate failure, not a selected threshold.

## Consequences and limits

- Keep [0051's angular appearance direction](0051-angular-coastline-appearance-direction.md) and [0052's conditional source candidate](0052-natural-earth-coastline-candidate.md). This decision does not select an asset, numeric method, raster, capacity, tighter-zoom detail, or a theorem for arbitrary input. Specify and validate the supported domain and bounded correct path before implementation authorization.
- A justified zero-screen-effect uncertainty is not a missing visible coastline; small *geographic span alone* cannot prove zero screen effect. An unknown visible fragment needs resolution or a revised candidate architecture, not a silent local gap.
- Behavior for truly invalid asset data or an irrecoverable unexpected runtime/physical-display failure is **unsettled**. Preserve Earth/marker/notice truth and request a separate presentation decision; this record does not approve a silent blank coast, a new notice or full-frame rollback design.
- [Provisional measurement gates](../evaluations/m4-coastline-budget-preflight.md) remain screening targets only. No executable coast, bundled asset, hardware work or measured device headroom follows.
