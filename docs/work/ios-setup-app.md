# The board is set up only through an iOS app on TestFlight
Status: verifying: waiting for the board to install release 14 from `beta` after its next 8 h refresh
Started: 2026-11-09. Scope: PLAN M7 "iOS setup app"; decisions
[0087](../decisions/0087-ios-setup-app-over-ble.md) (protocol note),
[0084](../decisions/0084-settings-store.md), [0085](../decisions/0085-signed-updates-and-release-keys.md)

## Outcome
A SwiftUI + CoreBluetooth app in `ios/`, on TestFlight for the user, finds
`OVERHEAD-xxxx`, pairs with the Sharp's passkey, lists the device's scanned
networks, takes the passphrase, sends the phone's GPS location (manual
`lat, lon` fallback), and shows read-only status. The user's board is set up
only through the app, then updates over the air with those settings intact.

## Done when
- Swift protocol tests pass against `setup/`'s fixed byte vectors (`swift test`).
- The app builds for device and is on TestFlight; the user installs it from there.
- User observation: board in setup (IO6 3 s at boot) → app pairs, scans, saves;
  board tracks at the phone's location; a wrong passphrase is reported.
- The board then installs a newer signed `beta` release with settings intact.

## Boundaries
- Fixed GATT table and protocol v1 as in the 0087 note; no firmware change
  unless a real defect is found (then a separate, recorded fix).
- No factory reset of the user's board (it would return the channel to `stable`).
- No App Store submission (gift readiness item); TestFlight only.
- No new Mac tools (no XcodeGen/Tuist); the project file is checked in.

## Human gates
- Bundle ID / display name: asked 2026-11-09 / answered: `com.wesklein.overhead`, "Overhead"
- App Store Connect: asked 2026-11-09 / answered: user created the record, named "Overhead Setup" (home-screen name stays "Overhead"); agent uploads with xcodebuild (user's Xcode account)
- Board: answered 2026-11-09: stays on devbox, user nearby with the phone; agent reads UART over SSH. User presses/observes later
- Release 14 for the OTA half: built on devbox, user signed on the Mac 2026-11-09, published to `pages` (`feb8aac`)

## Plan
- [x] `ios/OverheadSetupKit` Swift package: frame decode/encode mirroring
      `setup/`, Configure.check rules; tests with the fixed vectors.
- [x] CoreBluetooth session (`SetupCentral`): scan by name prefix/service,
      connect with 3 retries, discover, subscribe (completes after the passkey
      pairing the device requests on connect), MTU check, one request at a time, timeouts.
- [x] `Testing` link loss: wait for the device to advertise again (60 s), reconnect
      (pairs again: no bonds), `GetInfo`, read `last_result`; not seen = left setup after saving.
- [x] SwiftUI flow: device list → pairing → networks/passphrase/location/status → result sheet.
- [x] Xcode project (`ios/Overhead.xcodeproj`, synchronized folder, local package),
      generated Info.plist with usage strings, `ITSAppUsesNonExemptEncryption = NO`,
      1-bit radar icon (`AppIcon.png`, from a CoreGraphics script); simulator build.
- [x] Device archive and TestFlight upload: 0.1.0 (1), internal testing only.
- [x] User run on the board (2026-11-09): IO6 held 3 s, app paired with the Sharp's code,
      wrong passphrase reported, correct one saved, board restarted and tracked.
- [ ] OTA with settings intact: release 14 (`0.1.14-beta`, same firmware as 13) on `beta`;
      the board checks only after a successful refresh (every 8 h, stored elements defer it),
      then installs once the knob/BOOT are idle 2 min. Confirm `OTA_INSTALL done`, build 14
      `VALID`, Wi-Fi joined from saved settings in devbox `firmware/target/setup/ios-ota14.log`
      (12 h capture started 2026-11-09 ~22:00 PT; board must stay plugged in).
- [ ] Promote: README for `ios/`, 0087 note, PLAN/HANDOFF/PROGRESS.

## Discoveries and assumptions
- A UART capture can't span the IO6 power cycle (USB power = the serial port); start it after replugging.
- Mac: Xcode 27.2, iOS 27.0 simulator; Apple Development identity, team
  `YNFZSX95R2` (individual). Board is on devbox `/dev/ttyUSB0`, not this Mac.
- Default min iOS 17 (reversible).

## Verification
- `swift test` in `ios/OverheadSetupKit` → 7 tests passed (2026-11-09).
- `xcodebuild … -destination 'platform=iOS Simulator,name=iPhone 17' build` → succeeded, no warnings; launched, list screen renders.
- `xcodebuild … archive -allowProvisioningUpdates` → succeeded after the user accepted the PLA; `-exportArchive` (method app-store-connect, destination upload) → "Upload succeeded", 0.1.0 (1).
- User run: observed by the user as described (pairing, wrong-passphrase message, Saved, tracking).
  No UART log: unplugging the board for the IO6 hold removed `/dev/ttyUSB0` and ended the capture.
- Release 14: built on devbox `OTA_CHANNEL=beta OVERHEAD_BUILD=14 OVERHEAD_VERSION=0.1.14-beta`, `--features device`;
  `image_gate.py` exit 0; SSID/password/site absent from the `.bin`; sha256 `ab4ed411…e57956` matched on the Mac;
  `overhead-release sign` + `verify` ok; raw GitHub serves the build-14 manifest.

## Open questions
- Show the app's Testing-loss recovery in practice (not hit on the user's run).
