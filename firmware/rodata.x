/* Local override of esp-hal 1.2.2 ld/sections/rodata.x (MIT OR Apache-2.0).
 * https://github.com/esp-rs/esp-hal/blob/915465df59aee91b627fe538f0b442e56c7f4fc9/esp-hal/ld/sections/rodata.x
 * Only semantic change: BYTE(0) makes .rodata_merge file-backed PROGBITS.
 * Keep in sync explicitly when upgrading the pinned HAL; see IMAGE_GATE.md.
 */
SECTIONS {
  /* ESP-IDF application descriptor must remain first. */
  .flash.appdesc : ALIGN(4)
  {
    KEEP(*(.flash.appdesc));
    KEEP(*(.flash.appdesc.*));
  } > RODATA

  .rodata_merge : ALIGN(4)
  {
    /* A location-counter-only gap becomes NOBITS with our GCC linker and
     * espflash omits it. Emit real data even when no gap was needed: that
     * costs one alignment unit but keeps this section mergeable in all cases.
     */
    BYTE(0);
    . = ALIGN(ALIGNOF(.rodata));
  } > RODATA

  .rodata : ALIGN(4)
  {
    . = ALIGN(4);
    _rodata_start = ABSOLUTE(.);
    *(.rodata .rodata.*)
    *(.srodata .srodata.*)
    . = ALIGN(4);
    _rodata_end = ABSOLUTE(.);
  } > RODATA

  .rodata.wifi : ALIGN(4)
  {
    . = ALIGN(4);
    *(.rodata_wlog_*.*)
    . = ALIGN(4);
  } > RODATA
}
