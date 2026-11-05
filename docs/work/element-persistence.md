# Fetched elements survive a reset (flash persistence)
Status: active
Started: 2026-11-05. Scope: PLAN M6 "Wi-Fi credentials, HTTPS elements, SNTP,
persistence, and refreshes"; decisions [0002](../decisions/0002-data-and-time-over-wifi.md),
[0075](../decisions/0075-fetched-element-refresh-and-staleness.md) (item 5 "no persistence yet"),
[0076](../decisions/0076-device-app-build.md); new decision 0079 (accepted by the user 2026-11-05).

## Outcome
After a reset the shared runtime (`device` app and `live-fetch-radar` diagnostic)
loads the last successfully fetched catalogue from a flash partition. Once SNTP
gives trusted UTC: stored copy younger than the refresh interval → track it with
no request, next refresh at stored fetch time + interval; older (or from the
future) → fetch as now; pre-request failure → keep tracking the stored copy under
0075's `OLD DATA` (24 h from the stored fetch time) / expiry (7 days) rules.
Before SNTP the radar still shows `NO FRAME`.

## Done when
- Host tests: slot format (header CRC, body CRC, version, length bound), newest
  valid slot chosen, corrupt/partly written/wrong-version slot falls back,
  every interrupted write step leaves the previous copy loadable, resume rule.
- Board (`live-fetch-radar`): reset loads from flash with no GET then tracks;
  stored copy older than the interval triggers a fetch; erased and corrupted
  slots fall back to a fetch; frame cadence (`FRAME_WINDOW`, store gap) during a
  save within the soak's limits; heap flat across saves.
- Device app flashed; user sees the radar come back after a reset without a fetch.

## Boundaries
No RTC, location source, Wi-Fi provisioning or group change; 0075 thresholds and
backoff unchanged. Selection is not persisted. `live-fetch-faults` keeps its
plan semantics (persistence off in that build). Frozen M2 evidence untouched.
Each reset that fetches costs one CelesTrak request; keep board resets few.

## Human gates
- esp-storage dependency, raw-body format, two-slot partition, per-sector save:
  asked 2026-11-05 / answered: accepted as recommended (also accepts 0079).
- User visual check of the radar after a reset: needed at the end.

## Plan
- [x] `firmware/src/element_store.rs`: header codec, CRC-32, layout from
      partition, slot choice, step-wise writer, NOR-semantics mock; host tests.
- [x] `element_refresh::Schedule::resume_stored`; host tests.
- [x] `frame_stats::Cause::Store`.
- [x] `firmware/partitions.csv` (factory 0x10000+0x7a0000; `elements` data/undefined
      0x7b0000+0x50000, two 160 KiB slots).
- [x] Runtime: esp-storage dep (already locked via esp-bootloader-esp-idf),
      find partition, boot load (internal bounce buffer, never PSRAM ↔ ROM),
      resume after first in-budget SNTP, save after Success one sector per step
      with yields, STORE/FETCH_RESUME event lines, diag store gap.
- [ ] Builds + image gate; flash diagnostic; board runs (load, old, erased, corrupt, save cadence).
- [ ] Device app flash; user check.
- [ ] Decision 0079, README evidence, PLAN/HANDOFF/PROGRESS; delete this file.

## Discoveries and assumptions
- esp-storage 0.10.0 is already in firmware/Cargo.lock (dependency of
  esp-bootloader-esp-idf 0.6.0, which also provides `partitions::read_partition_table`).
- espflash 4.6.0 rejects numeric custom data subtypes (panic); `data, undefined` works.
- esp-storage runs ROM flash calls in a critical section (interrupts off) for
  the whole sector erase (~45 ms typical): BOOT sampling and the display loop
  stall for that long per step. Measure; do not batch erases.
- Flash ↔ PSRAM transfers go through a static internal-RAM 4 KiB buffer
  (ROM flash routines and the cache that serves PSRAM share the MSPI bus).

## Verification
- 2026-11-05 checkpoint: `cargo check --workspace` ok; firmware host default 30,
  `element-fetch` 80, `live-utc-boot` 82, `sharp-boot-capture` 72; dummy release
  builds of `live-fetch-radar`, `device`, `live-fetch-faults` (RWX warning only);
  clippy on `live-fetch-radar`: only the pre-existing `deref_addrof` warnings.

## Open questions
