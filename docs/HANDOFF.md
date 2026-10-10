# Overhead — Handoff

_Last updated: 2026-10-10 UTC. Next: broaden target evidence; inspect boot diagnostic._

## State
M2 remains open for operating limits/cadence/scheduling. First distinct-catalogue
S3 heap and written-stack captures now exist; no production defaults selected.
Headless engine works; render stub/static simulator unchanged. [PLAN](PLAN.md).

## This session
With explicit approval, flashed the prepared `overhead-s3-catalogue-memory` binary
and captured two runs, each eight cases × three samples. All work/heap counters
and stack marks agree. Eight-entry initialization peaks at 16,024 occupied bytes;
24h catalogue/results retain 8,088; prediction takes ~14.53s. Observed stack write
depth is 5,824 bytes, not a safe stack size or maximum reserved stack.
[Raw evidence, hashes, summary and limits](evaluations/s3-catalogue-memory.md);
[method/build/capture](../firmware/CATALOGUE_MEMORY.md). No code/dependency/toolchain
changes during capture; evidence/docs only after the prepared implementation.

## Hardware / local setup
Same S3-DevKitC-1/WROOM-1 v0.2, 8 MB flash, UART `/dev/cu.usbserial-110`.
PSRAM unconfirmed/unused; display disconnected. Board now runs the catalogue-memory
matrix with three samples; reset reruns it. No monitor left running. Ask before
another flash. Measured ELF/source snapshot/build logs are ignored local artifacts
under `firmware/target/catalogue-memory-capture.r3l6jk`, not a durable binary archive.

## Next / risks
Both boots report multiple DROM segments; both complete, but the diagnostic is
unresolved and must not be assumed harmless. Existing linker RWX warning remains.
Broaden distinct populations/pass densities, duplicate/conflict cases, collection
growth and RAM input lifetimes; add independent target numeric checks and stronger
stack evidence before capacity/cadence/scheduling/over-budget choices. Flash-only
input and written-stack observations do not establish whole-device peak RAM.
Sharp remains a separate early check; no display/PSRAM work performed.

## Verification
Clean snapshot: workspace tests (150 + 6 doctests), firmware tests (6/10), 11 Python
tests, strict host/target Clippy, fmt, both target builds and regenerated manifest
pass. Saved captures/source/evidence hashes revalidate without ignored artifacts.
[Fresh-session commands](../firmware/CATALOGUE_MEMORY.md). No hardware access this check.
