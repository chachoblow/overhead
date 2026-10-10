# Overhead — Handoff

_Last updated: 2026-10-10 UTC. Next: distinct-catalogue and peak-memory measurements._

## State
M2 remains open for measured operating limits/cadence/scheduling. Headless engine
works; render stub and static sim unchanged. Firmware is measurement-only.
[PLAN](PLAN.md); no production defaults selected.

## This session
With explicit approval, flashed/captured the remaining ten V2 suites: baseline,
64-slot scaling, all signed-age and search-setting sweeps. All 360 samples match
host work counts; all 122 prepared workloads now have target evidence including
the prior 16-slot run. [Results, CSV, hashes, and raw evidence](evaluations/s3-expanded.md).
64-slot tracking averages 76.69ms/tick; 24h prediction 109.37s. HEO search grows
from 2.35s near epoch to ~21s at ±30 days despite equal evaluation counts.
Core, harness, historical fixtures, dependencies, and toolchain unchanged.

## Hardware / local setup
Same S3-DevKitC-1/WROOM-1 v0.2, 8 MB flash; PSRAM unconfirmed/unused.
Direct UART `/dev/cu.usbserial-110`; display untouched/disconnected.
`search-gnss` (three samples) is flashed and reruns on reset; no monitor running.
Ask before another flash. ELFs/build logs retained locally under ignored
`firmware/target/measurements/2026-10-10-remaining/`, not a durable archive.
Build/flash/capture instructions: [firmware README](../firmware/README.md).

## Next / risks
Measure distinct-catalogue initialization/storage, pass aggregation, and peak
RAM/stack; broaden orbital samples and independent target numeric checks.
Repeated slots are not catalogue capacity; section/type sizes are not peak RAM.
Age and search settings were swept separately, not as a Cartesian product.
Prediction remains synchronous. Choose scheduling/caching/over-budget policy
and cadence only after broader evidence; no worst-case bound proved.
Minimal Sharp output remains a separate early check.

## Verification
Workspace check/tests pass (145 tests + 6 doctests), six firmware host tests,
seven Python tests, strict firmware-host Clippy, and firmware fmt pass.
Ten target release builds/flashes/captures pass; known RWX linker warning remains.
Clean staged snapshot checks and evidence validation pass without local measurement artifacts.
