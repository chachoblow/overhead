# M4 coastline — host-only cost preflight

Date: 2026-10-20. Source: baseline `2841327` plus this M4 host-verification/cost-harness change; the two tabulated runs precede the subsequent late-failure test-hook change. This **does not** measure the S3, prove the [provisional target ceilings](m4-coastline-budget-preflight.md), satisfy [0053's no-omission constraint](../decisions/0053-no-resolvable-coastline-omissions.md), or approve the asset for production.

## Reproduce

```sh
cargo test -p overhead-render --release coast_cost_preflight -- --ignored --nocapture
```

The ignored test in `render/src/coast/cost_preflight.rs` runs ten fixed wide-view scenes: SF 0.1°/1°/5°/100°, Colorado 20°, polar 20°, dateline 5°, and generated limb 20°/100° and 0.1° horizontal viewport-edge observers. It derives the boundary positions deterministically from exported segment index 46; output prints their coordinates. A single parsed ISS fixture supplies the same app/foreground path for every scene; catalogue setup and app update are **outside** the timed invocation. It is not a representative production population.

For each scene it times (a) complete `coast::draw` including asset validation and line raster writes, (b) complete `draw_frame` with coast, (c) complete `draw_frame` without coast, and (d) complete `draw_frame` given a deliberately invalid short asset. The last case validates the early rejection, clear/repaint and `COAST ERROR` path; **it does not exercise a late failure after coast ink**. All modes use the same in-memory 400×240 1-bit test target; drawing to a physical display or SDL is excluded. An untimed warm-up precedes five individual wall-clock `Instant` samples per scene/mode. Each reported min/median/max is from those five, with no outlier removal; input and output are black-boxed and an extra untimed invocation counts attempted pixel writes and final lit pixels. Timed calls repeat on the same screen (full-frame modes clear it each time). These counts include foreground/clear in frame modes, and repeated writes to the same pixel; they are **not** unique coastline pixels or CPU instruction counts. Subtracting two separately timed frame medians is illustrative, not a measured paired incremental cost. Tests use `--release` but test instrumentation and the host target remain different from device codegen and hardware.

Host: Apple M1 Pro / arm64 Darwin 25.6.0, Rust and Cargo 1.99.0. Two sequential runs without deliberately concurrent build/test workloads. All units below are **ms per invocation, median of five** for each run; full min/max and write counts are printed by the command. The first run's individual first-scene coast timing varied (max 4.986 ms), illustrating noise rather than a bound. After the late-failure test hook was added, one additional release sanity run still passed: polar 20° coast-only median 5.214 ms, SF 0.1° coast-only median 4.187 ms; SF 0.1° full-frame median 5.262 ms (versus 4.429/4.401 in the earlier runs). These are noisy observations, not an established cost delta or a target gate.

| Scene | Coast only (run 1 / run 2) | Frame + coast | Frame no coast | Frame invalid asset | Coast on-writes |
| --- | ---: | ---: | ---: | ---: | ---: |
| SF 0.1° | 4.261 / 4.039 | 4.429 / 4.401 | 0.355 / 0.349 | 0.721 / 0.682 | 242 |
| SF 1° | 4.204 / 4.050 | 4.531 / 4.381 | 0.358 / 0.343 | 0.722 / 0.677 | 362 |
| SF 5° | 4.450 / 4.293 | 4.777 / 4.630 | 0.354 / 0.343 | 0.720 / 0.686 | 5,180 |
| SF 100° | 4.863 / 4.744 | 5.236 / 5.313 | 0.350 / 0.343 | 0.726 / 0.678 | 12,714 |
| Colorado 20° | 4.948 / 4.852 | 5.221 / 5.223 | 0.356 / 0.354 | 0.707 / 0.690 | 14,393 |
| Polar 20° | 5.434 / 5.246 | 5.791 / 5.612 | 0.361 / 0.355 | 0.705 / 0.683 | 17,613 |
| Dateline 5° | 3.840 / 3.817 | 4.176 / 4.194 | 0.346 / 0.343 | 0.693 / 0.681 | 2,112 |
| Boundary limb 20° | 4.530 / 4.654 | 4.914 / 5.021 | 0.344 / 0.344 | 0.685 / 0.681 | 11,827 |
| Boundary limb 100° | 4.525 / 4.520 | 5.056 / 4.828 | 0.341 / 0.340 | 0.728 / 0.692 | 10,839 |
| Boundary viewport 0.1° | 3.765 / 3.732 | 4.085 / 4.065 | 0.343 / 0.345 | 0.687 / 0.682 | 263 |

The checked asset is **15,484 bytes**, with 134 parts, 1,868 vertices, 1,734 stored segments and 23,377 interpolation intervals traversed **on every valid rebuild**, even when very little coast is on-screen. Visibility roots use 24 iterations each when reached; this harness does not count roots, projections, cycles, stack, heap or compiled coast code/tables. The 15,484-byte asset alone fits the provisional ≤32 KiB asset screen; **incremental compiled size ≤64 KiB is not measured**. The changing raster-write counts (242–17,613) show one source of scene-dependent work; fixed validation and interval scanning remain in all scenes. The deliberately invalid path repaints and has roughly twice the full-frame clear writes; its speed says nothing about an expensive late failure.

## Gate status

Host timings cannot be compared numerically to the S3's provisional ≤5 ms observed full-coast rebuild gate: double precision, compiler, memory and target/display draw costs differ. Nor can a small host sample establish maximum device time, zero dynamic heap, ≤1 KiB target scratch increment, whole-device 20 Hz headroom or Sharp refresh throughput. No target build, firmware, flash, hardware, toolchain change or frozen-M2 refresh was performed. A separate authorized target measurement must include the **full** successful and late-failure paths, incremental image sections and scratch alongside whole-device/Sharp behavior. No appearance change follows from this preflight. A separate host regression now injects failure after emitted coast ink and verifies pixel-level rollback, but a naturally reached late numeric failure and its cost are not measured. Independently bounded visibility evidence remains open.
