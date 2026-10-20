# M4 coastline — 40 km sampled-curve convergence study

Date: 2026-10-20. **Offline diagnostic only.** The user chose **40 km for further study**, not as an accepted export tolerance. This follows the [40 km encoding/culling preflight](m4-coastline-40km-refinement.md) and uses the same pinned Natural Earth 1:110m v4.1.0 archive. [Native San Francisco comparisons](../mockups/0057-coastline-sampling-comparison.md) are offline OR composites, **not** executable coastline output or physical-panel evidence.

## Method and scope

`tools/offline/coastline_sampling_study.py` keeps the same **unquantized** 40 km ECEF-reduced source parts and same conservative coarse-frustum cull at each FOV. For admitted source segments it compares geodetic-linear surface interpolation at maximum geographic step **0.2°, 0.05° and 0.01°**, using the existing 32-iteration sign-crossing tangent bisection, normalized wide-viewport segment clipping before pixel rounding, and Pillow 1px line raster. Finer sampling is a *reference trial*, **not truth**: all three sample the same 40 km simplified surface curve, not the lost Natural Earth vertices; no analytic front-face/viewport error bound is proved. Detailed algorithm and assumptions live in the [reduction study](m4-coastline-reduction-study.md) and [culling study](m4-coastline-40km-refinement.md). No runtime renderer, export or dependency changes.

Four observer sites — Colorado `39.007 -104.883 2.187`, San Francisco `37.7749 -122.4194 0`, equator/dateline `0 179.9 0`, near pole `89.9 0 0` — are sampled at **0.1°, 1° and/or 5°**. The tool also evaluates a synthetic line with both endpoints outside a 0.1° viewport and a sign-crossing surface-tangent segment at equatorial 20°. For geometry context only, it samples 19 evenly spaced interior points in each ≤0.2° *coarse* geographic interval with both coarse endpoints camera-facing, and reports maximum **pixel-scaled point-to-projected-chord distance** for fine points that are also front-facing and in the viewport. That is a measured deviation at these points, **not** a bound for all positions, tangent intervals, renderer raster or source reduction. Rows with no eligible fine points report no deviation measurement, not zero error.

## Measured results

Pixel differences below are the symmetric difference between two **offline 1-bit coastline masks**. A smaller difference from a finer run does not imply that raster pixels have converged.

| Site / FOV | 0.2° → 0.01° intervals after coarse cull | 0.2° ↔ 0.01° mask pixels different | 0.05° ↔ 0.01° different | Max sampled chord deviation | Eligible fine points |
| --- | ---: | ---: | ---: | ---: | ---: |
| Colorado / 0.1°, 1° | 44 → 873; 150 → 2,930 | 0; 0 | 0; 0 | none; none | 0; 0 |
| Colorado / 5° | 10,360 → 199,949 | 308 | 66 | 0.00037 px | 73,353 |
| San Francisco / 0.1° | 106 → 2,083 | 108 | 140 | 0.01050 px | 139 |
| San Francisco / 1° | 190 → 3,702 | 120 | 77 | 0.00107 px | 1,596 |
| San Francisco / 5° | 9,198 → 177,943 | 287 | 55 | 0.00037 px | 61,744 |
| 89.9°N / 0.1° | 1,577 → 31,294 | 0 | 0 | none | 0 |
| 89.9°N / 5° | 16,048 → 311,354 | 399 | 77 | 0.00038 px | 190,043 |
| Equator/dateline / 0.1° | 0 → 0 | 0 | 0 | none | 0 |
| Equator/dateline / 5° | 5,445 → 104,574 | 164 | 27 | 0.00034 px | 24,021 |

The **San Francisco 0.1° masks differ by 108 pixels even though measured projected chord deviation is only 0.011 px** in 139 eligible fine samples; 0.05° versus 0.01° differs by *more* (140 pixels). At San Francisco 5° and Colorado 5°, the 0.01° raster has more black ink (2,135 vs 1,850, and 2,513 vs 2,205) despite sub-0.001px measured chord deviations. The likely cause is **repeated rounding and line rasterization of many short strokes**; this comparison alone does not attribute every changed pixel or certify the analytic shape. Finer geographic samples can therefore increase ink/work and alter 1-bit texture rather than reliably increasing legibility. The [native sheet](../mockups/0057-coastline-sampling-comparison.md) is needed to judge that change. No sampling step or rasterization algorithm is approved.

The synthetic **two-offscreen-endpoint** surface segment produces 347 pixels at 0.2°, 0.05° and 0.01°, with no difference; viewport *segment* clipping is exercised. The near-tangent control reports one sign-changing crossing and one coast pixel at all three steps, with no difference. These narrow controls do **not** prove tangent correctness for arbitrary curves, both-back-facing intervals with an interior visible run, strict physical camera limits or tolerance at a pixel-boundary grazing contact. In several 5° scenes thousands of coarse intervals have at least one back-facing endpoint; the chord metric intentionally excludes them. The 0.1° Colorado and polar views contain no visible qualifying coast points, so their zero mask differences are not positive tight-zoom accuracy evidence.

## Reproduction and next gate

Using the [same verified temporary archive](m4-coastline-source-preflight.md#reproduction-and-input) and optional [0056 headless source frames](m4-coastline-reduction-study.md#reproduction-checks-and-next-gate):

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/offline/coastline_sampling_study.py \
  /tmp/overhead-coastline-preflight/ne_110m_coastline.zip \
  /tmp/overhead-coastline-comparison/sampling-40 \
  --frames /tmp/overhead-coastline-comparison/frames
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools/offline -p 'test_coastline_compare.py'
```

`report.json` SHA-256 `cac19f876909962ec4bb13eac8575877187027fc3977f24c475065c66027d1c2`; sheet SHA-256 `3eb34830a5e60397b3b2bdb558bac3d87d212b6fabc07c9b2806ae5ecf7b2226`. Output is temporary; only the native [0057 1-bit sheet](../mockups/0057-coastline-sampling-comparison.md) is retained. Eight Python geometry/preflight tests pass, including synthetic viewport/tangent sampling; `cargo check --workspace` passed. No new simulator build/capture, interactive replay, hardware, firmware or frozen M2 capture for this follow-up (0056's fresh headless frames were reused).

**Gate still open:** agree a pixel-stable line/raster strategy *and* a demonstrable surface-curve/tangent error bound that avoids disappearing front-facing runs at 0.1°; test quantized vertices and target-float margin with that strategy. A smaller geographic step alone neither closes pixel convergence nor justifies its up-to-~20× subdivision cost at a 5° scene. Obtain separate approval for any renderer/asset implementation and actual S3 timing before choosing a device-work budget.
