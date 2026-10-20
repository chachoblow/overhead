# M4 coastline — offline source-wide visibility-enclosure work proxy

Date: 2026-10-20. **Diagnostic only**, extending the [synthetic second-derivative study](m4-coastline-curvature-bound-study.md) to the pinned Natural Earth 1:110m v4.1.0 *candidate*. The trial 40 km ECEF reduction, 1e-4° `i32` coordinates, coarse-frustum cull and front-face enclosure remain **unapproved** as an export, renderer or operating policy. No app/render/firmware code or bundled data changed.

## Method

`tools/offline/coastline_enclosure_work_study.py` validates the pinned archive, reduces its parts to the same trial 1,868 vertices / 1,734 segments, encodes and decodes the same **temporary** 15,484-byte trial (`SHA-256 951ed542a445201c24241bfb263b81567cef20641854be281778788a7a7a6796`), and rejects a longitude-wrap branch change. For each trial source segment, separately on unquantized and decoded coordinates, it runs the [second-derivative front-sign and conservative frustum enclosure](m4-coastline-curvature-bound-study.md) with an **8,191-node per-segment cap** and `0.00001°` unknown-leaf threshold. It counts nodes, classified leaves, unknowns and work exhaustion. The tool retains every unfinished interval as **unknown**, not invisible. It samples Colorado, San Francisco, equator/179.9°E and 89.9°N at 0.1°, 1°, 5°, 20° and 100° vertical FOV. The cap and threshold are study controls, not safe runtime limits or coastline visibility rules.

| Tested sites / FOV | Maximum nodes across 1,734 segments | Maximum nodes on one segment | Unknown intervals (by site: CO / SF / dateline / polar) | Budget-exhausted intervals |
| --- | ---: | ---: | ---: | ---: |
| All four / 0.1°, 1° | 1,734 per scene | 1 | 0 / 0 / 0 / 0 | 0 |
| All four / 5° | 1,738 (CO, polar) | 3 | 0 / 0 / 0 / 0 | 0 |
| All four / 20°, 100° | **2,432** (dateline) | **51** (dateline) | **16 / 10 / 18 / 14** per scene | 0 |

These are maxima and unknown counts on the **tested** four observer sites, not a global catalogue/work bound. At 20° and 100° the measured per-site totals match; this does not make FOV-independent runtime work a contract. The largest single unknown interval in those tested scenes is <0.00001°; the largest *sum* across one scene is ~0.00013° (dateline). At every tested scene, unquantized and decoded trial inputs agree on the **identities of segments with unknown/budget-exhausted work**, as well as on the integer node and unknown counts. Their precise unknown widths can differ slightly. Those equivalences do not prove root location or pixel identity after trial quantization. Tight views with zero unknown leaves can still have no nearby coastline or have an inaccurate sampled surface; absence of unknown here does **not** prove below-5° detail or legibility.

A deliberately synthetic [exact graze](m4-coastline-curvature-bound-study.md) already needs 767 nodes and retains **334 unknown** intervals on **one** segment, and the first-order variant exhausts the 8,191-node cap. The measured 51-node maximum over this candidate at four sites therefore cannot set a supported per-frame cap or justify dropping/inking unresolved intervals. Returning unknown is a conservative *diagnostic outcome*, not a complete executable drawing rule: ink, omission and user-facing behavior need separate agreement. The tool's node count omits projected subdivision/raster, cull cost weighting, S3 timing/flash/stack, catalogue work, Sharp refresh and scheduling. Host f64 sign tests and 0.1 km cull slack lack a proved **target-float** margin; mathematical inequalities are not verified interval arithmetic.

## Reproduction and remaining gate

From repo root with the [previously verified temporary archive](m4-coastline-source-preflight.md#reproduction-and-input):

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/offline/coastline_enclosure_work_study.py \
  /tmp/overhead-coastline-preflight/ne_110m_coastline.zip \
  /tmp/overhead-coastline-comparison/enclosure-work
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools/offline -p 'test_coastline_compare.py'
```

Temporary `report.json` SHA-256 `9d06f9f7532e16b6fcbbcf319842593f5c9060ff1d84bbeb2ce30f3cc195c85e` retains all 40 representation×scene rows, including work and worst offending source segment. Eighteen offline tests, workspace check and tests pass. No executable coastlines, interactive replay, hardware/firmware, new source archive or frozen M2 capture. **Next gate:** the user chose to [investigate local gaps with a bounded whole-geography fallback](m4-coastline-unknown-policy-study.md), not approved thresholds or a renderer. Establish a finite, conservative target-float/numerical margin and an exact handling contract for unresolved tangent and viewport crossings; then measure full candidate projection/raster work against a separately agreed device budget. This sampled work proxy alone cannot authorize asset adoption or renderer implementation.
