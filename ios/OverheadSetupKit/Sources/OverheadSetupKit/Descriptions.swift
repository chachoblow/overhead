import Foundation

extension ConfigureResult {
    /// What to tell the user.
    public var message: String {
        switch self {
        case .saved: "Saved. The Overhead is restarting on the new settings and will start tracking in about a minute."
        case .invalidSsid: "The network name must be 1–32 bytes."
        case .invalidPassphrase: "The password must be 8–63 characters, or 64 hex digits."
        case .invalidSite: "The location is out of range."
        case .networkNotFound: "The Overhead couldn't see that network. Check the name, or move it closer to the router."
        case .wrongPassphrase: "The network refused the password."
        case .noAddress: "The Overhead joined the network but got no address from it (DHCP)."
        case .joinFailed: "The Overhead couldn't join the network."
        case .saveFailed: "The network worked, but the settings couldn't be saved. Try again."
        }
    }
}

extension SetupReason {
    public var label: String {
        switch self {
        case .unconfigured: "Not set up"
        case .joinTimeout: "Couldn't join its network"
        case .button: "Knob held at power-on"
        case .factoryReset: "Factory reset"
        }
    }
}

extension Channel {
    public var label: String {
        switch self {
        case .stable: "Stable"
        case .beta: "Beta"
        }
    }
}

extension Security {
    public var supported: Bool { self == .personal }
}
