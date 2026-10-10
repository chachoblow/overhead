# Overhead — Handoff

_Last updated: 2026-10-10 UTC. Next: full host preflight and exact M2 manifest freeze._

## State
M2 remains open for operating limits/cadence/scheduling. The separate pinned
16-object pool passes initial source suitability; **not** full preflight or a
capture-ready manifest. Corrected catalogue-memory boots pass without the
multiple-DROM diagnostic; RWX warning remains. No production defaults/UI changes. [PLAN](PLAN.md).

## This session
Archived CelesTrak source bytes/provenance and deterministic offline extraction in
[tools/fixtures/m2-pool](../tools/fixtures/m2-pool/README.md). Added
`overhead-m2-pool-preflight`: actual initialized SGP4 paths, ≤48h epochs, four
mixed growth searches and all 30 density-site candidates pass. Case 07 is densest
(68 stored passes); pool is 6,672 bytes. [Evidence/remaining gates](evaluations/m2-pool-suitability.md).
Historical fixtures unchanged; no dependency/toolchain/firmware/hardware change.

## Next / risks
Complete historical/error/partial/RAM cases, retained diagnostics, collection
capacity/allocation observations, input-copy overlap/full release and independent
numerical references; freeze exact ≤20-case manifest before target implementation.
[Contract](evaluations/m2-next-evidence.md). Weather-derived LEOs are predominantly
polar, not broad inclination coverage. Target fit, safe stack, supported catalogue
size and whole-device peak RAM remain unestablished. Keep the fixed 64 KiB heap;
review failed/redundant cases rather than silently replacing/widening them.
Scheduling latency remains pending; Sharp bring-up stays separate.

## Hardware / local setup
Board unchanged: corrected three-sample catalogue-memory experiment; reset reruns
it. No monitor running. S3-DevKitC-1 UART `/dev/cu.usbserial-110`; display/PSRAM
unused, capacity unconfirmed. Ask before flashing; re-gate exact normal images.
[Image gate](../firmware/IMAGE_GATE.md). Prior ignored artifacts:
`firmware/target/corrected-boot-20261010T141219Z`; binaries not archived.

## Verification
Clean staged-source export passes offline workspace check/default/all-feature tests,
no_std core check, strict Clippy, fmt, firmware default/feature host tests/Clippy,
28 Python tests and exact pool/report regeneration. No ignored artifacts needed;
existing toolchain/cached dependencies and SDL2 required. Target images not rebuilt.
[Fresh-checkout commands](../tools/fixtures/m2-pool/README.md#offline-regeneration-and-checks).
