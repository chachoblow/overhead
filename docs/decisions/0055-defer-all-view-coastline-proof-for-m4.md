# 0055 — Stop all-view coastline proof work for this M4 iteration

Date: 2026-10-22
Status: accepted

## Decision

The user has reviewed the executable 40 km coastline at different zooms and accepts its appearance for the current wide-view comparison. The accumulated host checks found no qualifying screen-relevant gap in their tested scenes, including isolated short runs and all exported segments at seven fixed scenes. **Do not continue an all-observer/all-zoom numerical proof as a prerequisite for moving this M4 comparison forward.** Retain that investigation as a later TODO, not a completed theorem or a reason to re-run incremental proof studies each session.

[0053](0053-no-resolvable-coastline-omissions.md)'s product intent still stands: a known, ordinary resolvable gap or whole-layer dropout is a bug to investigate and fix, not an accepted appearance or a reason to classify uncertain geography as invisible. [0054](0054-first-executable-coastline-comparison.md)'s exceptional `COAST ERROR` treatment does not become a routine fallback. The accepted 40 km simplification does not promise pixel-perfect Natural Earth geography. A newly reproduced visible omission should be recorded by observer, FOV and segment and addressed on its merits.

## Why

The [bounded host study](../evaluations/m4-coastline-bounded-segment-study.md) found no qualifying gap in the checked scenes. Its conditional arithmetic audit encloses the stored lens coefficient, but not the camera evaluation; its 256 bins do not certify all views. The user has already accepted the executable appearance across zooms. Pursuing an all-view theorem through more small host studies has not been proportionate to this orientation-graphic iteration. Tested scenes and visual review provide practical confidence, **not** exhaustive completeness or source-geography accuracy.

## Consequences

This changes the *iteration gate*, not the measured result or production renderer. The later proof TODO remains open; a newly observed ordinary gap must still be fixed. Move on to separately authorized target resource measurement and physical-display assessment before adopting a device coastline. The [host cost preflight](../evaluations/m4-coastline-host-cost-preflight.md) and [provisional budget gates](../evaluations/m4-coastline-budget-preflight.md) are not S3 headroom or Sharp throughput evidence. Do not build/flash or access hardware without the required permission, and do not refresh frozen M2 evidence to claim current coastline performance. If target evidence reveals a problem, revisit the candidate rather than weakening the known-gap rule.
