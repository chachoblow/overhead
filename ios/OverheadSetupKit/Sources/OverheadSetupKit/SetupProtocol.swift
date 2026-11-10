import Foundation

/// The BLE setup protocol of
/// [0087](../../../../docs/decisions/0087-ios-setup-app-over-ble.md), version 1.
///
/// A Swift mirror of the Rust `setup/` crate (`overhead-setup`), which owns the
/// contract: a frame is `version u8, type u8, payload` in one ATT value;
/// integers and `f64` little-endian; strings a `u8` byte length then UTF-8.
/// Fields are only appended, so readers ignore trailing bytes. The tests check
/// this file against that crate's fixed byte vectors.
public enum SetupProtocol {
    /// This protocol version.
    public static let version: UInt8 = 1
    /// Largest frame either side sends; needs an ATT MTU of `maxFrame + 3`.
    public static let maxFrame = 180

    public static let serviceUUID = "6f760010-8a3e-4c1d-9b7a-0b5e1c2d3a40"
    /// Client → device: request frames (write with response).
    public static let requestUUID = "6f760011-8a3e-4c1d-9b7a-0b5e1c2d3a40"
    /// Device → client: response frames (notify).
    public static let responseUUID = "6f760012-8a3e-4c1d-9b7a-0b5e1c2d3a40"

    /// Advertised name prefix; the device appends the last two MAC bytes in hex.
    public static let namePrefix = "OVERHEAD-"

    public static let ssidMax = 32
    public static let passphraseMax = 64
    public static let versionMax = 32
}

enum MessageType {
    static let getInfo: UInt8 = 0x01
    static let scan: UInt8 = 0x02
    static let configure: UInt8 = 0x03
    static let info: UInt8 = 0x81
    static let network: UInt8 = 0x82
    static let scanDone: UInt8 = 0x83
    static let testing: UInt8 = 0x84
    static let configureResult: UInt8 = 0x85
    static let error: UInt8 = 0x86
}

/// Update channel (0085).
public enum Channel: UInt8, Sendable {
    case stable = 0
    case beta = 1
}

/// Why the device's setup window is open.
public enum SetupReason: UInt8, Sendable {
    /// No Wi-Fi or site stored.
    case unconfigured = 1
    /// The stored network did not join within 5 minutes of boot.
    case joinTimeout = 2
    /// The knob switch was held 3 s at boot.
    case button = 3
    /// Factory reset (switch held 10 s at boot).
    case factoryReset = 4
}

/// What a scanned network needs. Only `personal` can be configured.
public enum Security: UInt8, Sendable {
    /// WPA/WPA2-Personal (or WPA2/WPA3 transition): supported.
    case personal = 0
    /// No passphrase: not supported.
    case open = 1
    /// WEP, WPA3-only, Enterprise, …: not supported.
    case unsupported = 2
}

public enum ConfigureResult: UInt8, Sendable {
    /// Joined, got an address, saved; the device restarts on it.
    case saved = 0
    case invalidSsid = 1
    case invalidPassphrase = 2
    case invalidSite = 3
    /// The network wasn't seen.
    case networkNotFound = 4
    /// The access point refused the passphrase.
    case wrongPassphrase = 5
    /// Joined but got no address (DHCP).
    case noAddress = 6
    /// Any other join failure or timeout.
    case joinFailed = 7
    /// The test passed but the settings could not be written.
    case saveFailed = 8
}

public enum ErrorCode: UInt8, Sendable {
    /// Truncated, bad UTF-8, a string too long, version 0, or a request type
    /// in the response range.
    case malformed = 1
    /// A request type this device doesn't know (a newer client).
    case unknownType = 2
}

/// The settings a client sends: SSID, passphrase, latitude (deg, north
/// positive), longitude (deg, east positive), altitude (m above the WGS-84
/// ellipsoid).
public struct Configure: Equatable, Sendable {
    public var ssid: String
    public var passphrase: String
    public var latDeg: Double
    public var lonDeg: Double
    public var altM: Double

    public init(ssid: String, passphrase: String, latDeg: Double, lonDeg: Double, altM: Double) {
        self.ssid = ssid
        self.passphrase = passphrase
        self.latDeg = latDeg
        self.lonDeg = lonDeg
        self.altM = altM
    }

