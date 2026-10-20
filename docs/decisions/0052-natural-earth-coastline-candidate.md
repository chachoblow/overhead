# 0052 — Natural Earth coastline candidate for first bounded slice

Date: 2026-10-20
Status: accepted candidate source, conditional; originally not dataset bundling or implementation authorization; [0054](0054-first-executable-coastline-comparison.md) subsequently authorizes a checked host comparison export

## Decision

Use **Natural Earth 1:110m coastline v4.1.0** as the *candidate source* for a first bounded M4 coastline slice, subject to reproducible offline export-size/work evidence and fresh executable visual review. The [initial comparison](../mockups/0051-coastline-legibility-comparison.md#candidate-and-provenance) records the archive URL, declared version, WGS-84 projection, SHA-256 and public-domain terms; pin and reverify those inputs before any export. Do not silently substitute a later Natural Earth release.

## Why

The pinned archive supplied the reviewed [5°–100° angular comparisons](0051-angular-coastline-appearance-direction.md) and has recorded provenance, making it a reproducible first candidate. Those offline composites did not establish export size, clipping correctness, target cost or physical legibility; accepting the candidate avoids silently treating them as an adopted asset.

## Consequences

This answers the source-candidate question, not whether to ship an asset or implement the renderer. [0051's angular direction](0051-angular-coastline-appearance-direction.md) covers the reviewed 5°–100° appearance and the evidence-gated exploration down to 0.1°. The [contract proposal](../mockups/0055-coastline-contract-proposal.md) is still unapproved; no source filter, simplification tolerance, packed format, pixel rule, storage/work budget or tight-zoom detail is selected by this decision.

- Before adopting an exported asset, document the generator and provenance, confirm source terms/hash, report retained part/vertex/segment counts, bytes and checked maxima, and demonstrate bounded runtime work/stack against a separately stated target budget. The candidate archive's 134 polylines / 5,128 source vertices are **not** an exported-size or target-cost bound.
- Compare the proposed reduced source and exact geometry/layering in fresh **executable** 1-bit wide scenes at tight, local and overview zooms, with selected/empty cases and a no-coast control. Offline overlays cannot validate tangent/viewport clipping, foreground priority, physical-panel legibility or S3 performance.
- Keep the existing unfilled WGS-84 limb, true observer-horizon satellite eligibility, notices, selection, strip and square/circular baseline unchanged. No new crate, runtime fetching, firmware work or frozen-M2 evidence renewal follows. Seek separate agreement on the remaining contract and explicit authorization before implementation. [0054](0054-first-executable-coastline-comparison.md) subsequently grants a bounded comparison authorization, not a production asset or device guarantee.
