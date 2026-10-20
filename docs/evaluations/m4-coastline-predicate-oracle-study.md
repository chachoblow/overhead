# M4 coastline — narrow high-precision predicate controls

Date: 2026-10-20. Follow-up to [host-f64 threshold probes](m4-coastline-unknown-precision-study.md) and the [coast-only fallback/work comparison](m4-coastline-fallback-appearance-study.md). **This is a narrow independent numerical control, not a certified interval algorithm or an approved runtime rule.** No coastline asset, app/render code, target benchmark, threshold or new dependency was added.

## Why a separate reference

The existing camera uses Rust `f64`/`libm`; the offline coastline code uses Python host `math`/binary64. The workspace and isolated firmware lockfiles currently pin **`libm` 0.2.16**. Its published [`sin.rs` source](https://docs.rs/libm/0.2.16/src/libm/math/sin.rs.html#41-42) calls trig output *“nearly rounded”* without a quantitative bound there; the [published `sincos` source](https://docs.rs/libm/0.2.16/src/libm/math/sincos.rs.html#15-17) describes return values, not a numerical error contract. Neither source passage by itself justifies assigning a fixed ULP budget to every operation of a WGS-84 visibility/cull/viewport predicate. Host `math` observations are especially not measurements of the Rust `libm` S3 path.

`tools/offline/coastline_predicate_oracle_study.py` evaluates **only zero-altitude equatorial point-segments at a zero-longitude observer** using independent analytic real formulas for front sign and the positive-east coarse-frustum plane. It converts the already constructed host binary64 input values to exact `Decimal`, uses 80-digit arithmetic with Machin arctan/Taylor trigonometry, and compares against existing host float predicates. This avoids applying a second implementation of the full WGS-84/analytic derivative/camera algorithm as its own oracle. Separate 80/95-digit sanity checks at the tangent agree better than 1e-55 km. It does **not** provide formal directed-rounding intervals or a complete Taylor/reduction roundoff certificate, does not analyze all view planes, and does not cover non-equatorial curves or target `libm`.

| Narrow control | High-precision / host-f64 observation |
| --- | --- |
| Equatorial front sign at ±16 adjacent binary64 longitude steps around an analytic tangent | Correct sign in both sampled host cases; high-precision magnitudes about +3.048×10⁻¹⁰ and −3.391×10⁻¹⁰ km. The host results differ from reference by ~2.0×10⁻¹² and ~6.5×10⁻¹² km respectively, and the trial 1e-6 km slack retains both point tests as **unknown**. |
| Four adjacent input longitudes bracketing the equatorial positive-east frustum transition at 0.1° FOV | Host kept/cut matches the high-precision plane sign in all four cases. Reference plane magnitudes range ~2.9×10⁻¹⁵ to 2.2×10⁻¹⁴ km; the host `plane + radius` operation is also extremely close to zero. The **0.1 km cull pad does not supply a roundoff guarantee at the padded boundary**. |
| Three adjacent *supplied normalized chord x inputs* at an expanded viewport edge with 0.05px chord error | Host outside/unknown matches the high-precision sign in these three inputs, with reference normalized-edge margins down to ~9.7×10⁻¹⁷. This is **not** a projected WGS-84 curve or a raster oracle. |

The tested predicates happen to agree at these points; absence of a discrepancy is **not** a global soundness result or a libm guarantee. The chosen inputs differ by ULPs, and the high-precision reference treats the original binary64 *inputs* as exact, not idealized decimal source coordinates. At a decision boundary, arbitrarily small mathematical margins exist. A valid fail-safe implementation must either bound all numerical errors *before* classifying an interval as back/outside, or independently evaluate the near-boundary case with certified arithmetic and preserve unknown/work-exhaustion behavior. Simply increasing the 1e-6 km/0.1 km study slacks based on these few errors, or interpreting sample agreement as proof, would silently change the evidence gate.

## Reproduction / stop condition

From repo root, no archive or new crate is needed:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/offline/coastline_predicate_oracle_study.py \
  /tmp/overhead-coastline-comparison/predicate-oracle
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools/offline -p 'test_coastline_compare.py'
```

Temporary `report.json` SHA-256 `f9faea6c345cb3bfa25fe748623f07d57425976e0bfec90e31a11b2dcf73dbf4`; 22 offline tests and workspace check/tests pass. No interactive replay, firmware, hardware or frozen-M2 capture.

**Stop before export or renderer proposal:** a narrow independent reference cannot certify the source-wide enclosures. The user chose to [investigate bounded numerical uncertainty](m4-coastline-interval-predicate-feasibility.md); its narrow exact-rational control specifies proof obligations but does **not** certify the source-wide enclosures, target cost or a separately agreed device-work budget. No chosen tolerance, fallback threshold or evidence of 20 Hz target margin follows from this diagnostic.
