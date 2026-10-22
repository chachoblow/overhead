# M4 coastline — selected bounded segment screen-effect study

Date: 2026-10-20. Scope: **host-only selected-segment and seven-scene full-export checks**, not an all-observer no-omission proof, production algorithm or new asset. Run:

```sh
cargo test -p overhead-render visibility_bound_study -- --nocapture
```

`render/src/coast/visibility_bound_study.rs` independently subdivides each *stored segment* rather than reusing the renderer's 0.2° intervals or its endpoint/midpoint visibility classification. For each subinterval, it uses coarse WGS-84 angular derivative envelopes: ≤7,000 km/rad for each coordinate derivative of the surface position, ≤1 rad/rad for the unit normal, and <100,000 km camera-to-surface distance for these zero-altitude observers. From the midpoint and angular half-width it encloses ECEF travel and the facing test; it also bounds perspective-ratio drift using a lower bound on depth. Only intervals demonstrably behind the limb or outside the rectangle can be excluded. A front-facing interval entirely inside the viewport counts a reference pixel only when its projected enclosure falls inside **one renderer raster cell**. Every other interval, including limit exhaustion, remains **unknown**, never silently hidden. For unknown intervals with a finite projection enclosure, the test also collects **all possibly affected raster cells**, whether or not the interval is actually visible. At the full-export sites it can stop refining one such interval only after checking that every possibly affected cell already has the isolated segment's ink within the stated tolerance; visibility stays unknown, but no screen cell in the bound is left uncovered. The test compares reference pixels with the **isolated** renderer segment (no neighboring geography can mask a gap), allowing the previously used three-pixel chord/raster neighborhood. Erasing a 7×7 patch around one reference pixel is a negative control for the **same comparator**. A separate falsification test checks that sampled points never contradict definite hidden/outside/pixel classifications at stratified polar and lower-latitude sites; it is not a continuum proof.

The analytic derivative constants are deliberately generous, but the implementation uses ordinary host `f64` with additive 0.01-km facing / 1e-6-km position and 1e-9 normalized-projection margins. It does **not** use directed-rounding interval arithmetic or formally establish those margins cover all floating-point error. It also shares WGS-84 camera point evaluation, stored-coordinate interpolation and the checked export with the renderer; independence is specifically from the renderer's **finite visibility/sampling decision**, not from source geography or projection implementation. A reference pixel is a bounded cell for the simplified exported segment, not a pixel-perfect Natural Earth coastline claim. The 3px allowance does not prove an exact raster match or resolve a shorter run hidden inside that allowance.

| Observer derived from segment midpoint | FOV | Reference visible cells | Hidden / outside / unknown leaves | Result |
| --- | ---: | ---: | ---: | --- |
| Centered | 0.1° | 406 | 0 / 0 / 925 | No qualifying missing cell |
| Near horizontal viewport edge | 0.1° | 231 | 0 / 21 / 9,932 | No qualifying missing cell |
| Near facing limb | 100° | 1 | 25 / 0 / 56 | No qualifying missing cell |

Four additional checked-export segments were selected from a deterministic exploratory sweep of 240 near-viewport scenarios, rather than treating a long visible run as evidence for short ones. The sweep harness was not retained; the selected site constructors and assertions below are retained and reproducible. Their observers are constructed from positions 5% or 95% along each stored segment, with the 0.1° horizontal viewport boundary solved by bisection and moved 0.001–0.004° outward. Isolated comparison here uses a **one-pixel** allowance so the earlier three-pixel allowance cannot mask a two-pixel run:

| Segment | Reference cells | Unknown leaves | Unknown fraction of segment parameter | Missing within 1px |
| ---: | ---: | ---: | ---: | ---: |
| 552 | 2 | 70,043 | 0.016700 | 0 |
| 575 | 5 | 48,778 | 0.011630 | 0 |
| 621 | 7 | 62,240 | 0.014839 | 0 |
| 667 | 6 | 87,935 | 0.020965 | 0 |

These are deterministic test observations, **not coverage percentages**: a leaf is not a uniform geographic or screen area, and the one limb cell is particularly weak evidence. `unknown fraction` is the sum of unknown interval parameter widths divided by the entire stored segment; it does **not** establish whether these intervals have screen effects. The subdivision caps at depth 22 and 200,000 nodes; unresolved boundary leaves stay unknown. In these four cases, **all 6–12 possibly affected pixels per segment** (including unknown intervals) were within **one pixel** of the isolated renderer's ink; none hit the unbounded-work path. The sweep found no qualifying miss within its 3px control, but those screened scenarios are not themselves a bounded whole-asset test. These selected checks establish falsifiable screen claims even about two-cell edge runs without equating fixed-density sampling with proof.

A regular (not ignored) `named_scene_all_export_segments_preflight` also visits **all 1,734 exported segments individually** at each of seven zero-altitude observer/FOV pairs, retaining a 3px chord/raster allowance. Unlike the selected reference-cell tests, this control asks whether *every screen cell that any unresolved interval could affect* is already covered by that segment's own ink. It short-circuits an unresolved interval only after its **entire projected pixel enclosure** is covered, rather than treating unknown visibility as hidden. Neither its point classification nor its pixel envelope is formally rounded.

| Observer / FOV | Reference cells found before covered-unknown short circuit | Possible cells bounded and covered | Non-enclosed unknown leaves | Suspected segment gaps |
| --- | ---: | ---: | ---: | ---: |
| San Francisco / 0.1° | 0 | 1,819 | 0 | 0 |
| San Francisco / 1° | 0 | 1,942 | 0 | 0 |
| San Francisco / 5° | 0 | 16,181 | 0 | 0 |
| San Francisco / 20° | 3 | 17,101 | 0 | 0 |
| San Francisco / 100° | 294 | 1,958 | 0 | 0 |
| Polar / 20° | 4 | 22,085 | 0 | 0 |
| Dateline / 5° | 0 | 6,990 | 0 | 0 |

Counts sum per-segment cells and may count the same display coordinate repeatedly. Zero reference cells in a covered-unknown shortcut **does not mean no geography was visible**: that path asks a different (stronger-for-the-scene) question—whether *even the upper bound on possible screen effects* is already near existing ink. A blank-ink negative control at each of the seven scenes confirms the coverage shortcut cannot turn erased segment ink into a passing check: it reports missing possible cells or leaves the case unresolved. The four short-run regressions still require definite reference pixels. This seven-scene check covers the full simplified asset **only for those scenes**, not the continuum of observers, FOVs, observer altitudes or arithmetic boundary cases. These tests **do not close** [0053](../decisions/0053-no-resolvable-coastline-omissions.md): about 1–2% of each short-run segment's parameter remains unknown as to physical visibility even though its possible screen effect is covered, a 1px match is not an exact raster, observers and zooms are not exhaustively covered, and the reference uses unproven f64 error margins. Next evidence would require stronger rounding guarantees and either resolving screen-relevant unknowns or a defensible bound covering the supported domain; any actual missing cell should be recorded by segment/observer before proposing a renderer change. Target resource, compiled size and physical-display gates are separate ([host cost preflight](m4-coastline-host-cost-preflight.md)).
