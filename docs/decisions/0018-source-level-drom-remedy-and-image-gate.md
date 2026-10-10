# 0018 — Source-level DROM remedy and image gate

Date: 2026-10-10
Status: accepted

## Decision
Use a repository-local override of the pinned HAL rodata linker script to emit
file-backed alignment padding. Require the offline ELF/application-image gate
for both measurement binaries before a future approved flash, with regression
coverage for alignment stress and the original NOBITS failure.

## Why
The [offline diagnosis](../evaluations/s3-drom-diagnostic.md) isolates a linker
NOBITS gap that espflash omits. Emitting one zero byte before alignment fixes the
source of that omission without changing registry sources, dependencies or the
installed toolchain. The descriptor remains first; an already aligned boundary
costs one extra alignment unit. The compiled ELF and image, rather than linker
search-order assumptions, are the acceptance evidence.

Reject ELF surgery as a build remedy, warning suppression, and reliance on the
historical image's incidental page coverage. A dependency/toolchain upgrade is
not needed for this bounded fix and would require separate approval/revalidation.

## Consequences
- Keep the small local override synchronized explicitly with HAL upgrades.
  Target builds continue to run from `firmware/`; the build script tracks edits.
- [IMAGE_GATE.md](../../firmware/IMAGE_GATE.md) owns commands and supported image
  format. Verify descriptor/hash, single DROM, flash alignment, and all loadable
  RAM/IROM/DROM bytes; do not accept a segment-count-only test.
- Keep stress/negative fixtures out of flash/capture workflows. Re-gate the exact
  ordinary ELF if it is rebuilt; retain hashes with any new target evidence.
- No corrected target boot is claimed by this decision. Ask before flashing;
  capture the boot and bounded workload before closing the diagnostic on target.
  The separate RWX warning and M2 operating-budget questions remain open.
