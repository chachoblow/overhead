# 0058 — Wide view draws a fixed-observer prepared quadratic coastline

Date: 2026-10-24
Status: implemented host/simulator slice; not device-adopted

## Decision

The wide view now draws the visually accepted [0056](0056-bounded-coastline-architecture-trial.md)
quadratic coastline through a **fixed-observer `CoastLayer`**, replacing the
[0054](0054-first-executable-coastline-comparison.md) 40 km interval renderer there.
The user chose replacement over an opt-in flag. The location stays fixed at
startup ([0004](0004-hardcoded-location.md) for firmware; CLI observer in the
simulator); setting or changing locations is future work under
[0057](0057-fixed-observer-coastline-preparation-scope.md).

- **Prepare once:** validate the embedded asset (`render/assets/coastline-110m-40km-quadratic.bin`,
  byte-identical to the S3 trial fixture) and observer, build the observer's
  camera transform and facing plane, and mark packed pieces with any
  front-facing interval. No heap, retained curves or bitmaps; the layer is
  **360 B** on the host.
- **Reuse:** satellite updates and lens zoom draw from the same layer. Drawing
  applies the current lens to the candidate's unchanged per-piece arithmetic,
  skipping only pieces the candidate classifies as wholly hidden. Tests require
  exact raster equality with `candidate_draw` across 12 sites × 6 zooms.
- **Loading screen:** the interactive wide simulator shows a provisional
  full-screen `PREPARING MAP` frame before the first radar frame. Its wording
  and layout are not a final contract.
- **Failures:** an invalid preparation, a layer for a different observer, a
  non-wide layout or a numeric draw failure shows the existing exceptional
  `COAST ERROR` and never stale or partial geography. Draw-target errors still
  fail the frame. The 0054 renderer remains for tests and the measurement harness.

## Why

The user wanted a working vertical slice rather than more pre-testing. The
piece mask is the smallest exact form: unlike the earlier host trials, it needs
neither 37–74 KB of retained `f32` curves nor the unapproved 1px differences of
the 16-bit format.

## Consequences

- Host draw time is only modestly below the per-call candidate (one release
  run: roughly 10–25% lower on most scenes, single noisy samples). The S3
  candidate took 9,961–22,489 µs, so this layer **should not be expected to meet
  the provisional 5 ms screen**; per-frame cost still needs a decision and a
  separately approved target measurement. Stack, Sharp transfer and whole-device
  cadence remain open.
- 0053's ordinary-gap rule and 0055's deferred all-view proof are unchanged.
- No firmware build, flash or device check was performed for this slice.

## Follow-up — 2026-10-24

[0059](0059-current-view-coastline-raster-cache.md) adds a cached raster of the
current view (layer now 10,960 B), so unchanged-view frames no longer redraw
geometry. The 360 B figure above describes the layer before that cache.
