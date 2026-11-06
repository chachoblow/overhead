# 0079 — Fetched elements persist in a two-slot flash partition

Date: 2026-11-05
Status: accepted (user, 2026-11-05: esp-storage, raw response body, two
alternating slots in a custom partition, sector-at-a-time saves).
Carries out [0002](0002-data-and-time-over-wifi.md)'s "elements and fetch
timestamp persist in flash" and replaces
[0075](0075-fetched-element-refresh-and-staleness.md) item 5 ("no persistence
yet") for the shared runtime of [0076](0076-device-app-build.md).

## Decision
1. **What is stored.** After each successful fetch, the HTTP body exactly as
   received (≤128 KiB GP JSON, already parsed and ingested) and its trusted
   fetch UTC. A stored copy is loaded through the same parser and ingest as a
   download, so it gets the same checks. Selection, view and time are not stored.
2. **Where.** An `elements` data partition (`firmware/partitions.csv`, 320 KiB
   at 0x7b0000; the factory app shrinks to 7,808 KiB) split into two 160 KiB
   slots. Each slot holds a 32-byte header in its first sector (magic, format
   version, sequence, fetch UTC, body length, body CRC-32, header CRC-32) and
   the body from the second. The highest-sequence slot whose header and body
   both check is loaded. Saves go to the other slot, so the previous copy
   survives until the new one is complete. A save erases the header sector
   first and writes the header last, so an interrupted save leaves only an
   invalid slot. Format code: `firmware/src/element_store.rs` (host-tested,
   including every interrupted-save point).
3. **Boot.** The stored copy is loaded before Wi-Fi is up. The radar still
   shows `NO FRAME` until SNTP, because without a battery-backed clock nothing
   dates a frame. At the first observation within the UTC budget, a copy
   younger than the refresh interval moves the next fetch to the end of its
   interval (`Schedule::resume_stored`), with no request at boot. A copy that
   old, or dated after the observed UTC, leaves the fetch due at once. 0075's
   `OLD DATA` (24 h) and expiry (7 days) apply to the stored fetch time and
   epochs as to a fetched catalogue, so a boot whose fetch fails keeps
   tracking the stored copy under the same rules.
4. **Saving without stopping the display.** esp-storage keeps interrupts off
   during each ROM flash call, so a save runs one sector erase or one sector
   program per step, with a 10 ms pause between steps. It starts 2 s after the
   catalogue is handed over, so the display applies the new catalogue first.
   Every flash transfer goes through a 4 KiB buffer in internal RAM; the PSRAM
   response buffer is never passed to the ROM routines.
5. **Degrades to the old behaviour.** Flashed without `partitions.csv` (no
   `elements` partition), the runtime logs `STORE unavailable` and fetches on
   every boot as before. The `live-fetch-faults` build never loads or saves,
   so its per-attempt fault plan keeps its meaning.
6. **Dependency.** `esp-storage =0.10.0`, already in the lockfile through
   `esp-bootloader-esp-idf =0.6.0`, whose partition-table reader it serves.

## Why
Each reset cost a CelesTrak request, and a reset during a CelesTrak or network
outage left the radar with nothing to show until a fetch succeeded. Storing
the raw body reuses the parser and its checks instead of adding a second
format to validate. It also spends flash only on what was already accepted. A
compact parsed format was rejected because it would need its own validation
for no need of space (156 objects ≈ 64 KB). One slot was rejected because
power loss during a save would lose the only copy. A blocking save of the
whole slot was rejected because it could hold interrupts off past the 500 ms
VCOM interval on slower flash.

## Consequences
- Cached elements alone still do not make an offline cold boot accurate. Time
  comes only from SNTP.
- Flashing a device image now needs `--partition-table partitions.csv`.
  `espflash flash` does not touch `elements`, so a reflash keeps the stored
  copy. `espflash erase-region 0x7b0000 0x50000` clears it.
- About three saves a day (one per refresh). Flash wear is negligible against
  NOR endurance.
- A format change bumps the header version, and older copies are then ignored
  (one fetch).
