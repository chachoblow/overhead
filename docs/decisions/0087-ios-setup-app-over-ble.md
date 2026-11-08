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
