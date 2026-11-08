# 0083 — Flash layout for updatable devices

Date: 2026-11-08
Status: accepted (user, 2026-11-08: giftable-device proposal accepted as written).
From [the giftable-device proposal](../giftable-device-proposal.md) (A).
Replaces the factory-app layout of [0079](0079-fetched-element-persistence.md)'s
`firmware/partitions.csv`; 0079's element format is unchanged.

## Decision
Every gift unit is an ESP32-S3 N8R8 (8 MB flash, 8 MB PSRAM) with this one
partition table, fixed from the first gift:

| Name | Type/subtype | Offset | Size | Purpose |
|---|---|---|---|---|
| nvs | data/nvs | 0x9000 | 0x4000 | kept for the radio stack; unused by our code |
| otadata | data/ota | 0xd000 | 0x2000 | which slot boots, and its state |
| phy_init | data/phy | 0xf000 | 0x1000 | unchanged |
| ota_0 | app/ota_0 | 0x10000 | 3 MiB | app slot A |
| ota_1 | app/ota_1 | 0x310000 | 3 MiB | app slot B |
| settings | data/undefined | 0x610000 | 64 KiB | [0084](0084-settings-store.md)'s two slots |
| elements | data/undefined | 0x620000 | 320 KiB | 0079's two element slots, moved |
| spare | data/undefined | 0x670000 | 1,600 KiB | reserved; never written by today's firmware |

No factory slot. USB flashing writes `ota_0` and erases `otadata`.

Internal RAM: runtime images also add the bootloader-reclaimed `dram2` region
(0x3FCDB700..0x3FCED710, 72 KiB) as a second internal heap region; the BLE
spike showed it unused and needed ([0087](0087-ios-setup-app-over-ble.md)).

## Why
An update can replace the app but not the partition table, so the table must
be right before the first gift. Each slot is ~2.2× today's ~1.39 MB image
(1.64 MB with BLE); `spare` leaves room for data not yet imagined (for
example a city list).

## Consequences
- The first flash of this layout clears the stored elements once.
- Two slots halve the room any single image can take; an image over 3 MiB
  needs a new layout, which only USB can install.
- The custom bootloader ([0086](0086-bootloader-rollback-and-health-check.md))
  must agree with `otadata` and the `dram2` reclaim.
