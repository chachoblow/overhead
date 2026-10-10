# Overhead — Handoff

_Last updated: 2026-10-10 UTC. Next: target catalogue and peak-RAM/stack experiment._

## State
M2 remains open for measured operating limits/cadence/scheduling. Headless engine
works; render stub and static sim unchanged. Firmware is measurement-only.
[PLAN](PLAN.md); no production defaults selected.

## This session
Added `tools/src/bin/overhead-catalogue-benchmark.rs`, its scoped requested-heap
instrumentation, tests, and eight distinct historical inputs in
`tools/fixtures/catalogue-costs.tle`. Two host release runs agree on all work and
allocation counts: eight-entry catalogue retains 5,927 bytes, 24h aggregation adds
2,560; initialization peaks at 14,279 requested bytes, excluding prepared input.
[Method, raw evidence, hashes, limits](evaluations/catalogue-costs.md);
[usage](../tools/README.md#overhead-catalogue-benchmark). Core, old fixtures,
dependencies, firmware, hardware, and toolchain unchanged.

## Hardware / local setup
Same S3-DevKitC-1/WROOM-1 v0.2, 8 MB flash; PSRAM unconfirmed/unused.
Direct UART `/dev/cu.usbserial-110`; display untouched/disconnected.
Last recorded image: `search-gnss` (three samples), reruns on reset; no monitor
started this session. Ask before flashing. Prior target evidence:
[S3 sweeps](evaluations/s3-expanded.md); [build/capture](../firmware/README.md).
Prior ELFs/build logs remain ignored local artifacts, not a durable archive.

## Next / risks
Plan bounded S3 catalogue/allocator and stack-high-water instrumentation, with
explicit input/storage lifetimes; firmware currently has no allocator/catalogue.
Host requested heap excludes allocator overhead, stack/static RAM, and input JSON;
it is not S3 capacity. Broaden distinct samples, pass density, collection growth,
and independent target numeric checks. Prediction remains synchronous; choose
scheduling/caching/over-budget policy and cadence only after broader evidence.
No worst-case bound proved. Minimal Sharp output remains a separate early check.

## Verification
Workspace check/tests pass (150 tests + 6 doctests), targeted strict Clippy and
workspace fmt pass. Two release runs and pinned-source line comparison pass.
Clean staged snapshot checks/tests and evidence/link validation also pass.
Firmware checks/captures not rerun; no hardware access or flash this session.