    /// The device's acceptance rules (also the settings record's, 0084), in
    /// UTF-8 bytes: SSID 1...32; passphrase 8...63, or 64 hex digits; latitude
    /// −90...90, longitude −180...180, altitude −500...9000 m, all finite.
    /// `nil` when the device would accept these settings.
    public func check() -> ConfigureResult? {
        let s = Array(ssid.utf8), p = Array(passphrase.utf8)
        if s.isEmpty || s.count > SetupProtocol.ssidMax {
            return .invalidSsid
        }
        let passOK = (8...63).contains(p.count)
            || (p.count == 64 && p.allSatisfy { $0.isASCIIHexDigit })
        if !passOK {
            return .invalidPassphrase
        }
        func inRange(_ v: Double, _ lo: Double, _ hi: Double) -> Bool {
            v.isFinite && (lo...hi).contains(v)
        }
        if !inRange(latDeg, -90, 90) || !inRange(lonDeg, -180, 180) || !inRange(altM, -500, 9000) {
            return .invalidSite
        }
        return nil
    }
}

public enum Request: Equatable, Sendable {
    case getInfo
    case scan
    case configure(Configure)

    public var typeByte: UInt8 {
        switch self {
        case .getInfo: MessageType.getInfo
        case .scan: MessageType.scan
        case .configure: MessageType.configure
        }
    }
}

/// Device status.
public struct Info: Equatable, Sendable {
    public static let wifiStored: UInt8 = 1
    public static let siteStored: UInt8 = 2

    public var build: UInt32
    public var version: String
    public var channel: Channel
    public var mac: [UInt8]
    /// Bit 0: Wi-Fi stored; bit 1: site stored.
    public var flags: UInt8
    public var reason: SetupReason
    /// Stored SSID, empty if none.
    public var ssid: String
    /// Last successful element fetch, Unix seconds; 0 = none.
    public var lastFetchUnixS: Int64
    /// Last build that failed its health check; 0 = none.
    public var failedBuild: UInt32
    /// This setup window's last `Configure` result.
    public var lastResult: ConfigureResult?

    public init(
        build: UInt32, version: String, channel: Channel, mac: [UInt8], flags: UInt8,
        reason: SetupReason, ssid: String, lastFetchUnixS: Int64, failedBuild: UInt32,
        lastResult: ConfigureResult?
    ) {
        self.build = build
        self.version = version
        self.channel = channel
        self.mac = mac
        self.flags = flags
        self.reason = reason
        self.ssid = ssid
        self.lastFetchUnixS = lastFetchUnixS
        self.failedBuild = failedBuild
        self.lastResult = lastResult
    }

    public var wifiStored: Bool { flags & Self.wifiStored != 0 }
    public var siteStored: Bool { flags & Self.siteStored != 0 }
    public var macString: String { mac.map { String(format: "%02X", $0) }.joined(separator: ":") }
}

/// A network from the device's scan.
public struct Network: Equatable, Sendable, Hashable {
    public var rssiDbm: Int8
    public var security: Security
    public var ssid: String

    public init(rssiDbm: Int8, security: Security, ssid: String) {
        self.rssiDbm = rssiDbm
        self.security = security
        self.ssid = ssid
    }
}

public enum Response: Equatable, Sendable {
    case info(Info)
    case network(Network)
    /// After the last `network`: how many were sent, and whether the scan
    /// itself worked (`false`: the list is empty because scanning failed).
    case scanDone(count: UInt8, ok: Bool)
    /// The device is trying the new network (it can take ~20 s).
    case testing
    case configureResult(ConfigureResult)
    /// `requestType` is the type byte of the request it answers.
    case error(code: ErrorCode, requestType: UInt8)
}

public enum DecodeError: Error, Equatable {
    /// Fewer than two bytes, or a field cut short.
    case truncated
    /// Version byte 0.
    case version
    /// A type this reader doesn't know (or from the wrong direction).
    case unknownType(UInt8)
    /// A string that isn't UTF-8 or is longer than its limit.
    case badString
    /// An enum byte outside its values.
    case badValue
}

/// The frame would exceed `maxFrame`, or a string exceeds its limit.
public struct EncodeError: Error, Equatable {}

/// A decoded frame: the sender's version and the message.
public struct Frame<M: Equatable>: Equatable {
    public var version: UInt8
    public var message: M
}

private struct Writer {
    var bytes: [UInt8]

    init(type: UInt8) { bytes = [SetupProtocol.version, type] }

    mutating func append(_ b: [UInt8]) throws {
        guard bytes.count + b.count <= SetupProtocol.maxFrame else { throw EncodeError() }
        bytes += b
    }

