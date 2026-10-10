# Overhead — Handoff

_Last updated: 2026-10-10 UTC. Next: numerical preparation and exact M2 manifest freeze._

## State
M2 remains open for operating limits/cadence/scheduling. All 20 candidate host
cases pass ingestion/work/requested-memory gates; **not** a frozen capture manifest
or target-fit claim. Corrected catalogue-memory boots pass without the multiple-DROM
diagnostic; RWX warning remains. No production defaults/UI changes. [PLAN](PLAN.md).

## This session
Added `overhead-m2-preflight`, deterministic [candidate input archive](../tools/fixtures/m2-cases/README.md),
and [host evidence](evaluations/m2-host-preflight.md). Verified error/partial cases,
retained diagnostics, actual collection observations, allocate-copy-free overlap,
dropped/retained RAM inputs and full release. All 20 cases repeat requested-memory
observations; no core API/dependency/toolchain/firmware/hardware changes. Historical
fixtures and the separate pinned pool are unchanged.

## Next / risks
Pin the 12 original TEME vectors and existing coordinate/observer selection with
unchanged tolerances and identical host/target checks. Finish metadata/diagnostic
representation review and freeze the ≤20-case manifest before target implementation.
[Contract](evaluations/m2-next-evidence.md). Host evidence measures requested bytes,
not backend occupancy/timing/stack or S3 fit. Target full-pipeline accounting,
numerical/two-pattern stack captures, safe stack, supported size and whole-device
peak RAM remain unestablished. Keep the fixed 64 KiB heap. Scheduling latency and
Sharp bring-up stay separate; weather-derived LEO inclination coverage is narrow.

## Hardware / local setup
Board unchanged; reset reruns the corrected three-sample catalogue-memory experiment.
No monitor running. S3 UART `/dev/cu.usbserial-110`; display/PSRAM unused, capacity
unconfirmed. Ask before flashing; re-gate exact normal images. [Image gate](../firmware/IMAGE_GATE.md).
Prior ignored captures: `firmware/target/corrected-boot-20261010T141219Z`.

## Verification
Clean staged-source export (no ignored artifacts) passes offline workspace check/
default/all-feature tests, no_std core, strict Clippy, fmt, firmware default/feature
host tests/Clippy, 31 Python tests and exact input/debug/release report regeneration.
Existing toolchain/cached dependencies and SDL2 required. No target images rebuilt
or hardware accessed. [Fresh-context commands](../tools/fixtures/m2-cases/README.md#offline-regeneration).
