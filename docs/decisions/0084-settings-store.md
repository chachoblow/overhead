# 0084 — Settings store; built-in personal values are developer-only

Date: 2026-11-08
Status: accepted (user, 2026-11-08: giftable-device proposal accepted as written).
From [the giftable-device proposal](../giftable-device-proposal.md) (B).
Supersedes [0004](0004-hardcoded-location.md); amends the build-time credential
and site parts of [0002](0002-data-and-time-over-wifi.md) and
[0075](0075-fetched-element-refresh-and-staleness.md).

## Decision
- Per-home settings live in the `settings` partition
  ([0083](0083-flash-layout-for-updatable-devices.md)) as a versioned,
  CRC-checked record in two alternating slots, header written last (0079's
  pattern), so a power cut during a save leaves the previous record.
- Fields: format version; one Wi-Fi SSID and passphrase; observer latitude,
  longitude and optional altitude; update channel (`stable` by default); the
  last update build that failed its health check; BLE bonding keys if bonds
  are kept (decided with [0087](0087-ios-setup-app-over-ble.md)'s work item).
- Future firmware reads every earlier format version or migrates it.
- Build-time values become developer-only: a `dev-settings` feature seeds an
  empty store from `firmware/.wifi.env`/`.site.env`. Release images are built
  without it and contain no personal data, because they are published.
- The passphrase is stored unencrypted. Flash encryption and Secure Boot burn
  eFuses permanently and are not used.

## Why
A gifted device must change network and location without a reflash, and a
published image must not carry the user's Wi-Fi or home site. Plain storage
is accepted for home gifts: reading it needs the board in hand.

## Consequences
- The settings format is fixed from the first gift; contents grow through new
  format versions.
- The observer still enters core/render as an input (0004's consequence holds).

## Note — 2026-11-08: formats 1 and 2, Wi-Fi and site in M7 item 1

`firmware/src/settings_store.rs`. Format 1 (channel, failed and pending
build) shipped first; format 2 adds one Wi-Fi network and the site, and reads
format 1 (migrated on the board). `pending_build` is an internal field: the
build an install activated but has not confirmed (0086). Wi-Fi/site and
`dev-settings` moved from M7 item 2 into item 1 (user, 2026-11-08) because
the `beta` image is published. `failed_build` keeps only the last failure.

## Note — 2026-11-09: no bonds; setup writes format 2

No BLE bonds are stored: every setup session pairs afresh with a new
passkey, and the user's iPhone paired again after a factory reset without
"Forget This Device" ([0087 note](0087-ios-setup-app-over-ble.md#note--2026-11-09-setup-protocol-and-firmware)).
The bonding-keys field is therefore not added; format 2 stays current, and
setup replaces its Wi-Fi/site with a tested network. A factory reset writes
defaults on `stable`, keeping only `failed_build`.
