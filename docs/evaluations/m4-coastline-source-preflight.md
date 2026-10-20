# M4 coastline candidate — offline size/work preflight

Date: 2026-10-20. **Evaluation only:** no runtime asset, clipping/raster implementation, approved simplification, memory budget or target timing. [0052](../decisions/0052-natural-earth-coastline-candidate.md) accepts a *conditional* source candidate; the [contract proposal](../mockups/0055-coastline-contract-proposal.md) remains unapproved.

## Reproduction and input

Fetch `https://naturalearth.s3.amazonaws.com/110m_physical/ne_110m_coastline.zip` to a **temporary** path and run:

```sh
shasum -a 256 /tmp/ne_110m_coastline.zip
python3 tools/offline/coastline_preflight.py /tmp/ne_110m_coastline.zip \
  --out-dir /tmp/overhead-coastline-preflight
```

`tools/offline/coastline_preflight.py` is a stdlib-only, host-only measurement tool. Optional `--out-dir` writes two *temporary* byte-count trials outside the repo; omit it for counts only. It refuses a SHA-256 other than `664449b39070027e882abb295974d182afec18ca21107273d17e9e8bf6f64817`, checks the archive's declared v4.1.0 and WGS-84 `.prj`, validates the PolyLine shapefile records/part offsets and finite lon/lat, then counts operations with a camera matching the current 75,000 km app view and wide 173×117 drawable half-extents. The archive was downloaded again on 2026-10-20; hash matched the [earlier provenance](../mockups/0051-coastline-legibility-comparison.md#candidate-and-provenance). It was **not** added to the repository. Two runs without trial output produced identical text (SHA-256 `8022137a3c828b32ab259f70bb8bcc1fe68c40bea0917b4ee21b0735f05fd5fb`); the optional temporary-artifact run produced text SHA-256 `9ca2cec7a594387b6ce9181cab1c7a15d987840d01db5e758e96ee08a8bbc5ff`. No extra package was installed.

## Measured inputs and modeled storage

| Input/strategy | Parts | Vertices | Segments | Trial bytes |
| --- | ---: | ---: | ---: | ---: |
| Source polylines | 134 | 5,128 | 4,994 | 41,564 |
| Geographic interpolation at ≤0.2° per latitude/shortest-longitude step | 134 | 26,437 | 26,303 | 212,036 |

Trial sizes match a modeled little-endian `int32` longitude/latitude pair at 1e-4° resolution per vertex followed by `(parts + 1)` `u32` part offsets. Temporary `.bin` trials written under `/tmp/overhead-coastline-preflight` measured exactly 41,564 and 212,036 bytes (SHA-256 `da85eb3da5b609c7c98bfb08ee39c6043aac9ece6e3ab5a7abe01fc06b7beca9` and `eb806d138aaebe7479c8f3e8c25b1cd71cbfd1937f935bf530d752d22d242216`); maximum per-coordinate rounding error is 0.00005°. These are **not** a committed export, a selected format, measured compiled flash/RAM, header/index/alignment, or validation of projected quantization error at 0.1°. The 0.2° rule is the earlier *preview* sampling, tested as a costly comparison here—not an approved generator rule or adequate tight-zoom error bound. It increases vertex count 5.2×. Source has 120 exactly closed parts; largest source part has 693 vertices, becoming 3,464 with this sampling. The largest original step is 9.055° measured as the Euclidean norm of latitude and shortest-longitude differences. Four source longitudes exceed +180° by at most 0.000000442°; the preflight checks the excess is ≤1e-6° and normalizes those four to 180°. No consecutive source longitude jump exceeds 180°; that fact alone does **not** prove all antimeridian part-boundary handling or longitude encoding safe.

## Per-frame operation proxies (0.2° candidate only)

An exhaustive scanner would inspect **26,303 surface segments per frame**, even if the screen contains no coast. `front` counts segments with at least one endpoint camera-facing by ellipsoid-normal dot product; `in` counts ones with at least one *front-facing endpoint* within the wide normalized viewport; `box` counts segments with both endpoints front-facing whose projected bounding box intersects it. These are independent **proxies**, not a union or a correct hidden/tangent/viewport segment clip. A segment with both endpoints outside can cross the viewport; both back-facing endpoints may conceal a visible curved run. The scan does not subdivide for screen error, rasterize, evaluate per-segment cost on S3, allocate an index, or time target work.

| Observer / FOV | Inspected | Front proxy | In proxy | Box proxy |
| --- | ---: | ---: | ---: | ---: |
| Colorado / 0.1°, 1° | 26,303 | 14,628 | 0 | 0 |
| Colorado / 5° | 26,303 | 14,628 | 4,446 | 4,446 |
| Colorado / 20°, 100° | 26,303 | 14,628 | 14,628 | 14,612 |
| San Francisco / 0.1° | 26,303 | 14,123 | 10 | 10 |
| San Francisco / 1° | 26,303 | 14,123 | 100 | 100 |
| San Francisco / 5° | 26,303 | 14,123 | 3,788 | 3,788 |
| San Francisco / 20°, 100° | 26,303 | 14,123 | 14,123 | 14,113 |
| Equator, 179.9°E / 5° | 26,303 | 9,943 | 1,458 | 1,458 |
| 89.9°N / 5° | 26,303 | 17,724 | 11,234 | 11,234 |
| 89.9°N / 20°, 100° | 26,303 | 17,724 | 17,724 | 17,710 |

All FOVs 0.1°, 1°, 5°, 20°, 100° were evaluated at all four observers; the script prints every row. Colorado and the equator/dateline example have zero in-view coastline endpoints at 0.1°; the San Francisco case has only ten candidate segments. This does **not** mean the final correctly clipped geometry has zero ink, nor validate the proposed below-5° look. Near polar 5° has much more candidate work than the midlatitude 5° sites. Pixel-space reduction in the preview cannot be translated into a single global stored vertex count from this table.

## Consequences / missing evidence

The raw source trial is 41.6 KB, but naive 0.2° interpolation costs ~212 KB *before* indexing and up to 26,303 segment visits per frame even in an empty local view. Neither is an agreed limit or an S3 capacity/cadence measurement. A smaller asset cannot be assumed to reproduce [0051's reviewed angular scenes](../decisions/0051-angular-coastline-appearance-direction.md); the 0.2° variant cannot be assumed accurate at 0.1°. The earlier mock's `scalerank == 0` and 2.5px projected RDP are **not** approved offline export rules. This preflight gives finite counts for two example representations, not a defensible bound for the still-unselected geometry algorithm, index, working memory or stack.

**Next gate before asset/renderer authorization:** compare a *specific* offline reduction and candidate spatial/index policy against the approved 5°–100° look and new 0.1°–<5° scenes, measure actual encoded bytes and worst-case subdivision/culling/clip work (including tangent, edge and polar cases), then set a checked export/runtime bound and evaluate it separately on target. Keep the M2 frozen capture separate. `cargo check --workspace` passed after adding the offline script; no Cargo tests, simulator images, physical panel or hardware captures were run for this preflight.
