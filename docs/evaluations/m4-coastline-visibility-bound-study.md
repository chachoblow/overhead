# M4 coastline — offline front-face and frustum enclosure stress

Date: 2026-10-20. **Synthetic diagnostic only**, following the [clipped-stroke](m4-coastline-stroke-study.md) and [trial-coordinate precision](m4-coastline-coordinate-precision-study.md) studies. No exported coastline, renderer geometry, sampling/LOD rule, target-float implementation or work budget is approved.

## Bounded experiment

`tools/offline/coastline_visibility_study.py` tests *geographic-linear WGS-84 surface intervals*, not a source coastline. In **exact real arithmetic** the surface normal's rate is at most the angular parameter speed, WGS-84 surface-coordinate speed is <6,500 km/rad, and `|camera − surface| ≤ 75,000 + 2×6378.137 + |site height|` km. For front-facing signed value `f = outward_normal · (camera − surface)`, the derivative bound is `(75,000 + 2×6378.137 + |site height| + 6,500) × angular_span_rad` km per unit interpolation parameter. The value at an interval midpoint therefore encloses the interval within ±half that bound: an interval proven positive/negative may be called front/back. Before that test, the existing conservative sphere/frustum candidate cull can prove an entire curve interval outside the viewport, including when both endpoints are outside. Anything else is bisected. At ≤`0.00001°` interval span it remains **unknown**, not invisible. A hard **8,191-node** limit retains all unfinished intervals as unknown; no silent truncation or false empty sky follows from budget exhaustion.

This describes the inequalities in real arithmetic, **not a certified f64 interval algorithm**. The study evaluates trigonometry, dot products, culling and splitting with host doubles; its `1e-6 km` front-sign slack and the coarse cull's `0.1 km` sphere pad have no proved target-float rounding margin. Nor does front-face certification bound projected curvature, the error in a chord clipped to viewport edges, longitude-source simplification or near-plane calculation. A straight clipped segment from the old study is drawn only as a *control*, never a proof of the surface curve's exact raster. The experiment cannot authorize an asset or runtime clipping algorithm.

| Synthetic curve / site / FOV | Front / back / outside-frustum leaves | Unknown leaves / budget-unknown | Widest unknown interval | Existing sampled stroke |
| --- | ---: | ---: | ---: | ---: |
| Equator sign crossing at analytic tangent / 20° | 15 / 15 / 0 | 2 / 0 | 0.00000916° | 1px, one sampled tangent |
| Equator exact graze, both endpoints behind / 20° | 0 / 4,094 / 0 | 4 / **4** | **0.1°** | no ink, no sampled tangent |
| Very narrow exact graze / 20° | 0 / 0 / 0 | 2 / 0 | 0.00001° | no ink, no sampled tangent |
| Both endpoints behind, visible middle / 5° | 6 / 0 / 4 | 0 / 0 | — | 347px, two sampled tangents |
| Both endpoints beyond opposite viewport edges / 0.1° | 1 / 0 / 0 | 0 / 0 | — | 347px |
| Near-pole shortest-dateline interval / 0.1° | 1 / 0 / 0 | 0 / 0 | — | 1px |
| Far side / 5° | 0 / 1 / 0 | 0 / 0 | — | no ink |

The equatorial zero-height analytic tangent lies at `acos(6378.137 / (6378.137 + 75000))` longitude. The wide exact-graze test is deliberately hard: under this loose **first-derivative** bound, 8,191 nodes leave 0.175° total unclassified surface arc, with one unfinished interval as wide as 0.1°. Therefore this mechanism **cannot** justify a limited-work runtime coastline by treating every unknown as a drawable tangent interval; treating unknown as hidden could miss an arbitrarily grazing run. The old sampled raster's zero ink at both exact grazes does not prove that a touch or a very thin run was safely handled. Even where a sign crossing is narrowed, the test bounds only which surface intervals *might* straddle the front/back boundary, not the projected location or raster error of the root. One front-certified interval with both endpoints outside the 0.1° viewport demonstrates why endpoints alone cannot establish absence of coastline.

## Reproduction and gate

No source archive or simulator frame is needed for this synthetic study:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/offline/coastline_visibility_study.py \
  /tmp/overhead-coastline-comparison/visibility-study
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools/offline -p 'test_coastline_compare.py'
```

Temporary `report.json` SHA-256 `288b30fa46709a0b5842702995b6d84c8a48f8c2c09938b906ef7ae0f58d9c17` records signed endpoint/midpoint values, classified/unknown spans, work and illustrative old-stroke pixels. Fifteen offline tests cover sampled derivative enclosures, crossing/graze retention, hard-limit fail-closed behavior, coordinate quantization, raster splitting, edge crossings and source-trial decoding. Workspace check/tests also passed. No executable coastline, interactive replay, physical display, firmware or frozen M2 capture.

**Next gate:** the [second-derivative follow-up](m4-coastline-curvature-bound-study.md) tightens this synthetic exact graze and bounds point-to-projected-chord deviation, but still leaves unknown intervals. A finite work/failure policy, checked roundoff (including the target float path), viewport-intersection handling and measured source-wide worst cases remain open. This first-derivative probe exposes where prior 0.2° sampling and small pixel differences are insufficient; it does not settle a production error tolerance, packed format, target budget or implementation authorization.
