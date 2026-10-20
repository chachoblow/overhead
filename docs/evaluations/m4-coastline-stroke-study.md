# M4 coastline — offline clipped-stroke raster experiment

Date: 2026-10-20. **Diagnostic only**, following the [sampling study](m4-coastline-sampling-study.md) and [0057 comparison](../mockups/0057-coastline-sampling-comparison.md). No raster, sampler, 40 km reduction, packed asset, tight-zoom policy or renderer implementation is approved. The [native 1-bit comparison](../mockups/0058-coastline-stroke-comparison.md) is an offline overlay, not executable coastline output.

## Question and method

Can the observed sampling-dependent ink be reduced without increasing geographic samples? `tools/offline/coastline_stroke_study.py` reuses the **same pinned** Natural Earth 1:110m v4.1.0 source, unquantized 40 km ECEF-reduced parts, coarse-frustum cull, 0.2°/0.05°/0.01° geodetic-linear surface interpolation, front-face bisection and normalized viewport segment clipping as 0057. `coastline_compare.draw` now optionally records the clipped continuous screen segments **before** its existing Pillow integer-endpoint line raster. That original path remains the 0057 reference. The experiment tests each integer pixel center inside `x=3..349, y=3..237` for distance ≤0.5 pixel from any continuous clipped segment; the union of these radius-0.5px capsules is its 1-bit stroke mask. It does not repeatedly round the ends of every short stroke. No supersampling, blur, postprocessing, fill, runtime renderer or new dependency is involved.

| San Francisco FOV | 0057 Pillow mask 0.2° ↔ 0.01° | Clipped stroke mask 0.2° ↔ 0.01° | Stroke 0.05° ↔ 0.01° | Stroke ink at 0.2° / 0.01° | Eligible intervals at 0.2° / 0.01° |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0.1° | 108 | **6** | 1 | 268 / 268 | 106 / 2,083 |
| 1° | 120 | **0** | 0 | 299 / 299 | 190 / 3,702 |
| 5° | 287 | **0** | 0 | 1,724 / 1,724 | 9,198 / 177,943 |

Differences count the symmetric difference of **coastline masks**, not of the finished composites. The stroke rule is invariant under splitting a straight line at an arbitrary interior point in its synthetic test; an exactly horizontal, two-off-screen-endpoint viewport crossing retains 347 pixels at all three steps. The near-surface-tangent control retains one pixel and one sign-changing tangent at all three steps. A separate one-interval, both-hidden-endpoints case recovers the interior visible run with two tangent roots; the sampled scene also retains it. Tests check strict radar bounds and no ink from a fully off-radar segment. These cases do not establish arbitrary-curve or limb-junction correctness.

The six remaining changed 0.1° pixel centers have coarse/fine distances to the stroke ranging roughly **0.491–0.509 px**: each crosses the 0.5px coverage boundary as the *approximated projected curve* moves by less than ~0.01px at those sites. No tangent crossings were counted in these three San Francisco scenes. Much of the 0057 sampling difference is therefore consistent with repeated integer endpoint rounding/short-stroke raster artifacts; the six residual pixels show that a coverage threshold can still flip under very small geometric changes. Equal masks do **not** prove accurate tangent crossings or the absence of a missed thin visible run. The stroke rule has a different texture even when 0057 masks agree; e.g. at San Francisco 5°, it inks 1,724 pixels versus the 0057 0.2°/0.01° masks' 1,850/2,135. Visual preference needs review rather than an assumption of convergence.

As a limited numerical sensitivity probe, rounding **only final projected screen endpoints** to IEEE f32 changes zero pixels in these six cases. This does not emulate f32 geodetic/ECEF/camera calculations, prove a target-float margin, bound the geodetic curve or quantify source reduction/vertex encoding error. Bounding-box pixel-candidate counts and clipped segments are in the temporary report; e.g. San Francisco 5° at 0.2°: 5,872 candidate pixel tests and 3,262 clipped segments, versus 64,659 and 62,377 at 0.01°. These are host algorithmic counts, not S3 time, stack, flash or a production budget. The conservative cull and tangent solver remain exploratory.

## Reproduce and limits

Using the [validated temporary archive](m4-coastline-source-preflight.md#reproduction-and-input) and optional [0056 source frames](m4-coastline-reduction-study.md#reproduction-checks-and-next-gate):

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/offline/coastline_stroke_study.py \
  /tmp/overhead-coastline-preflight/ne_110m_coastline.zip \
  /tmp/overhead-coastline-comparison/stroke-study \
  --frames /tmp/overhead-coastline-comparison/frames
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools/offline -p 'test_coastline_compare.py'
```

The temporary `report.json` SHA-256 is `a6eaeca4817356e207096d0d653dc70969709f2abae651df8bcea547ac4a60de`; retained sheet SHA-256 is `69557ae7c3b42f9fa52dbf24047cd4157e14c001d7b586173f834faf9b7117ee`. Eleven offline tests and `cargo check --workspace` passed. No interactive simulator, fresh simulator build, hardware, firmware or frozen-M2 capture was run. Optional finished-frame OR overlays preserve source black ink and **cannot** validate background/foreground layering, notices, selected overlaps or executable legibility.

**Remaining gate:** a [follow-on trial-coordinate study](m4-coastline-coordinate-precision-study.md) bounds only corresponding on-screen point displacement from coordinate rounding, not surface-curve, tangent, viewport or target-float error. Independently bound those remaining errors (including nearly grazing visibility and antimeridian/polar cases), then compare an explicit proposed export and pixel rule against the 0051 appearance and a stated device-work budget. Separate authorization is required before adopting any coastline asset or renderer path.
