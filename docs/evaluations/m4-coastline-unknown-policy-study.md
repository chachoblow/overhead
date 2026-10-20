# M4 coastline — offline local-gap and fallback sensitivity

Date: 2026-10-20. After the [source-wide enclosure preflight](m4-coastline-enclosure-work-study.md), the user chose to **investigate local gaps with a bounded whole-geography fallback**, not to approve a renderer, threshold or screen treatment. This is an offline diagnostic of that direction. The 40 km unquantized source reduction, 0.2° sampling, screen stroke and real-arithmetic sign/chord bounds remain provisional; no coastline data or executable renderer has been added.

## Trial and limitations

`tools/offline/coastline_unknown_policy_study.py` runs the second-order front-face enclosure on each provisional source segment and collects **only certified-front** subintervals. It never draws a *chord* through a hidden/uncertain interval or across a part boundary; the trial's radius-0.5px stroke caps may still ink a pixel adjacent to an unknown interval, so this is **not** a proof of a pixel-empty gap. On each retained interval it uses at most 0.2° geographic-linear steps, projects the surface, and tests the *entire* projected curve's possible viewport contact against the [point-to-chord bound](m4-coastline-curvature-bound-study.md). If even the expanded chord bbox cannot enter, the interval is safely outside in real arithmetic; if the chord misses but its error envelope could enter, it records **viewport unknown** instead of declaring the interval absent. Chords that intersect are clipped and sent to the [experimental pixel-center stroke](m4-coastline-stroke-study.md). This last step is **not** a certified raster of the actual clipped curve or a target-float proof; even a subpixel deviation can flip a pixel at a boundary. An unknown is a diagnostic outcome, not a satellite/ingestion notice.

For six tested candidate scenes, the *trial* local-gap stroke differs by **zero pixels** from the earlier 0.2° sampled continuous-stroke mask:

| Scene | Enclosure nodes | Geographic unknown intervals / total angular span | Viewport unknown | Front subintervals / clipped strokes | Stroke ink |
| --- | ---: | ---: | ---: | ---: | ---: |
| San Francisco 0.1° | 1,734 | 0 / 0° | 0 | 106 / 8 | 268 |
| San Francisco 5° | 1,734 | 0 / 0° | 0 | 5,468 / 3,262 | 1,724 |
| San Francisco 20° | 2,096 | 10 / 0.0000758° | 0 | 12,585 / 12,585 | 890 |
| San Francisco 100° | 2,096 | 10 / 0.0000758° | 0 | 12,585 / 12,585 | 82 |
| Colorado 5° | 1,738 | 0 / 0° | 0 | 5,932 / 3,869 | 2,049 |
| Colorado 20° | 2,322 | 16 / 0.000113° | 0 | 13,056 / 13,056 | 980 |

Mask equality is **not** proof of tangent, viewport or source detail correctness: a deliberately synthetic *exact graze* leaves **334** geographic-unknown leaves spanning 0.00204° yet produces zero ink in both paths; the older sampler can miss a touch. A one-node forced-budget sign crossing retains two unknown intervals spanning 0.6° and draws **zero** trial pixels versus the old sampler's one; the whole-geography fallback is needed rather than treating such a large gap as normal. A two-off-screen-endpoint viewport crossing retains 347 pixels with no unresolved work. A direct geometric test confirms that a chord entirely outside the viewport but within its curvature envelope is classified viewport-unknown, not hidden. None of the six candidate scenes exercised that viewport-unknown path; it needs further geometry and numerical validation. These six scenes reuse the pinned candidate but create **no fresh executable coastline snapshots** or Sharp-panel evidence.

For **threshold sensitivity only**, the temporary report asks whether a geography-layer fallback would trigger when (a) any per-segment work is exhausted, or (b) total unknown geographic span exceeds a hypothetical **0.0001°, 0.001° or 0.01°**, or (c) more than **0, 8 or 100** viewport intervals are unknown, respectively. All three combinations fall back on the forced-budget crossing; the 0.0001°/0-viewport trial also falls back at Colorado 20°; the 0.001°/8-viewport trial falls back on the exact graze but on **none of the six candidate scenes**. The 0.01°/100-viewport trial misses the synthetic exact-graze fallback. These cutoffs are **not approved**, are not worst-case bounds and must not become silent runtime constants. A whole-geography fallback, if later selected, would suppress only coastline background, never the limb, notices, satellite truth, selection or square baseline. Neither a user-facing warning nor a device error policy is chosen here.

The work accounting is **not** target time/stack: San Francisco 20° requires 12,585 front subintervals despite 2,096 enclosure nodes and the 100° stroke has only 82 pixels. Stroke/pixel-candidate costs, tangent/viewport handling, quantized-asset behavior, real-f64 libm roundoff and S3 costs remain open. Full 0.1°–100° multisite visual acceptance, compiled asset size, on-device budget and runtime over-budget behavior are not established. The [contract proposal](../mockups/0055-coastline-contract-proposal.md) remains unapproved.

## Reproduction and next gate

With the [previously verified archive](m4-coastline-source-preflight.md#reproduction-and-input):

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/offline/coastline_unknown_policy_study.py \
  /tmp/overhead-coastline-preflight/ne_110m_coastline.zip \
  /tmp/overhead-coastline-comparison/unknown-policy
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools/offline -p 'test_coastline_compare.py'
```

Temporary `report.json` SHA-256 `734a7d5835e9dba6148e55f63739ee74a6e66d66f510aadeab64c67d29e87db5`; 19 offline tests and workspace check/tests passed. No interactive simulator, firmware, hardware or frozen-M2 capture. **Next gate:** the [decoded-coordinate and host-f64 threshold preflight](m4-coastline-unknown-precision-study.md) found five trial-pixel changes and no certified float-error bound. Review candidate and adversarial unknown/fallback appearances and work, and establish a justified sign/cull/viewport numerical strategy before proposing precise tolerances. The user's chosen *investigation direction* is not authorization to implement the renderer or asset.
