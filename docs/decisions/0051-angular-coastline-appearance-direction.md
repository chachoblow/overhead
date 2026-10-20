# 0051 — Angular coastline appearance across the reviewed zooms

Date: 2026-10-19
Status: accepted (5°–100° reviewed appearance; 0.1°–<5° exploration agreed on 2026-10-20, subject to evidence; no bundled data, raster contract or implementation)

## Decision
Pursue an angular, strictly 1-bit coastline appearance in the opt-in wide radar across the **reviewed 5°–100° vertical FOV range**, including a small geography cue on the tiny globe at the widest zoom. Local 5° coastline matters; do not impose a tight-zoom suppression, switch back to full line detail at 20°, or hide coastlines at wide FOV merely because they are small.

## Why
The user reviewed [a full versus no-coast reference](../mockups/0051-coastline-legibility-comparison.md), [5° local alternatives at two observers](../mockups/0052-local-coastline-comparison.md), [5°–20° progression](../mockups/0053-coastline-zoom-progression.md) and [40°–100° overview](../mockups/0054-coastline-overview-comparison.md). They liked the simplified angular treatment and said it was fine throughout, despite loss of full-detail coastline at 20° and a tiny texture at 100°. These were native-size offline composites over fresh headless simulator frames, **not** executable coastline rendering or physical-display acceptance.

## Follow-up — tighter zooms (2026-10-20)
The user agreed to **pursue the same angular treatment down to 0.1°**, subject to tight-zoom visual and work-budget evidence, rather than assuming a cutoff below 5°. This extends the *direction to investigate*, not the visually reviewed 5°–100° range: no native comparison below 5° has been accepted, and neither coastline visibility at every tight-zoom site nor a particular level of detail, sampling rule, source or implementation is approved. The [contract proposal](../mockups/0055-coastline-contract-proposal.md) identifies the remaining gates.

## Consequences
- This settles an **appearance direction**, not bundling [0052's conditional Natural Earth candidate](0052-natural-earth-coastline-candidate.md), the `scalerank == 0` filter, 2.5px projected RDP, 0.2° interpolation, a runtime data format, exact raster/LOD rules or precise below-5° behavior. Those preview inputs remain provisional, with version/hash/licensing provenance in 0051's mock record. At the time no new dependency or renderer/app/core change was approved; [0054](0054-first-executable-coastline-comparison.md) later authorizes a bounded executable comparison using the reviewed 40 km orientation look.
- Keep [0026's fixed-camera WGS-84 limb and physical eligibility](0026-fixed-camera-perspective-slice.md), [0033's opt-in wide layout](0033-wide-radar-comparison-contract.md), [0048's notices](0048-refined-empty-view-notices.md), and [0049's selection](0049-stable-wide-selection.md) intact. Coastline geography must not silently introduce a filled mask or new satellite camera-occultation rule; the proposed draw order/shape details in the [unapproved rules proposal](../mockups/0051-coastline-rules-proposal.md) still need agreement.
- Before requesting implementation, review the [bounded contract proposal](../mockups/0055-coastline-contract-proposal.md) for source/version and offline preprocessing, how the angular shape is represented at 0.1°–100°, exact front-face/tangent and viewport clipping, draw order and overlap, deterministic tests, storage and target-work budget. Obtain separate authorization to implement; host mock readability does not establish physical-panel legibility or S3 performance. Frozen M2 evidence is unchanged.
