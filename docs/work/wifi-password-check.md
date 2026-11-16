# The setup app checks a Wi-Fi password before saving
Status: active
Started: 2026-11-16. Scope: PLAN M7 (iOS app UI/UX refinement, user request);
decisions [0087](../decisions/0087-ios-setup-app-over-ble.md) (protocol: fields
and types only appended, client offers what the device's version has).

## Outcome
In the iOS setup app, tapping a network opens a password sheet. Join asks the
Overhead to try the network without saving. A wrong password keeps the sheet
open with the reason; a working one closes it and checks the network. Save
still tests again and saves. A device without the new request (protocol
version 1, release 15 and earlier) still works: Join just closes the sheet
and the check at Save is the only test.

## Done when
- `setup/` protocol version 2: `TestWifi` request, `TestResult` response,
  `ConfigureResult::Joined`, `Info.last_test` (appended, optional on read);
  fixed vectors updated; `cargo test --workspace`.
- Firmware answers `TestWifi` (join, DHCP, leave; nothing stored; the Sharp
  shows the result); firmware host tests; a device image builds and gates.
- `overhead-setup-client` has `test-wifi` (compiles; Linux-only to run).
- `OverheadSetupKit` mirrors it (vectors in `swift test`); the app has the
  password sheet with the version-1 fallback; simulator build, no warnings.
- On the board: a wrong password is refused in the sheet, the right one
  checks the network, Save saves and the board tracks (user, with UART log).

## Boundaries
iOS UI changes limited to the Wi-Fi selection flow. No GATT table change
(0087: fixed). No release/signing in this item (a release carrying it can
follow with 16). App Store submission out of scope.

## Human gates
- Approach: asked 2026-11-16 / answered: add a test-only request.
- On-board check: user installs the app build on the iPhone, holds the knob
  at boot for setup, tries a wrong then the right password, saves.

## Plan
- [x] `setup/`: v2 types, vectors, tests.
- [x] Firmware: `TestWifi` handling, `Show::Joined`, render screen; host tests.
- [x] Setup client `test-wifi` (written; compile on devbox, Linux-only).
- [x] Swift kit: mirror + tests; `SetupCentral.testWifi`, device version.
- [x] SwiftUI: password sheet, link-loss recovery via `Info.last_test`.
- [x] Checks: cargo check/test, firmware host tests, swift test, xcodebuild.
- [x] Build + gate + flash a dev image (`device`, `OVERHEAD_BUILD=15`, beta,
      no `dev-settings`): `firmware/target/wifitest.elf`.
- [ ] App on the iPhone; user on-board check with a UART capture.
- [ ] 0087 note, README evidence, delete this file, handoff.

## Discoveries and assumptions
- Board is on this Mac (`/dev/cu.usbserial-21410`); build and flash here.
- Save re-tests (simple contract); skipping the second test is an open
  question.

## Verification
- `cargo test --workspace` 363 passed; `cargo check --workspace` ok; firmware
  host tests 38 passed; `swift test` 9 passed; simulator build ok, no new
  warnings.
- `wifitest.elf` sha256 `b00268cc…f024`, `image_gate.py` pass; flashed
  2026-11-16 22:52 UTC on `/dev/cu.usbserial-21410` (MAC `80:45:6b:2a:45:08`):
  `OTA boot build=15 version=0.1.15-wifitest … Valid channel=beta`,
  `STORE loaded`, Wi-Fi 2 s (`firmware/target/wifitest-boot.log`).

## Open questions
- Skip Save's second test when the same network was just joined? Default: no.
