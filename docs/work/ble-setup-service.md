# The device can be set up over BLE with passkey pairing
Status: active
Started: 2026-11-08. Scope: PLAN M7 item 2, firmware half (user, 2026-11-08:
split agreed; the iOS app is the next item); decisions
[0084](../decisions/0084-settings-store.md), [0087](../decisions/0087-ios-setup-app-over-ble.md),
[0083](../decisions/0083-flash-layout-for-updatable-devices.md)–[0086](../decisions/0086-bootloader-rollback-and-health-check.md).

## Outcome
A board with empty settings is set up end to end over BLE with LE Secure
Connections passkey pairing (code on the Sharp), driven only through the
versioned setup protocol, then tracks; a signed published `beta` release then
installs over the air with those settings intact. The protocol is specified
and tested so the iOS app (next item) can be written against it.

## Done when
- `overhead-setup` (no_std, no alloc) owns the framed protocol (version, type,
  payload); host tests cover every message, malformed input, fixed byte
  vectors (for the Swift tests) and a newer client against this device
  (unknown types and appended fields).
- Firmware: setup window on no stored Wi-Fi, on no join for 5 min after boot,
  or IO6 held 3 s at boot; passkey on the Sharp; network scan list; the new
  network is tested before saving; result to the client and on the Sharp;
  save, then run at the new site. IO6 held 10 s at boot = factory reset
  (settings + elements, into setup). Short press = info page.
- Board, driven from devbox's test client: setup from empty settings;
  re-entry by the 3 s hold; factory reset; wrong passphrase rejected without
  saving; internal memory while paired/connected recorded.
- iPhone (nRF Connect): pair with the code, factory reset, pair again; record
  whether iOS needs "Forget This Device".
- A published release (no `dev-settings`/`ota-dev`/`ota-drill`, SSID absent
  from the `.bin`), signed on the user's Mac, installs over the air from
  `beta` with settings intact.
- `cargo check --workspace`, `cargo test --workspace`, firmware host tests,
  fmt; firmware release builds gated.

## Boundaries
No Swift/iOS/App Store work. No OTA, bootloader or toolchain changes. BLE is
off outside the setup window (user, 2026-11-08). No bonds stored unless the
iPhone check forces it (then one bond in settings format 3).

## Human gates
- Split / bonds / BLE outside window / dev passkey on UART / test-client
  dependency: asked and answered 2026-11-08: agreed as recommended.
- BlueZ on devbox (`bluez` package + `bluetooth.service`; machine change,
  needs sudo): asked 2026-11-08, answered: user installed it; logged in
  ~/code/devbox/README.md, snapshot run. `bluetoothctl show` works as the
  user without group changes.
- Physical: hold IO6 3 s / 10 s at boot, short press; look at the Sharp
  screens (passkey, result, info page): later, asked once.
- nRF Connect pair / reset / re-pair on the user's iPhone: later.
- Sign the release on the Mac and push `pages`: at the end.

## Plan
- [x] `setup/` crate: protocol types, encode/decode, vectors, tests.
- [x] render: setup screens (waiting with name, passkey, testing, result,
      hold feedback, info page) — `render/src/setup.rs`.
- [x] firmware: IO6 switch input (boot hold classification, short press).
- [x] firmware: setup mode (`setup_mode.rs`, rules in `setup_policy.rs`):
      builds; not yet run on the board.
- [x] firmware: info page in normal run (display task; builds).
- [x] BLE spike removed (feature `ble-spike`, `ble_spike.rs`); BLE now in
      every `live-fetch-radar`/`device` image, static heap 128 KiB + dram2.
- [ ] tools: `overhead-setup-client` (bluer) driving the protocol from
      devbox, passkey from the UART capture in dev builds.
- [ ] board runs: empty settings → setup → tracking; 3 s re-entry; factory
      reset; wrong passphrase; memory.
- [ ] user gates: physical presses/screens, nRF Connect.
- [ ] published release via Mac signing; OTA with settings intact.
- [ ] promote: decision note(s) on 0084/0087 (protocol contract), firmware
      README usage/evidence, tools README, PLAN, HANDOFF, PROGRESS.

## Discoveries and assumptions
- trouble-host 0.7.0 `security` feature: LE Secure Connections, `PassKeyDisplay`
  events, IoCapabilities::DisplayOnly, characteristics with
  `authenticated` permission, not bondable by default; RNG seeded from the
  controller's LeRand. Adds p256/aes/cmac/rand_chacha (its own optional deps).
- devbox: Intel AX200 (`hci0`, btusb); `bluez` not installed, bluetoothd not
  running; libdbus-1-dev present (bluer builds).
- Default: setup is a separate boot mode (radar replaced by setup screens;
  no fetch/TLS while BLE runs), so BLE never runs during a refresh. Leaving
  setup = save then software reset into the normal run (which prepares the
  coastline for the new site).
- Default: with stored settings, setup mode with no client for 10 min resets
  back into the normal run (5 min join / 10 min setup cycle), so a slow
  router after a power cut doesn't strand the device in setup.
- Default: no setup window while the image is on probation (0086); the
  health check decides first.
- trouble-host 0.7.0 needs `central` with `security` (its GAP attribute
  count is only defined for that pair); enabled, unused.
- `dev-settings` images re-seed an empty store at every boot, so setup from
  empty settings is tested on an image built without `dev-settings`
  (`setup-test` prints the passkey on UART for the devbox client).
- The join-timeout flag lives in RTC fast RAM (`persistent`) with a magic
  word; whether the 0086 bootloader keeps it across `software_reset` is
  checked on the board.
- Info page: a press can be missed during a TLS stall (~450 ms loop gap).

## Verification

## Open questions
