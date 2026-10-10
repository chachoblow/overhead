# Overhead — Handoff

_Last updated: 2026-10-10 UTC. Next: source-level DROM remedy/image gate; refine bounded M2 evidence proposal._

## State
M2 remains open for operating limits/cadence/scheduling. First distinct-catalogue
S3 heap and written-stack captures exist; no production defaults selected.
Headless engine works; render stub/static simulator unchanged. [PLAN](PLAN.md).

## This session
Offline investigation reproduced the multiple-DROM cause: a 32-byte NOBITS
alignment gap is skipped by espflash. A disposable ELF section-type counterfactual
produces one DROM segment; it is not a source fix or flash candidate. Page mapping
explains why this particular captured layout completes, not arbitrary-image safety.
[Evidence, source references and next validation](evaluations/s3-drom-diagnostic.md).
[Proposed bounded follow-up](evaluations/m2-next-evidence.md): at most 20 memory/work
cases plus separate independent numeric and stronger stack gates; not yet accepted
or implemented. Existing decisions/contracts and historical measurements unchanged.

## Hardware / local setup
No hardware accessed, port opened, reset or flash this session. Board remains on
the three-sample catalogue-memory matrix; ask before flashing. Same S3-DevKitC-1,
UART `/dev/cu.usbserial-110`; display disconnected, PSRAM unconfirmed/unused.
Measured ELF remains under ignored `firmware/target/catalogue-memory-capture.r3l6jk`;
forensic images under `/tmp/overhead-drom-investigation` are disposable, not archived.

## Next / risks
Prepare a source-level remedy and single-DROM image gate for both firmware binaries
and alignment stress; no registry/toolchain patch or warning suppression adopted.
A corrected build still needs approved target boot/capture. RWX warning remains.
Refine/freeze the proposed evidence contract before broadening measurements.
No supported catalogue size, safe stack size or whole-device peak RAM established.
Sharp validation remains a separate early check; scheduling requires latency evidence.

## Verification
Clean snapshot without ignored artifacts: workspace/default/all-feature tests,
firmware host tests (6/10), 11 Python tests, strict host/catalogue-target Clippy,
fmt, both target builds, regenerated manifest and saved capture/hash checks pass.
Original-ELF forensic checks are in the linked investigation; binaries are not archived.
[Fresh-session build/test commands](../firmware/CATALOGUE_MEMORY.md). No hardware access.
