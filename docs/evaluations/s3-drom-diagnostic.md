# S3 multiple-DROM diagnostic — offline investigation

Investigated 2026-10-10 UTC. **Source remedy and image gate now pass offline;
corrected target boot/capture still pending approval.** Original captures, hashes
and measured costs remain unchanged.
This is a follow-up to [the catalogue-memory capture](s3-catalogue-memory.md),
not grounds to treat arbitrary multiple-DROM images as safe.

## Evidence

The saved `benchmark.elf` still matches the recorded SHA-256
`6d67a77de3baa5aa5b3a3c7337095623d69160fb618a9e97e61ae86aeccec47b`.
Installed espflash 4.6.0 regenerated a 368,848-byte application image without
hardware access. All seven segment addresses/sizes match both saved boot logs;
the image checksum and appended SHA-256 validate. Regenerated image SHA-256:
`128181a72e3c350dd18ba0ccab8e31b70b1d73dba591a6726c7bf0aef4aeb4f7`.
This is reconstruction from the saved ELF, not a flash readback.

Relevant ELF sections (`readelf -SWl`):

| Section | Type | Address | Size | Alignment |
|---|---|---:|---:|---:|
| `.flash.appdesc` | PROGBITS | `0x3c000020` | `0x100` | 4 |
| `.rodata_merge` | NOBITS | `0x3c000120` | `0x20` | 4 |
| `.rodata` | PROGBITS | `0x3c000140` | `0x14538` | 64 |
| `.eh_frame_hdr` | PROGBITS | `0x3c014678` | `0x1c` | 4 |

One ELF LOAD program header includes all four, but espflash extracts **sections**.
Its `segments()` accepts PROGBITS/INIT_ARRAY, not NOBITS. Its merge function only
bridges gaps up to the preceding end's four-byte alignment. Consequently the
32-byte `.rodata_merge` gap is skipped, splitting the descriptor from rodata:

| DROM image segment | Flash address | Virtual address | Bytes |
|---|---:|---:|---:|
| Descriptor | `0x10020` | `0x3c000020` | 256 |
| Rodata + frame header | `0x10140` | `0x3c000140` | 83,284 |

The HAL linker script explicitly intends `.rodata_merge` to prevent this split,
but its location-counter-only section is NOBITS in this GCC-linked ELF.

## Controlled counterfactual

In a disposable copy only, changed `.rodata_merge`'s section type from NOBITS to
PROGBITS. This changes one ELF byte; the gap's existing 32 file bytes are zero.
Re-running the same espflash command produces five image segments with a single
DROM range: `0x3c000020`, length `0x14674`. The image stays 368,848 bytes; RAM
segment packing and padding change. All original non-padding address contents
are preserved except the automatically regenerated ELF-SHA field in the descriptor.
The checksum and appended hash validate. Counterfactual image SHA-256:
`3429f0359a97cbc623eebdc18698c37858bded04f683ba7b2274c06e85ed55d7`.

This isolates the section-type/gap mechanism. **The edited ELF is not a fix or
flash candidate.** No measured artifact, registry source, dependency, toolchain,
linker script or board state was changed.

## Why this image completes

The boot log identifies ESP-IDF commit `14f663f003e`. Its bootloader selects the
last DROM segment, logs the diagnostic, then aligns the selected flash/virtual
addresses down to MMU page boundaries and extends the mapping length by the
removed virtual offset. This image's descriptor requests 64 KiB pages.

Here the last segment maps flash `0x10000` to virtual `0x3c000000`, covering two
pages through virtual `0x3c020000` (exclusive). The descriptor is in that same
first page with the same flash-to-virtual offset. Thus the source-level mapping
calculation covers both DROM sections despite the diagnostic. This explains the
successful runs without assuming the bootloader separately maps every segment.
It is not a runtime MMU readback, proof for future layouts, or a general waiver.
The separate RWX linker warning is not addressed by this investigation.

## Reproduce / next

Use the saved ELF path from [the artifact manifest](s3-catalogue-memory-artifacts.json),
verify its hash first, then run (no port or flash operation):

```sh
xtensa-esp32s3-elf-readelf -SWl "$ELF"
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --skip-update-check "$ELF" /tmp/catalogue-original.bin
```

