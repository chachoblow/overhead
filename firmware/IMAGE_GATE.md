# Offline S3 image gate

Source-level DROM remedy and pre-flash check for the two measurement binaries,
not a boot test or device-memory/capacity measurement. Rationale:
[0018](../docs/decisions/0018-source-level-drom-remedy-and-image-gate.md).
[Diagnosis, offline results and corrected catalogue-memory boots](../docs/evaluations/s3-drom-diagnostic.md).
Two approved corrected target captures now pass; this gate remains an offline
check and does not authorize any future flash.

## Remedy

`rodata.x` is a local override of the pinned esp-hal 1.2.2 section script. GNU ld
searches its working directory for `INCLUDE "rodata.x"` before the `-L` paths;
**build target firmware from `firmware/`**, as for the existing configuration.
`build.rs` tracks the override so edits relink. A separate `-Llinker` flag was
insufficient: Cargo's dependency search directories took precedence in the tested
link command. The gate checks the actual ELF, not assumptions about search order.

The sole semantic change is `BYTE(0)` before the existing alignment expression.
This forces file-backed PROGBITS rather than a NOBITS gap that espflash skips.
If the descriptor end already meets rodata alignment, this deliberately consumes
one alignment unit (8 bytes in the default kernel build, 4 in the small fixture).
Otherwise it fills the existing gap (32 bytes in the catalogue build). Descriptor
placement, rodata input collection and symbols are retained. No registry patch,
new dependency, toolchain change, ELF mutation, or warning suppression is needed.
Review this override explicitly on any HAL/linker/espflash upgrade.

## Complete regression run — no hardware

Host/workspace checks are in [README](README.md#offline-checks); optional-feature
host checks are in [CATALOGUE_MEMORY](CATALOGUE_MEMORY.md#offline-verification--next-step).
These and the image runner pass from a clean source snapshot without ignored artifacts.

From repository root, with the installed toolchain:

```sh
. "$HOME/export-esp.sh"
python3 -m unittest discover -s firmware -p 'test_*.py'
python3 firmware/check_images.py firmware/target/image-gate-new-run
```

The output directory must not already exist. The runner uses a separate Cargo
target directory, builds `baseline`/three-sample binaries separately with their
proper features, and retains ELFs, application-only images, commands/build logs,
and a JSON report (`complete: true` only after every case passes):

- Normal kernel and catalogue-memory builds.
- Each binary with a retained assembly input forcing 65,536-byte rodata alignment.
- Four minimal source linker fixtures with 4/64/4,096/65,536-byte alignment, using
  the same local script, descriptor, and known RAM/IROM/rodata bytes.
- Negative control: remove only `BYTE(0)` in a disposable script, relink the
  64-byte fixture, require the 32-byte NOBITS gap and two DROM segments, and
  require the gate to reject it. No historical ignored ELF is needed.

**Stress, minimal-fixture, and negative-control artifacts are never flash
candidates.** They contain test inputs; minimal fixtures are not executable apps.
Do not flash the runner's Cargo output path, whose last build is stressed. Prepare
an ordinary, explicitly selected measurement build and gate that exact ELF/image.

## Gate one normal build before requesting flash approval

From `firmware/`, after an ordinary build using its documented suite/features:

```sh
ELF=target/xtensa-esp32s3-none-elf/release/overhead-s3-catalogue-memory
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --skip-update-check "$ELF" /tmp/catalogue-corrected.bin
python3 image_gate.py "$ELF" /tmp/catalogue-corrected.bin
```

Use `overhead-s3-benchmark` for the default binary. `save-image` does not flash or
open a port. Keep the report/hash with any future capture and use the same ELF
and image-generation options for the approved flash. A rebuild invalidates the
previous gate result; rerun it. Default application offset is `0x10000`, matching
the existing captured partition; `--app-offset` must remain 64-KiB aligned.

The standard-library Python gate requires ELF32 Xtensa/S3, an explicit 64-KiB
page descriptor first at `0x3c000020`, contiguous file-backed zero merge padding,
exactly one DROM and one IROM image segment, matching entry points and flash-page
offsets, no overlapping image ranges, valid checksum and appended SHA-256. Every
allocated PROGBITS/INIT_ARRAY section is compared byte-for-byte across segments,
including RAM sections split by espflash. The only allowed mutation is the exact
ELF SHA-256 inserted into the descriptor. Unclaimed payload bytes must be zero.
NOBITS is not file payload; unsupported allocated section types fail closed.

This is not a general ESP/secure-boot/merged-image validator, a provenance proof,
or a runtime MMU/boot/stack test. It checks image contents against the supplied
ELF, not binary equivalence with earlier builds. Build paths/metadata and legitimate
relocation can change hashes and sizes. The independent LOAD-segment RWX warning
remains visible. Any future target flash/capture still requires approval.
