# M4 coastline — two-pass atomic fallback feasibility (offline)

Date: 2026-10-20. Following the [provisional resource preflight](m4-coastline-budget-preflight.md), this tests one **architecture hypothesis**, not a renderer, selected fallback cutoff, approved asset, runtime buffer or target performance claim. [0051](../decisions/0051-angular-coastline-appearance-direction.md)'s reviewed 5°–100° appearance and [0052](../decisions/0052-natural-earth-coastline-candidate.md)'s conditional source remain unchanged.

## Trial and observation

`tools/offline/coastline_atomic_fallback_study.py` reuses the unapproved 40 km reduction, host-f64 visibility/viewport enclosure and 0.2°/pixel-center stroke study. Pass 1 visits all source segments and accumulates diagnostic unknowns and bounding-box candidate counts **without a coast raster or accumulated screen-segment list**. The illustrative `arc_le_0.001_viewport_le_8` sensitivity example (not an accepted tolerance) decides whether to skip coast entirely. If it does not, pass 2 repeats the geometry and streams stroke pixels into a simulated existing frame. A separate full-mask reference traversal checks pixel equivalence but is **excluded from two-pass work counts**. Neither the reference nor the simulated 400×240 byte frame is proposed as additional target storage. The simulated frame starts white and omits limb, notices, markers, display errors and device output; it cannot validate finished-frame compositing.

For pinned Natural Earth v4.1.0, four observer sites × five FOVs (0.1°, 1°, 5°, 20°, 100°), the example policy chose the commit path in all 20 scenes. Every streamed coast-only result matched the existing local-gap mask (zero changed pixels). The largest **combined two-pass** work proxy was **4,864 enclosure nodes** (dateline 20°), **31,612 front subintervals** and **35,714 stroke candidate visits** (both polar 20°). These double the corresponding single-pass maxima of 2,432 / 15,806 / 17,857; other geometry and decision work is also repeated, but these counts are not cycles or worst-case bounds. Preflight counts candidates using a clamped bounding box in constant time, while commit tests each candidate's pixel distance; counting candidate *visits* in the table does not mean preflight paid their raster cost.

A synthetic mixed case puts a visible center segment **before** a horizon-crossing segment with a one-node-per-segment cap. The latter forces budget unknowns; the example fallback skips the entire commit, leaving **0 coast pixels**, whereas local gaps would have drawn **5**. The preflight itself called no pixel writer. All comparisons passed for this fixed host input; no other scene triggered the example threshold. This is a demonstration of an early decision, not a universal atomicity proof.

## Unresolved architecture and resource gates

- This original trial's `enclose(..., collect_front=True)` retains front-interval lists **per segment**; source parts and other host objects also live in memory. The later optional streaming study removes those lists in its own path, but neither avoiding them nor avoiding a whole-layer bitmap/segment list establishes the provisional ≤1 KiB incremental scratch/stack, zero-heap target or a bounded, checked runtime representation. The experiment's shared *simulated* frame is 400×240 bytes, not a Sharp packed target buffer or a coast-only layer.
- Pass 2 could fail **after** emitting ink (numeric overflow, differing input/state, exhausted work, draw-target error or an unexpected raster branch). Comparing pass summaries at the end detects drift only after damage; it cannot roll back a real draw target. A true atomic contract needs immutable per-frame inputs, fully preflighted finite work and checked arithmetic/raster paths with no possible new coast-specific failure on commit, or a separately costed rollback/repaint/buffer scheme. A platform draw failure also needs an explicit whole-frame failure policy. No such proof or recovery path exists here.
- Existing host-f64 culls/curve and viewport bounds are not target-float certified; unknowns and grazing cases remain unknown. `0.001°` / `8` are illustration values, not a justified geographic or raster omission limit. The synthetic cap is per segment, not a global invocation limit. A global cap, work exhaustion semantics, warning/presentation policy and actual foreground draw order remain open.
- **No target measurements** were taken. The provisional maximum-observed **≤5 ms** coast rebuild, compiled ≤64 KiB including ≤32 KiB data, zero added heap and ≤1 KiB measured incremental scratch/stack cannot be judged from these Python counts. Whole-device app/Sharp output, task-stack and frame headroom remain unmeasured; the nominal 50 ms period is not spare time.

**Next gate:** the [bounded streaming-enclosure follow-up](m4-coastline-streaming-enclosure-study.md) removes the per-segment front/unknown lists in an optional offline path and retains unfinished intervals as unknown when its illustrative pending cap fires. It does **not** close the target scratch or late-commit failure questions. Specify a finite, target-safe end-to-end numeric and commit path (or reject the two-pass direction), then measure it *with* a whole-device baseline and explicit failure presentation before seeking approval for export/renderer/firmware. No device work, interactive replay or implementation authorization follows from this study.

## Reproduction

With the [verified temporary archive](m4-coastline-source-preflight.md#reproduction-and-input) from repository root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/offline/coastline_atomic_fallback_study.py \
  /tmp/overhead-coastline-preflight/ne_110m_coastline.zip \
  /tmp/overhead-coastline-comparison/atomic-fallback
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools/offline -p 'test_coastline_compare.py'
cargo check --workspace
cargo test --workspace
```

Temporary `report.json` SHA-256 `b299b2343f22db9b97c3973cccadc2addd6aafe9dc86ccc64fe7f594e1c839c1`. The study ran for all 21 scenes; 26 offline Python tests, workspace check and tests passed. `git diff --check` passed for tracked changes. No hardware, firmware, asset, renderer, interactive simulator or historical M2 gates were run or changed.
