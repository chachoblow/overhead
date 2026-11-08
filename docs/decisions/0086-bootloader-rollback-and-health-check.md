# 0086 — Bootloader rollback and the health check

Date: 2026-11-08
Status: accepted (user, 2026-11-08: giftable-device proposal accepted as written;
ESP-IDF bootloader build in rootless Docker approved).
From [the giftable-device proposal](../giftable-device-proposal.md) (E).
Amends [0076](0076-device-app-build.md)'s halt-on-panic.

## Decision
- **Bootloader.** espflash 4.6.0's bundled S3 bootloader never rolls back
  (ESP-IDF `release/v6.1`, no sdkconfig overrides), so runtime images use a
  custom one: built once on devbox in rootless Docker from Espressif's
  `espressif/idf` image pinned to v6.1, with
  `CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE=y`. The binary, its sdkconfig, the IDF
  commit and its hash are committed; every runtime image is flashed with
  `--bootloader`. Fixed from the first gift.
- **Health check.** A new image boots `PendingVerify` and marks itself
  `Valid` only after, within 10 minutes: a radar or `NO FRAME` frame drawn,
  Wi-Fi joined, trusted UTC, a catalogue loaded, and the channel manifest
  fetched with its signature verified. Otherwise it marks itself `Invalid`
  and resets into the old slot, which records the failed `build`
  ([0084](0084-settings-store.md)) and does not reinstall it.
- **Reset paths.** The device app resets on panic instead of halting, and a
  hardware watchdog resets a hung loop.
- **Brick drill** before the first gift, on the user's board: a release that
  panics, one that hangs, one that cannot reach the manifest, one with a bad
  signature, a download cut off midway, and a power cut mid-install. All must
  leave the board on a working image.

## Why
Without rollback, an image that boots and then hangs or panics before marking
itself bad is booted forever, and a gifted device is out of reach. Including
the manifest check makes a build that breaks updating roll back. A Wi-Fi
outage at the wrong moment rolls back a good build, which is harmless: it
retries with the next build.

## Consequences
- 0076's "halt on panic" no longer holds for the device app once this lands;
  diagnostics may still halt.
- The toolchain gains a Docker-built bootloader artefact; rebuilding it is a
  toolchain change.

## Note — 2026-11-08: implemented; brick drill passed

Bootloader built by `firmware/bootloader/build.sh` (IDF v6.1 `fff9895c`,
sha256 `d86e62bf…`). The firmware writes `otadata` itself
(`firmware/src/otadata.rs`): `esp-bootloader-esp-idf` 0.6.0's `Ota` gives
sequence 0 for `ota_1` after an erased `otadata`. Rollback is detected from
settings `pending_build` (saved just before activation). RWDT (20 s) is fed
by the display loop only while the network loop beats (15 min). A new image
checks its manifest as soon as it has trusted UTC rather than waiting for the
8 h refresh. Drill images never confirm themselves. All six drills passed on
the user's board, plus a real defect (release 9, TLS 1.3) rolled back.
[Evidence](../../firmware/README.md#signed-ota-and-rollback-00830086).
