# M4 coastline — bounded predicate strategy feasibility, narrow control

Date: 2026-10-20. The user selected **investigating** a bounded numerical-uncertainty strategy, not adopting an asset, tolerance, renderer or whole-frame fallback. This builds on the [high-precision but non-interval equatorial control](m4-coastline-predicate-oracle-study.md) without treating its host-f64 agreement as proof.

## A rigorously enclosed *limited* predicate

`tools/offline/coastline_interval_predicate_study.py` uses Python's exact rational fractions for **only** zero-altitude equatorial **point segments** at a zero-longitude observer and a separately supplied normalized viewport chord. It starts from the exact rational values of the host binary64 inputs and constants; this is a deliberately specified *real* geometry model, not the output of target `libm` or an interval around nominal decimal coordinates.

- Machin's identity `π = 16 atan(1/5) − 4 atan(1/239)` plus alternating-series **next-term bounds** encloses π (80 and 30 terms). Input angles are restricted to `[0, 11/7]` radians for this study.
- Midpoint sine/cosine Taylor polynomials through degree 48 use the Lagrange remainder `M^49/49!`; the uncertainty of the input angle is added using `|sin'|, |cos'| ≤ 1`. Interval operations use exact rational endpoint arithmetic.
- The positive square root is enclosed by integer-square-root floors at 70 decimal places, checking division denominators exclude zero. These operations enclose the equatorial front sign and positive-east frustum-plane expression in *real arithmetic* for the stated inputs. The independent [80-digit decimal probe](m4-coastline-predicate-oracle-study.md) lands inside one tested tangent interval and one tested cull interval; it is only a cross-check, not the enclosure's justification.

| Probe group | Interval result | Scale / warning |
| --- | --- | --- |
| Two ±16-ULP-input tangent points | One strictly front, one strictly back in the **real point** predicate; the earlier **segment** enclosure still retains both as unknown due to its finite sign slack | Each interval ~8.85×10⁻⁵⁰ km wide; this says nothing about a whole geographic interval's visibility. |
| Four adjacent positive-east coarse-plane points | Two strictly keep, two strictly reject; matches observed host decisions on these exact binary64 inputs | Intervals ~10⁻⁷¹ km wide at the edge. **Not** a certified Rust/libm/target plane operation. |
| Three supplied viewport-envelope x values | Two strictly overlapping the expanded box, one strictly outside; matches the host sign | Exact rational supplied-chord edge; **not** a WGS-84 curve or proof of clipped raster pixels. |

Even this tiny exact-rational **offline** prototype produces numerator/denominator sizes up to **108,915 bits** for one cull predicate. It is **not proposed as an S3 implementation** or a whole-Earth bounded-capacity result; no host duration is promoted to target cost. Very narrow intervals are possible because this example is restricted to analytic equatorial points. No proof exists here for WGS-84 normals/position at arbitrary latitudes, pole and dateline handling, the second-derivative sign enclosure, geographic reduction errors, minimum depth, projected-chord and pixel-cap bounds, decoded input range, target overflow/rounding or worst-case work. Near an exact root a valid interval method may **never decide sign** at finite precision; the policy must retain unknown and spend only a bounded amount of work.

## Candidate proof obligations before implementation can be proposed

1. **Define model and input domain:** say whether WGS-84 constants are exact decimal or exact binary64, what rounded trial coordinates represent, validated lat/lon/observer-altitude/FOV limits, and which geodetic-linear curve (including shortest-longitude branch) the renderer means. Treat nonfinite input/invalid depth as explicit failures, not hidden coast. An error bound for reduction to source geography is separate from the numerical predicate.
2. **Prove numerical enclosures end-to-end:** include angle conversions, trigonometry/range reduction for all supported latitudes/longitudes, observer and camera basis, WGS-84 radius/normal/square root, dot products, sign-slack and derivative/Taylor bounds, four frustum planes and padded curve radius, perspective division, chord envelope, and viewport intersection. Bind π/rational/truncation and target arithmetic error at every operation. Do **not** substitute `libm`'s qualitative “nearly rounded” comment or host sample margins for a bound. Checked fixed-point/outward-rational approximations might be possible but need proof of range/overflow, finite precision, stack/storage and work; this unlimited-precision Python implementation is **not** such a design.
3. **Retain a finite uncertainty outcome:** if any interval straddles front/cull/viewport zero, a division range touches zero, capacity is exceeded or the proof assumptions fail, mark it **unknown** rather than invisible. Do not bridge hidden/unknown parts. A curve approximation/stroke cap can ink adjacent unknown pixels; require an explicit raster definition if a pixel-empty gap is promised. Compare local blank geography and an agreed whole-geography fallback without changing tracking truth/notices/foreground.
4. **Measure the target path against a separately agreed budget:** use worst-case asset/frame intervals plus adversarial tangencies and viewport grazes, compiled bytes, RAM/stack and target timing/cadence. Existing [15,806 front steps / 17,857 pixel candidates in sampled scenes](m4-coastline-fallback-appearance-study.md) are host trial counts only. M2's frozen S3 captures exclude coastline and do not set its budget.

The user chose to [screen checked-width fixed-point arithmetic](m4-coastline-fixed-interval-feasibility.md): even with trig inputs pre-enclosed offline, its equatorial controls retain cull unknown at Q≤52 and overflow at Q56. An alternative proposal could prove a sufficiently conservative error bound for the **pinned** Rust `libm` and the entire evaluation graph rather than replace trig; it would still need items 1, 3 and 4. **No simple constant ULP multiplier has been established.** Choose a precise numerical representation and error proof *before* the source/export/raster contract, not by silently increasing trial slacks.

## Reproduce / status

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/offline/coastline_interval_predicate_study.py \
  /tmp/overhead-coastline-comparison/interval-predicates
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools/offline -p 'test_coastline_compare.py'
```

Temporary report SHA-256 `b2cacadc269ffbb010c9aa9cf1c47bb5ab1eca4a02d8bede778d4a3dc54e96fc`. Offline tests and workspace check/tests pass; no new crate, renderer, interactive replay, firmware, hardware or frozen-M2 capture. The study makes the remaining correctness/work gap **more specific**; it does not close the numerical or M4 adoption gate. Separately agree an engineering budget and an achievable proof scope before selecting a runtime path.
