# Overhead iOS setup app

SwiftUI + CoreBluetooth app for [0087](../docs/decisions/0087-ios-setup-app-over-ble.md)'s
BLE setup service. It finds `OVERHEAD-xxxx`, pairs with the code on the
Sharp, lists the device's scanned networks, takes the passphrase, sends the
phone's location (or typed `lat, lon`), and shows read-only status. Needs a
Mac with Xcode; devbox can't build it.

- `OverheadSetupKit/`: Swift package. `SetupProtocol.swift` mirrors the Rust
  `setup/` crate (which owns the contract), and its tests use that crate's
  fixed byte vectors; `SetupCentral.swift` is the BLE session.
- `Overhead/`: the app (a folder synced into `Overhead.xcodeproj`; new files
  need no project edit). The icon is `AppIcon.png`, 1-bit, 1024 px.
- Bundle ID `com.wesklein.overhead`, team `YNFZSX95R2`, iOS 17+, iPhone only,
  portrait. App Store Connect name "Overhead Setup"; home-screen name
  "Overhead". `ITSAppUsesNonExemptEncryption = NO`.

```sh
cd ios/OverheadSetupKit && swift test            # protocol vectors
cd ios && xcodebuild -project Overhead.xcodeproj -scheme Overhead \
  -destination 'platform=iOS Simulator,name=iPhone 17' build   # no Bluetooth in the simulator
```

**TestFlight.** Raise `CURRENT_PROJECT_VERSION` (both configurations in
`project.pbxproj`; `MARKETING_VERSION` for a new version), then:

```sh
xcodebuild -project Overhead.xcodeproj -scheme Overhead -destination 'generic/platform=iOS' \
  -archivePath /tmp/Overhead.xcarchive -allowProvisioningUpdates archive
xcodebuild -exportArchive -archivePath /tmp/Overhead.xcarchive \
  -exportOptionsPlist ExportOptions.plist -exportPath /tmp/export -allowProvisioningUpdates
```

`ExportOptions.plist` (not checked in): `method` `app-store-connect`,
`destination` `upload`, `teamID`, `signingStyle` `automatic`,
`testFlightInternalTestingOnly` true. Signing uses the Xcode account; if Apple
updates its Program License Agreement, signing fails until the user accepts it.

**Behaviour** (contract: the 0087 notes):
- The device asks for security on connect, so iOS prompts for the code at
  once; subscribing to responses completes after pairing (2 min timeout).
  Connects retry 3 × 12 s.
- No bonds: every connection pairs again with a new code.
- If the link drops after `Testing`, the app waits up to 60 s for the device
  to advertise again, reconnects and reads `Info.last_result`. A device that
  never comes back has left setup, which it does only after saving.
