# M4 coastline — bounded streaming-enclosure memory probe

Date: 2026-10-20. Follow-up to the [two-pass fallback study](m4-coastline-atomic-fallback-study.md). This is an **offline host comparison**, not an asset, renderer, target representation, numeric proof, selected pending limit or device memory measurement. [0051](../decisions/0051-angular-coastline-appearance-direction.md) and [0052](../decisions/0052-natural-earth-coastline-candidate.md) remain unchanged.

## Method

The original `enclose` list-collecting path remains available as an oracle. In the new optional bounded path, `enclose` calls a front-leaf visitor immediately and an unknown-leaf visitor (with an explicit budget flag) rather than retaining front/unknown interval lists per source segment. It holds a depth-first **pending interval stack** with a checked maximum; when another split would exceed it, both unvisited children **and all pending intervals** are reported as *budget unknown*, never declared hidden. Peak pending entries and cap hits are counted. `trial` can consume front leaves as they arrive, without accumulating per-segment front leaves or screen segments in preflight; per-part iteration does not slice/copy a source part. No fallback threshold or pixel rule was changed.

The study uses an **illustrative 16-pending-entry cap** on the previously pinned Natural Earth 1:110m v4.1.0, unapproved 40 km/unquantized reduction at four sites × five FOVs. For each of the 20 scenes, the retained-list mask and every shared counter matched the bounded-streaming mask; there were **zero changed pixels and zero pending-cap hits**. The largest observed pending depth was **15** (polar, 20°). Two-pass preflight/commit still matched the bounded local-gap oracle and did not trigger the illustrative geography fallback in those 20 scenes. This sample does not bound arbitrary source curves, observer locations, input precision or numerical behavior.

Synthetic exact graze and sign crossing with an illustrative **four-entry** cap each triggered one cap hit and retained five budget-unknown leaves. On the crossing, previously certified front leaves remained counted, so a partial-result failure is real rather than hypothetical. The mixed case places a visible center coast before the capped crossing: its bounded local-gap trial would draw **6 coast pixels**, but preflight triggered whole-geography fallback **before any pixel writer call** and emitted zero. Separate two-offscreen-endpoint and polar/dateline controls did not hit the 16-entry cap; their coast-only pixels matched the list oracle. No finished frame with limb, notices or selected marker was rendered.

## What the cap does **not** establish

- An interval consisting of two latitude/longitude endpoints alone could occupy **4×8 = 32 bytes** in a compact four-binary64 representation: 16 pending entries would already consume **512 bytes before** indices, transform/trig temporaries, call frames, counters, sampling and raster state. Python objects consume much more; the retained source and the simulated 400×240 frame/oracle also use host memory. There is **no measured ≤1 KiB incremental scratch/stack** or zero-heap target representation. The depth-first algorithm still has a per-segment node cap of 8,191; it does not impose a whole-geography operation cap or an end-to-end finite-cycle bound.
- A pending-stack overflow calls the unknown visitor for all unfinished leaves, possibly **after** already streaming certified front leaves. The two-pass study can discard those only because preflight emits no ink and the illustrative fallback fires on budget unknown. A late numeric, draw-target or raster failure during commit is still non-atomic. Pending entries are not the only live memory, and streaming callback/test sinks may themselves retain data.
- The host-f64 front/cull/viewport tests, sampling, clipping and distance raster still lack independently justified target arithmetic and error bounds. Unknown does not mean invisible; the illustrative `0.001°`/`8` fallback inputs are not accepted omission or presentation rules. No target cycles, Sharp output, frame headroom, safe-stack depth, compiled asset bytes or panel legibility was measured. The [provisional measurement gates](m4-coastline-budget-preflight.md) cannot yet be scored.

**Next gate:** specify a checked finite global coast-work limit and commit-time failure treatment, then decide whether a fully bounded numerical path and real device baseline are practical *before* seeking separate export/renderer/hardware authorization. Do not infer S3 cost or 20 Hz support from bounded Python queue length.

## Reproduction

With the [verified temporary archive](m4-coastline-source-preflight.md#reproduction-and-input), from repo root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/offline/coastline_streaming_enclosure_study.py \
  /tmp/overhead-coastline-preflight/ne_110m_coastline.zip \
  /tmp/overhead-coastline-comparison/streaming-enclosure
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools/offline -p 'test_coastline_compare.py'
cargo check --workspace
cargo test --workspace
```

Temporary report SHA-256 `23c5c6c23986ceb1dac84e580b545c82887080bddebbf3e65eaf420950cb5496`. The prior two-pass report was rerun and retained its original SHA-256 `b299b2343f22db9b97c3973cccadc2addd6aafe9dc86ccc64fe7f594e1c839c1`. **28 offline Python tests**, workspace check/tests and `git diff --check` passed. No firmware, hardware, asset, renderer, interactive simulator or frozen-M2 capture was used.
