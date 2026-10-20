# M4 coastline — 40 km offline quantization and coarse-culling diagnostic

Date: 2026-10-20. **Study only.** The user chose 40 km from the [native comparison](../mockups/0056-coastline-reduction-comparison.md) as a *refinement baseline*, not as an approved simplification/export tolerance, LOD or renderer. The [contract proposal](../mockups/0055-coastline-contract-proposal.md) and [0052's conditional source](../decisions/0052-natural-earth-coastline-candidate.md) remain bounded by separate implementation authorization and target/visual evidence.

## Method and reproducibility

`tools/offline/coastline_refine_40km.py` reads the pinned v4.1.0 archive through the verified [preflight loader](m4-coastline-source-preflight.md), reduces each source part with the [40 km 3D ECEF chord-distance RDP trial](m4-coastline-reduction-study.md), serializes the **unapproved** little-endian 1e-4° `i32` lon/lat + `u32` part-offset layout, decodes those **actual temporary bytes**, and compares masks rendered from unquantized versus decoded coordinates. At every scene, it also compares the full-scan mask against a conservatively culled mask for **each** representation; any cull-induced pixel difference fails the run. This checks only the sampled trial curve and Pillow pixels, not an executable Rust renderer or embedded-graphics raster.

The candidate coarse cull scans all **1,734 stored source segments** per frame. For each segment, bound its geographic-linear, shortest-longitude surface path inside an ECEF sphere centered on its first vertex, radius `6500 km/radian × hypot(Δlatitude, Δlongitude) + 0.1 km`. The WGS-84 meridian/parallel surface-coordinate speeds are <6500 km/radian; the 0.1 km pad exceeds the maximum single-vertex trial quantization displacement (~0.01 km), with further host-rounding margin. Reject the entire segment only if the sphere lies strictly outside **one of four linear pinhole-frustum half-spaces**, with the plane normal's length applied to the radius. This is an analytically conservative bound for the specified surface curve in real arithmetic: a segment with *both endpoints outside* can still enter and must be retained. It is **not** a validated target-float error margin, a camera-front-face cull, a guarantee for the real unsimplified coastline, or an index-free cheap runtime implementation. Accepted segments still undergo the [0.2° sample/tangent/viewport experiment](m4-coastline-reduction-study.md).

The tool evaluates 20 geography-only masks: Colorado, San Francisco, equator/179.9°E, and 89.9°N, each at **0.1°, 1°, 5°, 20°, 100°** vertical FOV. No new satellite samples or simulator snapshots are produced; the [0056 sheets](../mockups/0056-coastline-reduction-comparison.md) remain the review images. Reproduce from repo root using the pinned archive path from the [source preflight](m4-coastline-source-preflight.md):

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/offline/coastline_refine_40km.py \
  /tmp/overhead-coastline-preflight/ne_110m_coastline.zip \
  /tmp/overhead-coastline-comparison/refine-40
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools/offline -p 'test_coastline_compare.py'
```

Output `report.json` SHA-256: `49c5a61f3b6a735839001d9484ddbc4ba0f368e2f528d75ec3cd8fdab0c8e856`. The 15,484-byte trial decodes to 1,868 vertices across 134 intact parts (SHA-256 `951ed542a445201c24241bfb263b81567cef20641854be281778788a7a7a6796`). Maximum coordinate-rounding error is 0.00005°; the runtime representation remains unselected. All binaries, reports and detail images live under `/tmp`, not in the repo. Host Pillow is used solely for comparison masks; no new Cargo dependency was added.

## Observed bounds and pixel effects

| Site / FOV | Stored segments kept / scanned | Subdivided intervals full → kept | Front tests full → kept | Encoded vs float changed pixels |
| --- | ---: | ---: | ---: | ---: |
| Colorado / 0.1° | 1 / 1,734 | 23,377 → 44 | 70,643 → 132 | 0 |
| San Francisco / 0.1° | 4 / 1,734 | 23,377 → 106 | 70,451 → 318 | 0 |
| San Francisco / 1° | 10 / 1,734 | 23,377 → 190 | 70,451 → 570 | 0 |
| San Francisco / 5° | 636 / 1,734 | 23,377 → 9,198 | 70,451 → 27,594 | 0 |
| Colorado / 5° | 773 / 1,734 | 23,377 → 10,360 | 70,643 → 31,112 | 2 |
| 89.9°N / 0.1° | 28 / 1,734 | 23,377 → 1,577 | 70,579 → 4,731 | 0 |
| 89.9°N / 5° | 1,000 / 1,734 | 23,377 → 16,048 | 70,579 → 48,176 | 1 |
| All four sites / 20°, 100° | 1,734 / 1,734 | 23,377 → 23,377 | same as full | 0 |

Across **all 20 tested scenes**, coarse culling changes zero pixels for both unquantized and decoded geometry. The encoded geometry changes **three** mask pixels total: two at Colorado 5°, one at 89.9°N 5°. It changes none in the four 0.1° examples or any 1°, 20° or 100° example. Float/decoded culled tangent counts agree in these samples. A zero difference at tested tight-zoom sites does **not** bound worst-case screen displacement near arbitrary sites, grazing tangencies, zoom transitions or a coastline passing precisely through a pixel center. The coarse stage still computes an ECEF point and four plane tests for every stored segment; reduced front tests do **not** imply reduced S3 runtime at 20°–100° or prove stack safety. The chosen 0.1 km pad is analytical/headroom for this host trial, not a target-float proof.

Eight current Python tests cover bounded viewport crossings with two offscreen endpoints, hidden-endpoint surface/tangent handling, antimeridian shortest-path interpolation, closed parts, sphere-bound spot checks, trial decoding and finer-sampling controls. Full and culled images are checked pixel-for-pixel by the refinement tool for both float and decoded inputs. This does not test all sites, all field-of-view transitions, clipping accuracy between 0.2° geographic samples, correct longitude/near-tangent behavior on target or executable draw order. The frozen M2 evidence is unchanged; no firmware or hardware was used.

**Remaining gate:** keep 40 km as a comparison baseline only. The follow-on [sampling study](m4-coastline-sampling-study.md) compares finer steps but exposes 1-bit raster instability rather than proving curve/tangent accuracy. Test pixel-stable strokes and non-truncated below-5° scenes at more coastal locations, specify a checked maximum source/segment count and conservative target-float behavior, and obtain explicit renderer/asset authorization only after visual and target-work review. No physical-display or production cadence claim follows from these host counts.
