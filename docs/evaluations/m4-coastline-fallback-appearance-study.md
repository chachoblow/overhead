# M4 coastline — fallback appearance and full trial raster-work proxy

Date: 2026-10-20. Continuation of [decoded local-gap and host-f64 evidence](m4-coastline-unknown-precision-study.md). The user chose to **investigate** local gaps with a bounded whole-geography fallback; this evaluates *illustrative* thresholds and work, **not an approved contract, renderer or exported asset**. See the [coast-only 1-bit six-tile comparison](../mockups/0059-coastline-fallback-diagnostic.md): none of its tiles is a device screenshot or a finished compositor frame.

## Trial

`tools/offline/coastline_fallback_appearance_study.py` uses the pinned Natural Earth candidate reduced by the **unapproved** 40 km trial, host-f64 second-order sign/cull enclosure, geographic-linear front sampling at ≤0.2°, projected-chord viewport test and experimental pixel-center stroke. It renders coast **only** on white. Where an illustrative fallback fires it clears **only the coast diagnostic tile**; the real limb, selected object, notices and other foreground layers are *not* represented or changed. No runtime policy or user-facing warning is chosen. The [trial stroke](m4-coastline-unknown-policy-study.md) can ink pixels adjacent to unknown arcs via endpoint caps; these are not certified empty raster gaps.

| Native 400×240 coast-only case | Enclosure nodes | Geographic unknown / total arc | Budget unknown | Front sample subintervals / pixel candidates | Coast ink: local-gap → example fallback |
| --- | ---: | ---: | ---: | ---: | ---: |
| Colorado 20° | 2,322 | 16 / 0.000113° | 0 | 13,056 / 14,582 | **980 → 0** at illustrative 0.0001° arc cutoff |
| San Francisco 20° | 2,096 | 10 / 0.0000758° | 0 | 12,585 / 14,030 | **890 → 890** at illustrative 0.001° cutoff |
| Synthetic center segment + sign-crossing horizon segment, 20°, **one-node per-segment cap** | 2 | 2 / 0.6° | 2 | 20 / 24 | **5 → 0** because work exhausted |

The Colorado comparison shows a trade-off: falling back at 0.0001° for *only 16 tiny uncertain horizon arcs* removes **all 980 otherwise drawn coast pixels**. This is an especially poor policy candidate visually, not proof the 0.001° example is correct everywhere. The synthetic mixed case shows the fallback genuinely erases independently certified coast; a test containing *only* an ambiguous tangent could produce blank geography with or without a fallback and would not exercise that distinction. The six-tile picture excludes foreground, so it cannot establish legibility, eclipse/limb junctions, layering, notices or panel appearance.

Across the [20 paired site×zoom scenes](m4-coastline-unknown-precision-study.md), maximum observed **front geographic subintervals = 15,806** and **stroke pixel-distance candidates = 17,857** (unquantized polar 20°). Maximum enclosure nodes = 2,432 and unknown leaves = 18 (dateline 20°/100°). These are host operation *counts* over four sites and five FOVs, not global maxima, target work/memory/cadence, full project state, compiled storage, a display refresh bound or a separately stated device budget. Raster work is far larger than enclosure work: San Francisco 100° still samples 12,585 front pieces for 82 coast pixels. Optimizing/decimating these without changing masks or visibility would need separate evidence. The illustrative 0.001° arc fallback is not a worst-case maximum or an adopted limit.

## Reproduction / next gate

With the [previously verified temporary archive](m4-coastline-source-preflight.md#reproduction-and-input), from repo root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/offline/coastline_fallback_appearance_study.py \
  /tmp/overhead-coastline-preflight/ne_110m_coastline.zip \
  /tmp/overhead-coastline-comparison/fallback-appearance \
  --poster docs/mockups/0059-coastline-fallback-diagnostic.png
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools/offline -p 'test_coastline_compare.py'
```

Temporary report SHA-256 `9903a6e70edfecfb0a38d25cbcdcf07eb0be84ee7e8345fd76643648aba5dd01`; poster SHA-256 `b68597b498ebc08fe907964a1d74866a253afdfba7f90b30f78c81de7dc53147`. 21 offline tests and workspace check/tests pass. No interactive simulator, firmware, hardware or frozen-M2 capture.

**Subsequent decision:** [0053](../decisions/0053-no-resolvable-coastline-omissions.md) rejects local gaps or whole-layer dropout when the geography is resolvable; this study's earlier presentation question is historical, not still open for ordinary valid scenes. **Remaining gate:** the [narrow independent predicate control](m4-coastline-predicate-oracle-study.md) does not certify f64/libm error handling. Specify how to resolve screen-relevant uncertainty within finite work and separately decide genuinely exceptional failure presentation, then establish independently justified sign/cull/viewport error handling and pixel-stable curve clipping and a device budget before proposing a compiled asset/renderer. A source-wide scan of only four observer sites plus a synthetic stress case cannot select 0.001° or any other tolerance. Preserve the accepted 5°–100° appearance direction and conditional asset candidacy; do not infer runtime approval from this poster.
