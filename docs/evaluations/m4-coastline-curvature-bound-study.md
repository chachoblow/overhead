# M4 coastline — offline second-derivative enclosure comparison

Date: 2026-10-20. **Synthetic study only**, following the [first-derivative failure case](m4-coastline-visibility-bound-study.md). No source reduction, sampler, coastline raster, asset, target budget or renderer rule is selected. The 0051 angular appearance and 0052 conditional Natural Earth candidate remain unchanged.

## Stronger real-arithmetic bounds, restricted meaning

`tools/offline/coastline_curvature_study.py` compares the original front-face derivative enclosure to an analytic **second-derivative** enclosure on the same synthetic WGS-84 geodetic-linear intervals. If `f = outward_normal · (camera − surface)` and `t∈[0,1]`, the midpoint Taylor enclosure is `|f(t) − f(1/2) − f'(1/2)(t−1/2)| ≤ M(t−1/2)²/2`, so `|f'(1/2)|/2 + M/8` encloses its total variation. Here angular parameter speed is `w` radians, `|normal'|≤w`, `|normal''|≤2w²`, `|surface'|<6500w km`, and `|surface''|<15000w² km`, giving `M = [2(75000 + 2×6378.137 + |site height|) + 2×6500 + 15000]w² km`. The surface acceleration bound follows from `N=a/sqrt(1−e²sin²(latitude))`, with `N<6400 km`, `|N'|<22 km/rad`, `|N''|<44 km/rad²`; the ellipsoid-position second derivative is then <15,000 km per angular-speed squared. `front_slope` evaluates the analytic derivative at the midpoint. The previous sphere/frustum cull and **fail-closed unknown** policy remain unchanged.

The synthetic wide exact graze (both endpoints behind the camera-facing limb, analytic tangent at its midpoint) is the critical comparison:

| Enclosure | Nodes / cap | Back-certified leaves | Unknown leaves | Total unknown arc | Widest unknown | Budget-exhausted unknown |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| First derivative | 8,191 / 8,191 | 4,094 | 4 | 0.175° | 0.1° | 4 |
| Second derivative | **767 / 8,191** | 50 | 334 | **0.00204°** | **0.00000611°** | **0** |

The second bound is substantially tighter but **does not decide the graze**: 334 tiny intervals stay unknown and must not be treated as hidden, and the absence of ink from the old 0.2° sampled control is not ground truth. A deliberately forced one-node second-order run still retains both unfinished intervals as unknown. At a sign-changing equatorial tangent both methods use 63 nodes and retain two intervals ≤0.00000916°; the narrow exact graze remains unknown. The both-hidden-endpoints/visible-middle, two-outside-viewport, polar dateline and front-only controls remain bounded without exhausting work. No archive-wide worst-case work or source/quantized curve coverage has been established.

An independent **point-to-corresponding-straight-chord** screen bound restricts projected curvature on a whole WGS-84 surface interval of geographic span `w`. For tested site heights ≤2.187 km, let `d=75000−2×6378.137−|height|>1`, `Q=(2×6378.137+|height|)/d`, and `S=117/tan(FOV/2)` pixels. Because the whole Earth is beyond this conservative depth, second differentiation of the perspective quotient bounds each screen-coordinate interpolation error by `S(1+Q)(15000+2×6500²/d)w²/(8d)`. For a **0.2°** surface interval at the minimum **0.1° FOV**, this is **≤0.06467 px per screen axis** in real arithmetic, whether the curve is inside or outside the viewport. The synthetic front-only 0.2° control has sampled maximum ~0.00000489 px, well below this loose bound. This does **not** bound the movement of a tangent/root, the position of a nearly grazing viewport *intersection*, coverage of a tiny run outside the sampled intervals, vertex quantization, 40 km source reduction, or pixel-mask equality. The old 0.2° trial's 23,377 intervals and 40 km retained source are *not* promoted to runtime inputs.

These are mathematical inequalities evaluated using **host f64 trigonometry and dot products**. Neither the sign slack nor the cull's 0.1 km pad is a certified roundoff margin; no S3 float path, exhaustive pole/dateline/near-plane proof, target stack/time or framebuffer layering was measured. The tests sample derivative/Taylor and chord enclosures at selected synthetic curves; sampled agreement cannot turn host computation into verified interval arithmetic. A small geometric chord bound is not a finite **raster/visibility guarantee** when the projected curve grazes a pixel or viewport boundary.

## Reproduction and remaining gate

No archive, simulator scene or hardware is used:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/offline/coastline_curvature_study.py \
  /tmp/overhead-coastline-comparison/curvature-study
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools/offline -p 'test_coastline_compare.py'
```

Temporary `report.json` SHA-256 `10f468e875c0933568d3d157b9458192fd130a2600207cd5e8e001d7b0f448cd`; 17 offline tests and workspace check/tests passed. No interactive replay, executable coastline, firmware, physical display or frozen-M2 capture. **Next gate:** the [source-wide work proxy](m4-coastline-enclosure-work-study.md) measures candidate enclosure nodes and unknown intervals at four sites, without selecting capacity or a policy. Define explicit behavior for remaining unknown tangent/viewport intervals, demonstrate an independently bounded target-float/rounding and cull margin, then measure full projection/raster work on candidate curves against a separately stated device budget. Do not infer asset or implementation authorization from the reduced synthetic work.
