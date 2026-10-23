# Overhead — Handoff

_Last updated: 2026-10-23 UTC. M4 coastline appearance accepted; Stage A S3 timing gate failed._

## State
- [M3 complete](PLAN.md#m3--minimal-real-data-radar-complete); M4 [0048 notices](decisions/0048-refined-empty-view-notices.md) and [0049 selection](decisions/0049-stable-wide-selection.md) are implemented and user-accepted. [0050](decisions/0050-keep-range-in-debug-and-defer-display-prediction.md) defers display prediction; M2 operating budgets/scheduling and Sharp bring-up remain open.
- The [0054 wide-only 40 km coastline](decisions/0054-first-executable-coastline-comparison.md) and exceptional `COAST ERROR` are implemented and visually accepted, **not adopted for the device**. [0053](decisions/0053-no-resolvable-coastline-omissions.md) treats a reproduced ordinary visible gap as a bug; [0055](decisions/0055-defer-all-view-coastline-proof-for-m4.md) defers all-view proof *for this iteration*, not correctness. Square/circular output and foreground truth remain unchanged.

## Session result
- In `firmware/src/coast_main.rs`, `firmware/src/coast_measurement.rs` and `render/src/measurement.rs`, fixed the pinned allocator call, made the off control coast-code-free in release, added byte-for-byte selected/empty and warning/no-warning late-failure controls, and measured coast on a 1-bit RAM target without changing production rendering. [Protocol, ELF/image hashes, gates and caveats](evaluations/m4-coastline-device-measurement-protocol.md); [retained raw logs and validator](evaluations/m4-coastline-s3/README.md).
- Separately approved on/off Xtensa ELFs and exact application images passed offline gates; off excludes coastline code/asset. Incremental `.rodata` + `.text` is **65,020 B**, only 516 B below the provisional 64 KiB screen. One boot each, 11 observer/FOV scenes × five samples: maximum complete coast rebuild **9,781,141 µs at 240 MHz**, against **5,000 µs** provisional gate: **STOP**. FrameOff outputs agree across images; all four late pixel controls matched. No independent coast geographic pixel oracle.
- The ESP32-S3 was left running the coast-on *measurement* image. The Sharp remains disconnected. No whole-device, live stack-safety or 20 Hz claim follows. Frozen M2 evidence remains pinned separately ([instructions](evaluations/s3-m2/README.md)); its results were not refreshed.

## One next bounded task
- Propose an **offline performance/algorithm feasibility review** explaining the multi-second coastline rebuild with source/profiling evidence before choosing a candidate revision. Preserve 0053's no-known-visible-omission rule; do not substitute routine `COAST ERROR`/dropout, restart the all-view proof without a new bug, or start Sharp/device work on this failed candidate. Any new Xtensa build, flash, board access or toolchain change requires separate approval and exact-image gating.

## Verification and limits
- This wrap-up: workspace check/tests, renderer measurement-feature tests and no-default check, firmware default/feature host tests, formatting, raw-log audit, whitespace and local-link checks passed (see [protocol](evaluations/m4-coastline-device-measurement-protocol.md)). S3 captures/image gates passed their controls; **timing failed**. Release ELFs/images remain ignored local build outputs, while source and raw serial logs are committed. No Sharp access, frozen-M2 rerun or further flash was part of wrap-up.
