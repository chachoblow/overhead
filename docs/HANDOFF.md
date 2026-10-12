# Overhead — Handoff

_Last updated: 2026-10-12 UTC. Next: interactive M3 radar review._

## State / changes
The first real-data M3 simulator slice is implemented under
[0021](decisions/0021-radar-first-iteration-on-verified-engine.md) and
[0022](decisions/0022-minimal-ground-map-simulator.md). The user chose map-like
satellite ground tracks over a sky chart; the initial projection is orthographic.
`app/` holds no_std + alloc clockless state/projection/zoom/identification;
`render/src/lib.rs` draws the 1-bit frame; `sim/` supplies explicit replay inputs,
SDL controls and deterministic PNG snapshots. Core, prediction and firmware
sources/fixtures are unchanged. No new third-party library/version.

## Next step
Run `cargo run -p overhead-sim -- --demo`, then the denser weather/GPS example
in [sim/README.md](../sim/README.md). Review moving-marker legibility, wheel/arrow
zoom, Tab identification and 200/240-pixel radar allocations with the user before
fixing density or panel contents. Initial inspected snapshots show 4 visible
markers for the 16-object pool, 22 for weather/GPS at 8,000 km and 6 at 2,500 km.
Not accepted catalogue sizes or target capacity. M3 feedback remains open in
[PLAN](PLAN.md); do not jump into M4/M5 polish.

## Contracts / open risks
Markers use ground positions, true elevation > 0, then map/view clipping. Map
radius is projected distance; readouts remain true. Failures replace old markers
and stay explicit. Sources are historical local data, not live tracking.
M2 production limits/cadences/overload/scheduling and whole-device RAM/safe stack
remain open. Preserve [frozen S3 evidence](evaluations/s3-m2.md); no new measurement
matrix without a specific question. Sharp bring-up and RWX remain separate.

## Verification / hardware
Clean staged-source export, empty build directory: offline locked workspace
check/default/all-feature tests, core/app/render no-default checks, Clippy/fmt,
simulator build/help/snapshots and outside-checkout demo pass. SDL dummy loop
passes; interactive input review pending. Cached tools/dependencies and SDL2 required.
Per [0023](decisions/0023-frozen-m2-revision-isolation.md), current Cargo.lock
intentionally fails the old freeze. At pinned revision `ab267ae`, all 52 Python
tests, frozen preparation gates and capture-derived tables pass unchanged.
No hardware/target build/toolchain change; board retains diagnostic image, no monitor active.
