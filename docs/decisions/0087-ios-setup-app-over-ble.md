# 0087 — iOS setup app over BLE

Date: 2026-11-08
Status: accepted (user, 2026-11-08: revised proposal F accepted; recipients all
use iPhones and the app will be published).
From [the giftable-device proposal](../giftable-device-proposal.md) (F, revised
after the BLE spike).

## Decision
- **Setup window.** The device offers its BLE setup service when it has no
  stored Wi-Fi, cannot join for 5 minutes after boot, or the knob switch (IO6)
  is held for 3 s at boot. Outside it, settings are not changed over BLE.
- **Pairing.** LE Secure Connections with passkey entry: the Sharp shows a
  6-digit code typed on the phone.
- **App.** SwiftUI + CoreBluetooth in `ios/` in this repo. It finds
  `OVERHEAD-xxxx`, pairs, lists networks from the device's scan, takes the
  passphrase, reads location from the phone's GPS (manual `lat, lon`
  fallback) and shows read-only status (build, channel, MAC, data age, last
  fetch, update result). It retries connections.
- **Saving.** The device tests the new network first, reports the result to
  the app and on the Sharp, then saves ([0084](0084-settings-store.md)),
  leaves setup, prepares the coastline for the location (0057/0058) and runs.
- **Protocol.** One versioned GATT service carrying framed messages (version,
  type, payload); older devices keep working with newer apps.
- **Distribution.** App Store, unlisted if Apple approves, otherwise listed;
  TestFlight for the user only.
- **Recovery.** Factory reset by holding the knob switch 10 s at boot (clears
  settings and elements, enters setup); a short knob press shows an info
  page; USB reflash in person as last resort.
- **Fallback.** If pairing or the protocol does not fit, a served setup page
  (access point, DHCP, captive DNS, one HTTP form) replaces the app; nothing
  else changes.

## Why
GPS removes the setup step a recipient is most likely to get wrong, and BLE
avoids captive-portal failures (iOS's mini-browser closing, phones leaving a
network without internet, the access point changing channel when the station
joins). The 2026-11-08 [spike](../../firmware/README.md#ble-coexistence-spike)
showed BLE and Wi-Fi fit together: ~33 KB statics and 35 KB heap, internal free
≥116 KB through a TLS fetch with the `dram2` region added, an iPhone wrote and
read, frames and knob unaffected. Rejected: a served page as primary (above),
Android (no recipients use it), firmware delivery through the app (updates
must not depend on anyone opening it, [0085](0085-signed-updates-and-release-keys.md)).

## Consequences
- The knob switch gets its first roles (setup, factory reset, info page).
- A second codebase (Swift) and an App Store listing to maintain; the Apple
  Developer account must stay paid.
- Unmeasured until the work item: pairing cost, long connections, BLE during a
  refresh with a phone connected; two of three first connection attempts
  timed out in the spike.

## Note — 2026-11-09: setup protocol and firmware

Firmware half done (M7 item 2, split from the app with the user,
2026-11-08); usage and board evidence in the
[firmware README](../../firmware/README.md#ble-setup-service-0087).
- **Protocol contract** (`setup/`, `overhead-setup`, no_std, no alloc; types
  and fixed byte vectors in its tests are the Swift app's reference).
  Service `6f760010-8a3e-4c1d-9b7a-0b5e1c2d3a40`; the client writes request
  frames to `…0011` (write with response) and reads responses as
  notifications on `…0012`; both need an authenticated (passkey) link. A
  frame is `version, type, payload` in one ATT value (≤180 bytes; MTU 246 on
  BlueZ and iOS). Version 1: requests `GetInfo`, `Scan`, `Configure`;
  responses `Info`, `Network`×n + `ScanDone`, `Testing` then
  `ConfigureResult`, `Error`. Fields are only appended; readers ignore
  trailing bytes and answer unknown types with `Error`. Requests are answered
  one at a time, in order. `Info.last_result` carries the window's last
  `Configure` result, so a client that loses the link while the device
  tests a network can reconnect and read it.
- **GATT table is fixed.** iOS caches an unbonded device's attribute table
  (it showed the old spike's service until Bluetooth was toggled), so the
  service's attributes never change for a device's life; new behaviour goes
  into frames or a new service UUID with Service Changed added first.
- **No bonds** (see [0084 note](0084-settings-store.md#note--2026-11-09-no-bonds-setup-writes-format-2)):
  `DisplayOnly`, LE Secure Connections, not bondable. Rejected: storing one
  bond (iOS re-pairs after a factory reset without "Forget").
- **Setup is a separate boot mode.** The window replaces the radar (no
  fetch/TLS while BLE runs) and is left by a software reset after a save, or
  after 10 min with no client when settings exist (so a slow router after a
  power cut doesn't strand the device). While an update is on
  probation (0086) it opens only with no Wi-Fi stored. The 5 min join timeout survives the reset as an RTC flag.
- **Scanning.** The device scans while no client is connected (every 60 s)
  and answers `Scan` from that cache when it is < 120 s old: a scan under a
  live link took the shared radio long enough to drop BlueZ's link. The test
  join uses the scanned channel. The device asks for a 4 s supervision
  timeout; iOS applies it, BlueZ doesn't.
- **Costs.** BLE controller ~35 KB internal; 92.5 KB internal free while
  paired. Pairing's P-256 work needs ~22.6 KB of CPU0 stack, so the static
  heap shrank 128 → 96 KiB (stack 46.9 KB). Pairing ~4.4 s.
- Dev images only (`setup-test`) print the passkey on UART; release images
  never do. The channel can't be changed over BLE.

## Note — 2026-11-10: the iOS app

M7 item 3 done; usage in [ios/README](../../ios/README.md), evidence in the
[firmware README](../../firmware/README.md#ble-setup-service-0087).
- **Shape.** `ios/OverheadSetupKit` (protocol mirror tested against
  `setup/`'s vectors, plus the CoreBluetooth session) and a SwiftUI app; the
  Xcode project is checked in (a synced folder, no XcodeGen/Tuist). Bundle
  ID `com.wesklein.overhead`, iOS 17+, iPhone only; App Store Connect name
  "Overhead Setup" ("Overhead" was taken), home-screen name "Overhead".
- **Link loss during `Testing`.** The app waits up to 60 s for the device to
  advertise, reconnects (pairing again, as there are no bonds) and reads
  `Info.last_result`; a device that doesn't return has left setup, which it
  does only after `Saved`. Not yet seen on hardware.
- **Location.** The phone's `ellipsoidalAltitude` (WGS-84, as `Configure`
  wants; 0 without a vertical fix); typed coordinates send altitude 0.
