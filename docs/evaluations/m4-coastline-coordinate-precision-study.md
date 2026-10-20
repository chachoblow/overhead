# M4 coastline — offline trial-coordinate precision at tight zoom

Date: 2026-10-20. **Study only** after the [clipped-stroke experiment](m4-coastline-stroke-study.md). The accepted [Natural Earth candidate](../decisions/0052-natural-earth-coastline-candidate.md), 40 km ECEF reduction and `i32` 1e-4° coordinate layout remain **unapproved as an export or renderer rule**. No coastline runs in app/render.

## Method and restricted bound

`tools/offline/coastline_precision_study.py` pins/verifies the Natural Earth 1:110m v4.1.0 archive, reduces to the same unquantized 40 km study curve, encodes it with the earlier **trial** 1e-4° lon/lat + part offsets, and decodes the *actual bytes*. It rejects trial coordinates if rounding changes any segment's shortest-longitude interpolation branch. For both decoded and unquantized parts, it generates 0.2° geographic-linear surface intervals, tangent/viewport clips and applies [0058's continuous screen-segment pixel-center stroke](m4-coastline-stroke-study.md). It checks that the coarse-frustum cull changes neither representation's stroke mask against a full scan at each tested scene. Quantized and unquantized strokes are then compared as independent masks (not OR overlays). No new sampler, export or dependency is selected.

A **real-arithmetic, corresponding-point** bound can be given for this trial's geographic-linear surface path: each coordinate error is ≤0.00005°, so each interpolated lon/lat pair differs by at most `sqrt(2) × 0.00005°` (shortest-longitude paths assumed consistent); WGS-84 surface-coordinate speed is <6,500 km/rad, yielding **≤0.008022 km** ECEF displacement. For this fixed 75,000 km camera and the tested site heights ≤2.187 km, conservative depth is ≥`75,000 − 2×6378.137 − 2.187 = 62,241.539 km`. At 0.1° vertical FOV, for a corresponding *unquantized point inside the 173×117 wide viewport*, the pinhole quotient gives **≤0.01731 px displacement in either screen axis** (`117/tan(0.05°) × δ/(depth−δ) × [1+(173/117)tan(0.05°)]`). This is a deliberately loose upper bound for **pointwise coordinate quantization only**, not pixel-mask equality; a stroke at the 0.5px coverage threshold may change with arbitrarily small displacement. A synthetic test demonstrates that threshold flip and checks point displacement at San Francisco, 89.9°N and the dateline. It is **not** a bound for source simplification, tangent/root location, screen position of a clipped viewport intersection, missed visible runs, or floating-point computation on S3. The tested coast can also have no pixels at a site; that is not positive tight-zoom evidence.

| Site / FOV | Trial-coordinate stroke differences | 0.2° intervals after cull | Unquantized / decoded tangent roots |
| --- | ---: | ---: | ---: |
| Colorado 0.1°, 5° | 0, 0 | 44, 10,360 | 0/0, 1/1 |
| San Francisco 0.1°, 1°, 5° | **3**, 0, 0 | 106, 190, 9,198 | 0/0 at all three |
| Equator/dateline 0.1°, 5° | 0, 0 | 0, 5,445 | 0/0, 1/1 |
| 89.9°N 0.1°, 5° | 0, 0 | 1,577, 16,048 | 0/0, 1/1 |

All nine full-scan versus culled stroke masks agree separately for unquantized and decoded geometry. The three changed San Francisco 0.1° pixel centers are `(42,12)`, `(158,219)` and `(162,224)`; their nearest-stroke distances move across 0.5px by ~0.002–0.0034px. Previous Pillow endpoint-rounding comparison reported **zero** trial-coordinate differences at San Francisco 0.1°; that is not a contradiction—raster policies differ, and neither mask is certified. Unchanged root *counts* do not verify root *locations* or a nearly tangent interior visible interval. At equator/dateline 0.1° no intervals are admitted, so zero differences there cannot test tight-zoom geography. The trial is 15,484 bytes, 1,868 vertices, 134 parts, SHA-256 `951ed542a445201c24241bfb263b81567cef20641854be281778788a7a7a6796` (same as the earlier [40 km diagnostic](m4-coastline-40km-refinement.md)). No packed asset was committed. The interval counts are host work proxies, not S3 performance or safe-stack limits.

## Reproduction and next gate

Using the previously verified [temporary source archive](m4-coastline-source-preflight.md#reproduction-and-input):

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/offline/coastline_precision_study.py \
  /tmp/overhead-coastline-preflight/ne_110m_coastline.zip \
  /tmp/overhead-coastline-comparison/precision-study
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools/offline -p 'test_coastline_compare.py'
```

Temporary `report.json` SHA-256 `96e3b26d9925fa3490c9544a11bcfed7bf7aa8fbf8106b2d84563a607c311849` retains per-case pixel coordinates/distances, admitted segments, intervals, clipped strokes and pixel-candidate work; no image composite is needed to see three changed pixels. The output remains in `/tmp`, not a renderer fixture. Thirteen offline tests and workspace check passed; no interactive replay, hardware, firmware or frozen M2 capture. **Next:** the [synthetic visibility-enclosure stress](m4-coastline-visibility-bound-study.md) finds that a first-derivative front-face bound can exhaust its finite budget on an exact graze; it does not yet bound viewport-curve raster error. Improve the tangent/viewport failure bound, then quantify end-to-end target-float behavior separately. Neither this pointwise bound nor unchanged masks justify authorizing the export or renderer.
