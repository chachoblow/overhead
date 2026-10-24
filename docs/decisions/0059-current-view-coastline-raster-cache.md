# 0059 — Cache the current view's coastline raster

Date: 2026-10-24
Status: implemented host/simulator slice; not device-adopted

## Decision

`CoastLayer` ([0058](0058-prepared-wide-coastline-slice.md)) keeps one **352×240 1-bit
raster (10,560 B)** of the coastline for the current lens. With a fixed observer,
coastline pixels change only when the view changes; satellite motion does not
affect them. Frames with an unchanged view copy the cached pixels, and a zoom
change triggers one fresh raster. The layer is now **10,960 B** on the host and
belongs in long-lived storage, not a transient stack frame.

This revises the provisional "no persistent coast-only bitmap" default in the
[budget preflight](../evaluations/m4-coastline-budget-preflight.md) for this
layer only, at the user's approval. The renderer still clears and repaints the
whole frame each time; only the coastline input is cached.

## Why

Per-frame redraw came from the renderer's single-layer, stateless design:
markers, labels and notice backings cover coastline pixels, and pixels cannot be
read back from the panel. Prepared geometry alone saved only ~10–25% on the host,
so it was unlikely to bring the S3's 9,961–22,489 µs coast cost under 5 ms. The
initial device zooms through about four BOOT-button presets, so most frames reuse
the cache; continuous zoom re-rasters only while the lens is moving.

## Consequences

- Output is pixel-identical to the per-call candidate; tests cover reuse,
  re-rastering on zoom, refusal for another observer without disturbing the
  cache, and invalidation after a failed raster. A raster that fails part-way, or
  writes outside the radar, never reaches the display and shows `COAST ERROR`.
- Two release host runs (single samples, noisy): cached draws averaged roughly
  an order of magnitude below a fresh raster; fresh rasters cost about the same as
  the candidate. Neither is an S3 figure.
- A zoom step on the device still costs one full raster (≈10–22 ms on the S3,
  inferred from the candidate, not measured). S3 cached-draw time, stack, memory
  placement, Sharp transfer and whole-device cadence remain unmeasured; any
  target check needs separate approval.
- 0053's ordinary-gap rule and 0055's deferred all-view proof are unchanged.
