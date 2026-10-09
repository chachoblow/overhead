# Overhead — Handoff

_Last updated: 2026-10-09. Next: remaining target suites and catalogue/peak memory._

## State
M2 remains open for measured operating limits/cadence/scheduling. Headless engine
works; render stub and static sim unchanged. Firmware is measurement-only.
[PLAN](PLAN.md); no production defaults selected.

## This session
Expanded [firmware harness](../firmware/README.md): separate 16/64-slot, signed-age,
and search-setting suites; host-generated expected counts; strict V2 capture
validation retaining V1 compatibility. All 122 workloads complete on the host.
With explicit approval, flashed/captured `scaling-16`, three samples per workload.
[Results and raw evidence](evaluations/s3-scaling-16.md): tracking 19.19ms per
16-slot tick (amortized); 24h prediction 27.34s. All six samples match host counts.
Core, historical fixtures, dependencies, and toolchain unchanged.

## Hardware / local setup
S3-DevKitC-1/WROOM-1, revision v0.2, 8 MB flash; PSRAM unconfirmed/unused.
Direct UART `/dev/cu.usbserial-110` works; prior dock path did not enumerate.
16-slot benchmark now flashed and reruns on reset; no monitor left running.
Display untouched/disconnected. Build/flash/capture details in firmware README.
Ask before another flash; this session authorized only the 16-slot run.

## Next / risks
Measure signed ages/search settings and 64-slot scaling; rerun V2 baseline for
a same-harness comparison. Broader orbit samples, independent target numeric
checks, distinct-catalogue storage/aggregation, and peak RAM/stack remain pending.
Repeated slots are not capacity. All seven age grids remain linked in flash;
type/ELF sizes are not peak memory. No worst-case bound or display cadence proved.
Prediction remains synchronous; select scheduling/caching/over-budget policy
only after broader evidence. Minimal Sharp output is a separate early check.

## Verification
Workspace check/tests pass (145 tests + 6 doctests). Six firmware host tests,
seven Python tests, strict firmware-host Clippy, fmt/diff checks, all-suite host
manifest round trips, target release builds (ages-heo/scaling-16), approved flash,
and complete capture pass. Existing embedded RWX LOAD linker warning remains.
