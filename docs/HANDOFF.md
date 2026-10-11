# Overhead — Handoff

_Last updated: 2026-10-11 UTC. Next: initial operating-budget decision and bounded scheduling._

## State
All frozen `m2-bounded-20-v1` target gates pass: two timed boots (120 samples),
two diagnostic boots (75 numerical rows each), and 252 two-pattern phase-stack
observations. Work/heap/collection counters agree across boots/images; every
pipeline fully releases. No failed/partial capture attempt or widened tolerance.
[Results](evaluations/s3-m2.md), [raw evidence/provenance](evaluations/s3-m2/README.md).
M2 is not closed: no production limit/cadence or safe-stack claim.

## Fresh session
Read [PLAN](PLAN.md) and [DESIGN](DESIGN.md); stay in the remaining M2 scope.
Use [host/build commands](../firmware/M2_CAPTURE.md#sources-and-build) and
[evidence regeneration](evaluations/s3-m2/README.md#offline-validation-and-table-regeneration).
Tracked sources suffice: no ignored capture/build artifacts or running monitor
are required. Existing stable/Espressif toolchains, cached dependencies, Python
3.11+ and Homebrew SDL2 remain prerequisites; no tools were installed/changed.
New builds require their own image gate and approval before hardware access.

## Next / risks
Decide input/group/record/accepted-satellite/stored-result limits, lookahead,
recomputation cadence and explicit over-budget behavior. Then select and verify
bounded scheduling latency separately; no architecture selected by these captures.
Largest observed heap: 31,993 requested / 32,124 backend bytes; slowest aggregation
~25.58 s. Deepest observed phase-stack mark: 9,248 B, not a safe allocation.
Allow app/display/network/refresh headroom; whole-device RAM/safe stack and RWX
remain open. No extra matrix without an unanswered decision. Sharp is separate.

## Hardware / verification
Board holds `m2_diagnostic`; reset reruns numerical/stack diagnostics. No monitor
active; UART `/dev/cu.usbserial-110`. Display/PSRAM unused; future access needs approval.
Captured binaries/disassembly/source tarball remain local under
`firmware/target/m2-capture-20261011T0423/`; ignored files are not durable archives.
Clean staged-source export passes offline workspace check/default/all-feature tests,
no_std core, strict Clippy/fmt, all firmware host feature combinations, 52 Python
tests, exact preflight/numerics/frozen tables and four-log evidence regeneration.
Both new normal images build/gate from that export; the full historical linker
regression also passes. Captured source/input/image hashes and raw bytes are intact.
