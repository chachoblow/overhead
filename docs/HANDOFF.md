# Overhead — Handoff

_Last updated: 2026-10-10 UTC. Next: refine/freeze bounded M2 evidence proposal._

## State
M2 remains open for operating limits/cadence/scheduling. DROM remedy/image gate
pass offline; two approved corrected catalogue-memory boots/captures now pass
without the multiple-DROM diagnostic. Separate RWX warning remains. No production
defaults; render stub/static simulator unchanged. [PLAN](PLAN.md).

## This session
Gated a normal build, then flashed/captured twice with explicit approval: 48 samples
validate; boot segments match the gate, work/heap/stack match historical records.
[Evidence, raw logs and artifact report](evaluations/s3-drom-diagnostic.md#corrected-normal-build-target-verification).
No code/dependency/toolchain changes. Corrected kernel binary not boot-tested.

## Hardware / local setup
Board now holds the corrected three-sample catalogue-memory experiment; reset
reruns it. No monitor left running. S3-DevKitC-1 UART `/dev/cu.usbserial-110`;
display/PSRAM unused, PSRAM capacity unconfirmed. Ask before any future flash.
Ignored ELF/image/build/flash logs/disassembly:
`firmware/target/corrected-boot-20261010T141219Z`. Binaries are not archived.
Prepare and gate the exact normal ELF again for any future build/flash;
stress/minimal/negative fixtures are never flash candidates. [Usage](../firmware/IMAGE_GATE.md).

## Next / risks
Refine/freeze the [next-evidence proposal](evaluations/m2-next-evidence.md) before
implementation or broader captures; it remains unaccepted/unimplemented. No
supported catalogue size, safe stack size or whole-device peak RAM established.
Independent target numeric checks and bounded scheduling latency remain pending.
Sharp bring-up stays separate. Review the local linker override on upgrades.

## Verification / fresh-session commands
Source snapshot without ignored artifacts passes workspace check/default/all-feature
tests, firmware host tests (6/10), strict firmware host Clippy, formatting, 23 Python
tests and all image regression cases. Saved capture hashes/matrices/boot inventories,
measured code/input hashes, regenerated host manifest and doc links validate.
[Offline commands](../firmware/README.md#offline-checks), [feature checks](../firmware/CATALOGUE_MEMORY.md#offline-verification--next-step),
[image regression](../firmware/IMAGE_GATE.md#complete-regression-run--no-hardware).
Use installed toolchain/SDL2; no historical ignored artifacts or hardware needed.