If the historical ELF is unavailable in a fresh checkout, use the existing
[build/test instructions](../../firmware/CATALOGUE_MEMORY.md#build-and-capture)
to build `overhead-s3-catalogue-memory` with the installed toolchain, then inspect
`firmware/target/xtensa-esp32s3-none-elf/release/overhead-s3-catalogue-memory` using
the commands above. Do not run the flash/capture steps without approval. This
reproduces a current build for investigation, not the historical ELF hash (build
paths and descriptor metadata can differ). Compare section types, alignment and
image structure; a byte-identical rebuild is not a prerequisite for the remedy.
Checked-in source hashes, manifests and raw logs permit historical evidence checks
without any ignored artifacts. Do not depend on `/tmp` files for subsequent work.

The application image starts with a 24-byte header; byte 1 gives the segment
count. Each segment has little-endian u32 address/length then its payload.
Add the captured application partition offset `0x10000` to payload file offsets
to compare with boot-log physical addresses. Do not confuse ELF program headers
with this application-image inventory. ELF/binaries remain ignored local artifacts,
not a durable binary archive; current scratch files are under
`/tmp/overhead-drom-investigation`.

## Source remedy and gate — subsequent offline validation

The repository-local `firmware/rodata.x` now emits `BYTE(0)` before the existing
alignment expression. This makes the gap PROGBITS without mutating the ELF or
registry. GNU ld's current-directory INCLUDE lookup selects the override; the
build script tracks it. Rationale: [0018](../decisions/0018-source-level-drom-remedy-and-image-gate.md).
Usage, scope and regeneration: [IMAGE_GATE.md](../../firmware/IMAGE_GATE.md).

Eight positive cases pass the new ELF/image gate with installed espflash 4.6.0
and the unchanged toolchain. All have one DROM segment beginning at `0x3c000020`:

| Build / fixture | Rodata alignment | File-backed merge bytes | Image bytes |
|---|---:|---:|---:|
| Kernel, baseline / 3 samples | 8 | 8 | 655,536 |
| Catalogue-memory / 3 samples | 64 | 32 | 368,848 |
| Kernel + retained alignment-stress input | 65,536 | 65,248 | 783,136 |
| Catalogue + retained alignment-stress input | 65,536 | 65,248 | 499,920 |
| Minimal alignment fixture | 4 | 4 | 65,616 |
| Minimal alignment fixture | 64 | 32 | 65,616 |
| Minimal alignment fixture | 4,096 | 3,808 | 65,616 |
| Minimal alignment fixture | 65,536 | 65,248 | 131,152 |

The gate verifies every loadable ELF byte in RAM/IROM/DROM against its image,
allowing only the exact regenerated descriptor ELF hash. Descriptor placement,
page offsets, checksums/digest, zero padding and section/segment coverage pass.
The real kernel image also exercises a RAM section split across image segments.
These are checks against each build's own ELF, not a binary-equivalence claim
against the historical build or a runtime mapping readback.

A ninth, negative source control removes only the remedy byte in a disposable
script: the 64-byte fixture regains a 32-byte NOBITS gap, produces two DROM segments,
and is rejected. The saved historical ELF/image is also rejected for the missing
file-backed merge. Twelve new synthetic gate tests plus the prior eleven capture
tests pass. Workspace check/tests, firmware host tests (6/10), default/feature
strict host Clippy and formatting pass. All four full target links still emit
the separate RWX warning, unsuppressed.

The [saved report](s3-image-gate-report.json) records tool versions, relevant source
hashes, image/ELF hashes, inventories and the negative-control result. Local ELFs,
images and commands/build logs are under ignored
`firmware/target/image-gate-final-20261010`; binaries are not archived. Fresh runs
may differ in paths/metadata/hashes and should use the reproducible source checks,
not expect byte-identical historical output. No hardware was accessed, port opened,
reset or flash performed. Stress/minimal/negative fixtures are not flash candidates.

Next: prepare and gate an ordinary corrected measurement build, obtain explicit
flash approval, then capture its boot and bounded workload before calling the
multiple-DROM diagnostic fixed on target. Do not broaden the evidence suite yet.

## Pinned source references

- [esp-hal 1.2.2 rodata linker script](https://github.com/esp-rs/esp-hal/blob/915465df59aee91b627fe538f0b442e56c7f4fc9/esp-hal/ld/sections/rodata.x).
- [espflash 4.6.0 section extraction](https://github.com/esp-rs/espflash/blob/c1fb39939c1b5880b2aec7f43063f19689d5b00a/espflash/src/image_format/mod.rs),
  [segment merging](https://github.com/esp-rs/espflash/blob/c1fb39939c1b5880b2aec7f43063f19689d5b00a/espflash/src/image_format/idf.rs).
  These match the installed crate sources inspected locally.
- [Bootloader selection/mapping at the logged ESP-IDF revision](https://github.com/espressif/esp-idf/blob/14f663f003e/components/bootloader_support/src/bootloader_utility.c).
- [Upstream issue 927](https://github.com/esp-rs/espflash/issues/927) discusses similar
  splitting, but is not the diagnosis or a verified remedy for this build.
