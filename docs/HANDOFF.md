# Overhead — Handoff

_Last updated: 2026-10-10 UTC. Next: pinned fixtures/host preflight and exact M2 manifest freeze._

## State
M2 remains open for operating limits/cadence/scheduling. Bounded evidence preparation
scope is accepted, **not** a capture-ready manifest or implemented expanded suite.
Corrected catalogue-memory boots pass without the multiple-DROM diagnostic; separate
RWX warning remains. No production defaults; render stub/static simulator unchanged.
[PLAN](PLAN.md).

## This session
Reviewed the existing harness, ingestion contracts and pinned upstream verification
set. User chose a separate pinned real-orbit cohort, preserving historical controls.
[0019](decisions/0019-bounded-m2-evidence-preparation.md) accepts the preparation scope;
[contract](evaluations/m2-next-evidence.md) bounds cases, inputs, site selection,
error outcomes, RAM lifetimes and separate numeric/stack gates. No new orbital
snapshot, code, dependency/toolchain change or hardware capture; offline rebuilds pass.

## Next / risks
Prepare the 16-object source pool and host preflight; freeze exact identities,
bytes/hashes, sites, diagnostics, work counts and numerical references before
extending the target harness. Source suitability and fit are not established.
Keep ≤20 cases and the fixed 64 KiB heap; review failures rather than silently
relaxing the contract. No supported catalogue size, safe stack size or whole-device
peak RAM established. Scheduling latency and independent target numeric checks
remain pending. Sharp bring-up stays separate; review linker override on upgrades.

## Hardware / local setup
Board still holds the corrected three-sample catalogue-memory experiment; reset
reruns it. No monitor left running. S3-DevKitC-1 UART `/dev/cu.usbserial-110`;
display/PSRAM unused, capacity unconfirmed. Ask before flashing. Ignored build
artifacts: `firmware/target/corrected-boot-20261010T141219Z`; binaries not archived.
Re-gate the exact normal image before future flashing: [usage](../firmware/IMAGE_GATE.md).

## Verification
Clean-source export passes workspace check/default/all-feature tests, no_std core
check, firmware host tests/Clippy, fmt, 23 Python tests and all image regressions.
Regenerated manifest and doc file links pass. Installed toolchain/SDL2 required;
no ignored artifacts/hardware needed. [Host](../firmware/README.md#offline-checks) / [feature](../firmware/CATALOGUE_MEMORY.md#offline-verification--next-step) / [image](../firmware/IMAGE_GATE.md#complete-regression-run--no-hardware) commands.
