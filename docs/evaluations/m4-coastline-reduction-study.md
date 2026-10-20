# M4 coastline reduction and segment-clipping study

Date: 2026-10-20. **Offline experiment, not a renderer contract or target budget.** [0052's pinned Natural Earth candidate](../decisions/0052-natural-earth-coastline-candidate.md) is unchanged. [Retained native 1-bit comparison sheets and provenance](../mockups/0056-coastline-reduction-comparison.md) own visual review; no image here is proof of physical-panel legibility or executable coastline rendering.

## Trial algorithm and bounds actually counted

`tools/offline/coastline_compare.py` validates the same pinned archive through `tools/offline/coastline_preflight.py`, then compares full raw vertices against **two exploratory 3D ECEF chord-distance RDP tolerances**, 40 km and 75 km. Preserve original polyline boundaries; split closed polylines at their farthest original vertex before RDP, and retain the closure. No DBF `scalerank` filter is used. These kilometre tolerances are **not** equivalent to the earlier 2.5px projected RDP and do not bound screen error across zoom or sites.

For each scene, the host trial interpolates *retained* consecutive lon/lat vertices at Euclidean geographic spacing ≤0.2°, including shortest-longitude wrapping; evaluates WGS-84 zero-height surface front-facing normal against the fixed 75,000 km camera; tests both endpoints and midpoint; bisects sign-changing front-face intervals for 32 iterations; projects front intervals; clips them against the wide normalized `[-1, 1]²` rectangle with Liang–Barsky **before** pixel rounding; then draws 1px lines into a 400×240 monochrome mask. These are bounded steps for the *sampled trial curve*, not an error proof for the underlying coastline between samples, whole-ellipsoid tangency, target floating-point, occlusion or embedded-graphics' eventual line raster. A more complex curve can still cross twice between tested samples; no double-front interval occurred in these tested scenes. Composite black ink is OR-ed over the finished simulator output, so foreground white notice backing and exact layering are **not** exercised.

| Trial | Vertices | Stored segments | Temporary `i32` trial bytes | Interpolated intervals / frame | Largest measured front evaluations / frame |
| --- | ---: | ---: | ---: | ---: | ---: |
| Raw | 5,128 | 4,994 | 41,564 | 26,303 | 79,485 |
| ECEF RDP 40 km | 1,868 | 1,734 | 15,484 | 23,377 | 70,707 |
| ECEF RDP 75 km | 1,171 | 1,037 | 9,908 | 21,696 | 65,664 |

Trial binaries under `/tmp/overhead-coastline-comparison/out/` use the **same unselected** 1e-4° little-endian `i32` coordinate pairs + `u32` part offsets as the [initial preflight](m4-coastline-source-preflight.md); hashes respectively `da85eb3da5b609c7c98bfb08ee39c6043aac9ece6e3ab5a7abe01fc06b7beca9`, `951ed542a445201c24241bfb263b81567cef20641854be281778788a7a7a6796`, `42cc7da911e7a201d2581076ed7aac8bc2fad1db13e5e4a1d67bc60d0586e19f`. This study draws the *unquantized* vertices; the subsequent [40 km diagnostic](m4-coastline-40km-refinement.md) compares decoded geometry at tight zoom. These counts are for a **full scan**, with no spatial index or part culling. The heaviest measured row for front evaluations is the equator/dateline site at 5° (not the largest possible site); every scene still visits all listed interpolated intervals. Host loop counts and kilobytes are not S3 timings, compiled flash/RAM, stack high water, safe bounds or production cadence. The 75 km trial reduces stored bytes ~76% relative to raw but total per-frame intervals only ~18%.

## Representative rendered-ink comparison

Pixel counts below are 1-bit *offline coastline mask* ink, not exclusive device pixels or drawable cost; slight differences from 0052's full overlay reflect separate float-to-pixel line and exact clipping rules. Each variant uses the **same** executable source frame for its row.

| Site / vertical FOV | Raw ink | 40 km ink | 75 km ink | Observation |
| --- | ---: | ---: | ---: | --- |
| San Francisco / 0.1° | 239 | 235 | 235 | Both reduced coastlines run as much straighter screen-crossing strokes; neither is visually approved below 5°. |
| San Francisco / 1° | 286 | 278 | 272 | Angular reduction visibly removes local bends; still a large line, not Bay/city detail. |
| San Francisco / 5° | 2,043 | 1,850 | 1,697 | 75 km is more angular than 40 km, but not the previously reviewed `rank == 0` + 2.5px preview. |
| Colorado / 5° | 2,387 | 2,205 | 2,037 | Several small bends/features drop as tolerance rises; source markers are unchanged in the composites. |
| Colorado / 20° | 1,165 | 1,144 | 1,101 | Geography continues to touch the limb and small-scale ink can merge. |
| Colorado / 100° | 96 | 96 | 96 | All three give the same tiny coastline ink *count*, not necessarily distinguishable geography. |

At San Francisco 0.1° there are zero visible markers and `SKY OUTSIDE VIEW` in the **source** snapshot; the added coast does not change that meaning. Colorado at 100° retains the tiny globe cue, but this trial does not prove physical legibility. Polar and dateline 5° scenes were also evaluated for work and clipping (polar 40 km: 10,017 clipped segments; dateline 40 km: 1,273); screenshots are disposable. After comparing the retained native sheets, the user chose 40 km for **further offline study**, not as a shipped tolerance; the follow-on [quantization/culling diagnostic](m4-coastline-40km-refinement.md) tests that baseline. **Do not equate the lower stored count with a sufficiently bounded per-frame renderer.**

## Reproduction, checks and next gate

After downloading/verifying the [pinned archive](m4-coastline-source-preflight.md#reproduction-and-input) to `/tmp/overhead-coastline-preflight/ne_110m_coastline.zip`, run `cargo build -p overhead-sim` and generate headless `--wide --snapshot` source frames for `sim/examples/weather-gps.json`, `2026-10-10T00:00:00Z`. For example:

```sh
mkdir -p /tmp/overhead-coastline-comparison/frames
./target/debug/overhead-sim sim/examples/weather-gps.json \
  2026-10-10T00:00:00Z 37.7749 -122.4194 0 --wide --fov-deg 0.1 \
  --snapshot /tmp/overhead-coastline-comparison/frames/sf-0.1.png
```

Repeat with sites/FOV/file stems: Colorado `39.007 -104.883 2.187` for `co-{5,20,100}.png`, San Francisco `37.7749 -122.4194 0` for `sf-{0.1,1,5,20,100}.png`, equator/dateline `0 179.9 0` for `dateline-5.png`, and near pole `89.9 0 0` for `polar-5.png`. Then:

```sh
python3 tools/offline/coastline_compare.py \
  /tmp/overhead-coastline-preflight/ne_110m_coastline.zip \
  /tmp/overhead-coastline-comparison/frames \
  /tmp/overhead-coastline-comparison/out
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools/offline -p 'test_coastline_compare.py'
```

The script writes per-scene composites, two sheets and `report.json`. Repeated `report.json` SHA-256 is `a6757f99bd795e93094b0ec15d730868931a8804b6487a4c8baca3b5408925f0`. Eight current offline host tests include two-outside-endpoint viewport entry, bounded far-side/tangent handling, shortest-antimeridian interpolation and closed-part preservation; they do **not** validate all tangent geometries. Retained sheets are strictly 1-bit and repeatable; all composite frames preserve source black ink and the 48px glyph strip. Workspace check and local link/whitespace checks passed. No interactive replay, target cost, panel, firmware or frozen M2 evidence was generated.

**Next gate:** the 40 km study direction is recorded, with [quantized-geometry and coarse-culling results](m4-coastline-40km-refinement.md). It still needs sample/clip error bounds at extreme zoom and tangent, marker/notice layering in an independently authorized executable slice, and S3 workload against a separately chosen budget. Retaining a smaller input alone does not close the geometry contract.
