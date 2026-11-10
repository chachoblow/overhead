// swift-tools-version: 6.0
// The 0087 setup protocol and BLE session, shared by the app; the protocol
// mirrors the Rust `setup/` crate and is tested against its byte vectors.
import PackageDescription

let package = Package(
    name: "OverheadSetupKit",
    platforms: [.iOS(.v17), .macOS(.v14)],
    products: [
        .library(name: "OverheadSetupKit", targets: ["OverheadSetupKit"]),
    ],
    targets: [
        .target(name: "OverheadSetupKit"),
        .testTarget(name: "OverheadSetupKitTests", dependencies: ["OverheadSetupKit"]),
    ],
    swiftLanguageModes: [.v5]
)
