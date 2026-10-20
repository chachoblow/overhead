# M4 coastline — checked-width interval arithmetic feasibility

Date: 2026-10-20. The user chose an **offline checked fixed-point feasibility study** after the [unbounded-fraction narrow predicate control](m4-coastline-interval-predicate-feasibility.md). This tests a representation branch, **not** an approved asset, algorithm, renderer, numerical tolerance or device budget.

## Deliberately limited method

`tools/offline/coastline_fixed_interval_study.py` takes the **already enclosed** equatorial sine/cosine inputs and exact binary64 constants from the prior offline rational study. Those trig enclosures are quantized outward *offline* to signed integer endpoints with scale `2^Q`. Within the toy predicate, `+`, `−`, `×`, positive-denominator division and positive square root round outward and reject overflow; every result and intermediate algebraic product/dividend/square-root scale is checked against signed **i128**. Arbitrary-width rational arithmetic **still supplies the trig inputs and quantized constants**; it is explicitly outside the checked-width path, not a hidden runtime implementation. The script checks that both computed fixed-point front/cull intervals enclose the independent exact-rational controls. It does not model compiler/target i128 cost or different floating-point constants.

| Fractional bits `Q` | Two ±16-ULP tangent point signs | Four adjacent east-frustum point decisions at 0.1° | Observed limitation |
| ---: | --- | --- | --- |
| 32 | both unknown | all unknown | Very wide intervals; tangent front width ~1.89×10⁻⁵ km. |
| 40 | both unknown | all unknown | Front width ~7.40×10⁻⁸ km. |
| 48 | positive / negative | all unknown | Tangent front widths ~2.89×10⁻¹⁰ km; near the real point values, so this sample is not a general error budget. |
| 52 | positive / negative | all unknown | East-plane intervals still ~6.80×10⁻¹¹ km wide versus true magnitudes ~10⁻¹⁵ km. An undecided cull must be **kept/unknown**, never rejected. |
| 56 | positive / negative on the tangent controls | **checked overflow** on all four east-frustum controls | Higher precision cannot simply be dialed up with the same signed-i128 evaluation order. |

The east-frustum's true plane signs were separately resolved by the [exact-rational reference](m4-coastline-interval-predicate-feasibility.md); this intentionally cheaper checked-width model correctly refuses to resolve them at Q≤52. It cannot use an arbitrarily wide fallback silently. At Q56, it fails explicitly rather than overflowing and misclassifying the coastline. Both are **feasibility outcomes on two equatorial point formulas**, not production defaults. Latitude-dependent WGS-84 coordinates, camera trigonometry, geodetic surface intervals, four-plane culling, second-derivative bounds, projected viewport clipping, raster stroke caps, nonfinite/depth paths and antimeridian/poles remain unimplemented and **uncertified**. This is not even a candidate runtime path until trig intervals and their range reduction can be generated within finite checked target work and every intermediate is accounted for.

## Design implication

A bounded target design could stage coarse checked predicates and retain *unknown* at precision/work limits, but this sample cannot select a fixed scale or a worst-case limit. Reordering, wider intermediates, precomputed geometry or interval refinement could alter the Q56 failure and cost substantially. Before proposing any approach, prove error and overflow bounds for **all** supported observer/FOV/source inputs, specify a finite uncertain/over-budget policy, and measure actual coast-plus-radar latency/stack/asset bytes against an **agreed** M2-informed device budget. The accepted [5°–100° appearance](../decisions/0051-angular-coastline-appearance-direction.md) and conditional [Natural Earth candidate](../decisions/0052-natural-earth-coastline-candidate.md) remain unchanged; no target cost follows from host Python counts or the historical frozen M2 captures.

## Reproduction

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/offline/coastline_fixed_interval_study.py \
  /tmp/overhead-coastline-comparison/fixed-interval
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools/offline -p 'test_coastline_compare.py'
```

Temporary `report.json` SHA-256 `2da6bff5df6cac93e023408086f6822cba948b887f8490ddc60f22c9fbf168d0`; offline tests and workspace check/tests pass. No new crate, runtime code, interactive replay, firmware, hardware or frozen-M2 capture. **Next gating decision:** the [M2-informed coastline ceilings](m4-coastline-budget-preflight.md) are now accepted as **provisional measurement gates only**, not proven available resources. Broaden a *specific* checked-interval implementation proposal only after checking its numerical scope and those gates against a whole-device baseline; otherwise defer executable geography. This narrow trial alone cannot justify either an asset export or a renderer.