    mutating func string(_ s: String, max: Int) throws {
        let u = Array(s.utf8)
        guard u.count <= max else { throw EncodeError() }
        try append([UInt8(u.count)])
        try append(u)
    }

    mutating func f64(_ v: Double) throws {
        try append(withUnsafeBytes(of: v.bitPattern.littleEndian, Array.init))
    }
}

private struct Reader {
    var bytes: ArraySlice<UInt8>

    mutating func take(_ n: Int) throws -> ArraySlice<UInt8> {
        guard bytes.count >= n else { throw DecodeError.truncated }
        let head = bytes.prefix(n)
        bytes = bytes.dropFirst(n)
        return head
    }

    mutating func u8() throws -> UInt8 { try take(1).first! }

    mutating func le<T: FixedWidthInteger>(_: T.Type) throws -> T {
        let raw = try take(MemoryLayout<T>.size)
        var v: T = 0
        for (i, b) in raw.enumerated() { v |= T(truncatingIfNeeded: b) << (8 * i) }
        return v
    }

    mutating func string(max: Int) throws -> String {
        let n = Int(try u8())
        let raw = try take(n)
        guard n <= max, let s = String(bytes: raw, encoding: .utf8) else {
            throw DecodeError.badString
        }
        return s
    }

    mutating func enumByte<T: RawRepresentable>(_: T.Type) throws -> T where T.RawValue == UInt8 {
        guard let v = T(rawValue: try u8()) else { throw DecodeError.badValue }
        return v
    }
}

extension Request {
    /// The request as one frame.
    public func encode() throws -> Data {
        var w = Writer(type: typeByte)
        if case let .configure(c) = self {
            try w.string(c.ssid, max: SetupProtocol.ssidMax)
            try w.string(c.passphrase, max: SetupProtocol.passphraseMax)
            try w.f64(c.latDeg)
            try w.f64(c.lonDeg)
            try w.f64(c.altM)
        }
        return Data(w.bytes)
    }
}

extension Response {
    /// Decodes a response frame. Trailing bytes after known fields are ignored.
    public static func decode(_ frame: Data) throws -> Frame<Response> {
        let b = [UInt8](frame)
        guard b.count >= 2 else { throw DecodeError.truncated }
        guard b[0] != 0 else { throw DecodeError.version }
        var r = Reader(bytes: b[2...])
        let message: Response
        switch b[1] {
        case MessageType.info:
            let build = try r.le(UInt32.self)
            let version = try r.string(max: SetupProtocol.versionMax)
            let channel = try r.enumByte(Channel.self)
            let mac = Array(try r.take(6))
            let flags = try r.u8()
            let reason = try r.enumByte(SetupReason.self)
            let ssid = try r.string(max: SetupProtocol.ssidMax)
            let lastFetch = try r.le(Int64.self)
            let failed = try r.le(UInt32.self)
            let last = try r.u8()
            let lastResult: ConfigureResult?
            if last == 0xFF {
                lastResult = nil
            } else if let v = ConfigureResult(rawValue: last) {
                lastResult = v
            } else {
                throw DecodeError.badValue
            }
            message = .info(Info(
                build: build, version: version, channel: channel, mac: mac, flags: flags,
                reason: reason, ssid: ssid, lastFetchUnixS: lastFetch, failedBuild: failed,
                lastResult: lastResult))
        case MessageType.network:
            let rssi = Int8(bitPattern: try r.u8())
            let security = try r.enumByte(Security.self)
            let ssid = try r.string(max: SetupProtocol.ssidMax)
            message = .network(Network(rssiDbm: rssi, security: security, ssid: ssid))
        case MessageType.scanDone:
            let count = try r.u8()
            let ok = try r.u8() != 0
            message = .scanDone(count: count, ok: ok)
        case MessageType.testing:
            message = .testing
        case MessageType.configureResult:
            message = .configureResult(try r.enumByte(ConfigureResult.self))
        case MessageType.error:
            let code = try r.enumByte(ErrorCode.self)
            message = .error(code: code, requestType: try r.u8())
        case let t:
            throw DecodeError.unknownType(t)
        }
        return Frame(version: b[0], message: message)
    }
}

extension UInt8 {
    var isASCIIHexDigit: Bool {
        (0x30...0x39).contains(self) || (0x41...0x46).contains(self) || (0x61...0x66).contains(self)
    }
}
