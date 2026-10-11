# Overhead — Handoff

_Last updated: 2026-10-11 UTC. Next: refine the minimal real-data M3 simulator slice._

## State / agreed direction
[0021](decisions/0021-radar-first-iteration-on-verified-engine.md) permits visual
iteration on the verified engine before remaining M2 budgets/scheduling are closed.
Radar quality comes first; catalogue size and information-area contents/proportions
need visual feedback. Prediction stays tested and available, but secondary.
No proposed numeric operating package was accepted; M2 remains open.
Updated [PLAN](PLAN.md), [DESIGN](DESIGN.md) and amendment links in 0003/0005.
Current `sim/src/main.rs` is static; `render/src/lib.rs` is still a stub.

## Fresh session / next step
Read PLAN, DESIGN and 0021. Refine the M3 slice before implementing: shared app
state/rendering, explicit clock/observer, configurable local real catalogue,
projection/clipping/zoom, real markers and basic identification. Use existing
physical APIs and preserve explicit failures; presentation cannot alter true data.
Compare density/layout on the 400×240 view; no fixed tens/hundreds target or
mandatory countdown. Live fetching/cache/refresh/time sync remain M6.
Use [workspace commands](../AGENTS.md#commands) and [local catalogue tools](../tools/README.md).

## Evidence / open risks
All frozen `m2-bounded-20-v1` S3 gates pass; preserve sources, fixtures and evidence.
[Results](evaluations/s3-m2.md), [raw provenance/regeneration](evaluations/s3-m2/README.md),
[host/build commands](../firmware/M2_CAPTURE.md#sources-and-build).
Measured workloads are not general capacity or whole-device guarantees. Production
limits/cadences/overload behavior, bounded scheduling, whole-device RAM/safe stack
and RWX remain open. Simulator visual success is not target performance evidence.
No extra measurement matrix without a specific unanswered question; Sharp is separate.

## Hardware / verification
No hardware accessed this session. Board holds `m2_diagnostic`; reset reruns it.
No monitor active; UART `/dev/cu.usbserial-110`. Display/PSRAM remain unused;
new hardware access requires approval and new images require their own image gate.
Documentation-only changes verified from a clean staged-source export: offline
workspace check/default/all-feature tests, no_std core, strict Clippy/fmt, simulator
build, catalogue smoke test, 52 Python tests and frozen manifest/capture tables pass.
No simulator window opened or target images rebuilt. Prior firmware/image gates
remain historical evidence. Tracked sources suffice; ignored binaries are not archives.
Existing toolchains/cached dependencies, Python 3.11+ and Homebrew SDL2 are prerequisites.
