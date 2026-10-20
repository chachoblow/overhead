# M4 coastline — decoded local-gap and host-f64 threshold preflight

Date: 2026-10-20. Follow-up to the [local-gap/fallback sensitivity study](m4-coastline-unknown-policy-study.md); **offline evidence only**, not an approved asset, drawing rule, threshold or target-float certification. The app's existing camera uses Rust `f64`/`libm` (`app/src/projection.rs`), not a chosen `f32` coastline path. This study uses **Python host `math`/binary64**, which is neither that Rust/libm implementation nor a measured S3 execution path.

## Paired trial coordinates

`tools/offline/coastline_unknown_precision_study.py` validates the pinned Natural Earth 1:110m v4.1.0 archive, performs the provisional 40 km reduction, round-trips the unapproved 1e-4° coordinate encoding and checks that longitude interpolation does not change branch. It then applies **the same provisional certified-front local-gap/stroke procedure** to unquantized and decoded 1,734-segment inputs at Colorado, San Francisco, equator/dateline and 89.9°N, at 0.1°, 1°, 5°, 20° and 100° FOV. Source/encoding and sampling are **not** adopted.

| Paired trial result over 20 scenes | Observation |
| --- | --- |
| Stroke pixels changed | **5 total:** San Francisco 0.1° (3), San Francisco 20° (1), polar 20° (1); 0 in the other 17 scenes. |
| Geographic and viewport unknown interval counts | Same in each paired scene (up to 18 geographic unknowns at dateline 20°/100°); **0 viewport unknowns** in these tested candidate scenes. |
| Enclosure node counts and the three illustrative whole-geography fallback outcomes | Same per paired scene; no per-segment node-cap exhaustion. |
| Certified-front leaf counts | **Can differ** at 20°/100° despite equal unknown counts; e.g. polar 20°/100°: 1,337 unquantized versus 1,324 decoded. Source geometry and subdivision thresholds shift. |

Changed pixel coordinates are included in the temporary JSON. Five pixel changes are not an error bound for another observer/zoom, a tangent location or a target framebuffer. Equal fallback decisions do not authorize the [example 0.0001°/0.001°/0.01° thresholds](m4-coastline-unknown-policy-study.md), nor does the stroke prove a pixel-empty gap around unknown leaves.

## Threshold conditioning, not float certification

The tool records the **smallest observed absolute host-binary64 distance to a decision threshold** across the 20 scenes and both inputs. The minimum for the coarse frustum plane test is **0.630 km** (decoded San Francisco 5°), for an accepted front/back sign decision after the trial 1e-6 km sign slack **0.0000354 km** (unquantized dateline 100°), and for an expanded chord-bbox viewport edge **0.0000391 px** (decoded polar 5°). These are distances in the *computed expressions*, **not bounds on numerical evaluation error**. `math.ulp(f)` ratios in the JSON apply only to the final sign terms, not errors accumulated from trigonometry, WGS-84, camera setup or analytic derivatives. Neither the 0.1 km cull pad nor these positive margins has a justified independent error budget.

Three adversarial input probes illustrate why strict boundary decisions must keep an unknown/error path:

- At ±16 `nextafter` steps around an equatorial tangent, the two host front-sign results change sign (approximately ±3×10⁻¹⁰ km), but both zero-length test intervals remain **unknown** under the trial sign slack.
- At the host-f64 coarse-frustum transition, adjacent representable longitude inputs switch kept/cut with computed plane distances as small as ~3×10⁻¹⁵ km.
- For a synthetic projected chord outside the viewport by approximately its **0.05 px** error envelope, one-ULP shifts of the normalized x input switch **viewport unknown** to **outside**, with computed edge distances ~10⁻¹⁴ px. The chord is an input to the viewport test, **not** a verified WGS-84 surface contact.

These are *different input geometries*, not a test of arithmetic error on an identical real input. They demonstrate near-boundary sensitivity but neither refute nor prove correctness of the earlier mathematical bounds. No `libm` rounding guarantee, operation-by-operation interval calculation, tested target build, global worst-case search, near-plane proof, pixel raster proof or certified fallback budget is provided. Source-wide candidate work remains a measured proxy, not a device budget. Unknowns must not be silently classified as invisible.

## Reproduction / next gate

With the [previously verified temporary archive](m4-coastline-source-preflight.md#reproduction-and-input):

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/offline/coastline_unknown_precision_study.py \
  /tmp/overhead-coastline-preflight/ne_110m_coastline.zip \
  /tmp/overhead-coastline-comparison/unknown-precision
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools/offline -p 'test_coastline_compare.py'
```

Temporary `report.json` SHA-256 `357e4529b28f170c5852dbdc2a53a54c4d45b6ca8f68e7e5d27f8bf7cd6f1d67`; offline tests and workspace check/tests pass. No firmware, interactive simulator, hardware or frozen-M2 capture. **Next gate:** the [coast-only fallback appearance/work comparison](m4-coastline-fallback-appearance-study.md) shows the visible cost of even a tight geography-wide threshold and reports full trial raster counts. Specify a defensible f64/libm error bound or independently checked interval/uncertainty strategy for sign, cull and projected-viewport tests, particularly at tangencies; then review **finished-frame** layering and target work against an agreed device budget. Do not promote host-f64 margins or the user's chosen investigation direction into implementation approval.
