# 0051 — Angular coastline appearance across the reviewed zooms

Date: 2026-10-19
Status: accepted (visual direction only; no data, raster contract or implementation)

## Decision
Pursue an angular, strictly 1-bit coastline appearance in the opt-in wide radar across the **reviewed 5°–100° vertical FOV range**, including a small geography cue on the tiny globe at the widest zoom. Local 5° coastline matters; do not impose a tight-zoom suppression, switch back to full line detail at 20°, or hide coastlines at wide FOV merely because they are small.

## Why
The user reviewed [a full versus no-coast reference](../mockups/0051-coastline-legibility-comparison.md), [5° local alternatives at two observers](../mockups/0052-local-coastline-comparison.md), [5°–20° progression](../mockups/0053-coastline-zoom-progression.md) and [40°–100° overview](../mockups/0054-coastline-overview-comparison.md). They liked the simplified angular treatment and said it was fine throughout, despite loss of full-detail coastline at 20° and a tiny texture at 100°. These were native-size offline composites over fresh headless simulator frames, **not** executable coastline rendering or physical-display acceptance.

## Consequences
- This settles an **appearance direction**, not Natural Earth 1:110m adoption, the `scalerank == 0` filter, 2.5px projected RDP, 0.2° interpolation, a runtime data format, exact raster/LOD rules or behavior below 5°. Those preview inputs remain provisional, with version/hash/licensing provenance in 0051's mock record. No new dependency or renderer/app/core change is approved.
- Keep [0026's fixed-camera WGS-84 limb and physical eligibility](0026-fixed-camera-perspective-slice.md), [0033's opt-in wide layout](0033-wide-radar-comparison-contract.md), [0048's notices](0048-refined-empty-view-notices.md), and [0049's selection](0049-stable-wide-selection.md) intact. Coastline geography must not silently introduce a filled mask or new satellite camera-occultation rule; the proposed draw order/shape details in the [unapproved rules proposal](../mockups/0051-coastline-rules-proposal.md) still need agreement.
- Before requesting implementation, scope a bounded contract for source/version and offline preprocessing, how the angular shape is represented at 0.1°–100°, exact front-face/tangent and viewport clipping, draw order and overlap, deterministic tests, storage and target-work budget. Obtain separate authorization to implement; host mock readability does not establish physical-panel legibility or S3 performance. Frozen M2 evidence is unchanged.
