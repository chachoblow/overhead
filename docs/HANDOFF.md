# Overhead — Handoff

_Last updated: 2026-10-09. Next: broader target measurements and operating policy._

## State
Headless engine and host benchmarks complete; first S3 kernel baseline now runs
on hardware. M2 remains open for supported limits/cadence/scheduling. Render stub,
sim static; `firmware/` is measurement-only, not the device app. [PLAN](PLAN.md).

## This session
Added [S3 benchmark firmware](../firmware/README.md), host-generated original
fixtures, and bounded capture/validation. No core/fixture changes.
[Results/raw capture](evaluations/s3-costs.md): 14 workloads × 5 samples; all work
counts match host baseline. 24h/60s searches ~1.18–2.14s per fixture; four-slot
search ~6.54s; full tracking ~4.77ms per four-slot tick (amortized). No defaults chosen.

## Hardware / local setup
User approved toolchain installation and benchmark flashing this session.
S3-DevKitC-1/WROOM-1, revision v0.2, 8 MB flash confirmed; PSRAM/N8R8 unconfirmed.
UART/CP2102N works at `/dev/cu.usbserial-110` directly connected to laptop; dock
path did not enumerate. Display untouched, not wired; no soldering needed yet.
Installed espup 0.18.0, espflash 4.6.0, Xtensa Rust 1.97.0.0; stable stays default.
Source `$HOME/export-esp.sh`, build from `firmware/`; own lock/config/toolchain.
Benchmark replaced the preloaded demo and reruns on reset. No monitor left running.

## Next step / risks
First explain the results/product implications; user deferred that discussion.
Then broaden ages/orbits/search settings and measure catalogue/peak memory;
choose cadence/scheduling/over-budget policy after that evidence. Current searches
are synchronous and cannot share a responsive display loop.
One recorded final-build run is not a worst-case bound; four fixture slots are
not catalogue capacity. f64 uses software math; no precision/model changes made.
16/64-slot target timings, independent target numeric checks, stack watermark,
and PSRAM capacity remain pending. [Detection limits](evaluations/detection-intervals.md)
still apply. Minimal Sharp output remains a separate early hardware check.

## Verification
Workspace check/default/all-feature tests pass (145 tests + 6 doctests); standalone
core feature checks, strict host Clippy, fmt, and diff checks pass. Two new firmware
host tests and three capture tests pass; target check/release build/flash and
complete 70-sample capture pass. Embedded release linker emits an RWX LOAD warning.
