import Foundation
import Testing
@testable import OverheadSetupKit

// Byte vectors copied from `setup/src/tests.rs` (`fixed_vectors` and the
// malformed/newer-peer cases); change only with a new protocol version.

let configure = Configure(ssid: "home", passphrase: "password1", latDeg: 51.5, lonDeg: -0.125, altM: 35.0)

let info = Info(
    build: 12, version: "0.1.12-beta", channel: .beta,
    mac: [0x80, 0x45, 0x6b, 0x2a, 0x45, 0x08],
    flags: Info.wifiStored | Info.siteStored, reason: .button, ssid: "home",
    lastFetchUnixS: 1_794_000_000, failedBuild: 0, lastResult: .wrongPassphrase)

let infoBytes: [UInt8] = [
    0x01, 0x81,
    12, 0, 0, 0,
    11] + Array("0.1.12-beta".utf8) + [
    1,
    0x80, 0x45, 0x6b, 0x2a, 0x45, 0x08,
    0b11,
    3,
    4] + Array("home".utf8) + [
    0x80, 0x44, 0xEE, 0x6A, 0, 0, 0, 0,
    0, 0, 0, 0,
    5,
]

func decode(_ b: [UInt8]) throws -> Frame<Response> { try Response.decode(Data(b)) }

@Test func requestVectors() throws {
    #expect(try [UInt8](Request.getInfo.encode()) == [0x01, 0x01])
    #expect(try [UInt8](Request.scan.encode()) == [0x01, 0x02])
    let expected: [UInt8] = [0x01, 0x03, 4] + Array("home".utf8) + [9] + Array("password1".utf8) + [
        0, 0, 0, 0, 0, 0xC0, 0x49, 0x40,
        0, 0, 0, 0, 0, 0, 0xC0, 0xBF,
        0, 0, 0, 0, 0, 0x80, 0x41, 0x40,
    ]
    #expect(try [UInt8](Request.configure(configure).encode()) == expected)
}

@Test func responseVectors() throws {
    #expect(try decode(infoBytes) == Frame(version: 1, message: .info(info)))
    #expect(try decode([0x01, 0x82, 0xC3, 0, 4] + Array("home".utf8)).message
        == .network(Network(rssiDbm: -61, security: .personal, ssid: "home")))
    #expect(try decode([0x01, 0x83, 3, 1]).message == .scanDone(count: 3, ok: true))
    #expect(try decode([0x01, 0x84]).message == .testing)
    #expect(try decode([0x01, 0x85, 5]).message == .configureResult(.wrongPassphrase))
    #expect(try decode([0x01, 0x86, 2, 0x09]).message == .error(code: .unknownType, requestType: 0x09))
}

@Test func infoWithoutResult() throws {
    var b = infoBytes
    b[b.count - 1] = 0xFF
    var none = info
    none.lastResult = nil
    #expect(try decode(b).message == .info(none))
    #expect(info.macString == "80:45:6B:2A:45:08")
}

@Test func malformedResponsesAreRefused() {
    #expect(throws: DecodeError.badValue) { try decode([1, 0x85, 99]) }
    #expect(throws: DecodeError.truncated) { try decode([1, 0x83, 1]) }
    #expect(throws: DecodeError.unknownType(0x01)) { try decode([1, 0x01]) }
    #expect(throws: DecodeError.version) { try decode([0, 0x84]) }
    #expect(throws: DecodeError.truncated) { try decode([1]) }
    // Every cut of an Info frame is truncated, never a partial message.
    for cut in 2..<infoBytes.count {
        #expect(throws: DecodeError.truncated) { try decode(Array(infoBytes[..<cut])) }
    }
    // Bad UTF-8 in a network's SSID.
    #expect(throws: DecodeError.badString) { try decode([1, 0x82, 0xC3, 0, 1, 0xFF]) }
    // An SSID length over the limit, with the bytes present.
    #expect(throws: DecodeError.badString) {
        try decode([1, 0x82, 0xC3, 0, 33] + Array(repeating: UInt8(ascii: "a"), count: 33))
    }
}

/// A newer device (version 2, appended fields) against this client.
@Test func newerDeviceKeepsWorking() throws {
    var b = infoBytes
    b[0] = 2
    b.append(0x42)
    #expect(try decode(b) == Frame(version: 2, message: .info(info)))
    #expect(try decode([2, 0x84, 7, 7]).message == .testing)
}

@Test func longStringsAreRefused() throws {
    var c = configure
    c.ssid = String(repeating: "s", count: SetupProtocol.ssidMax + 1)
    #expect(throws: EncodeError()) { try Request.configure(c).encode() }
    c.ssid = String(repeating: "s", count: SetupProtocol.ssidMax)
    c.passphrase = String(repeating: "p", count: SetupProtocol.passphraseMax)
    #expect(try Request.configure(c).encode().count <= SetupProtocol.maxFrame)
}

@Test func checkMatchesTheSettingsRules() {
    #expect(configure.check() == nil)
    func with(_ f: (inout Configure) -> Void) -> ConfigureResult? {
        var c = configure
        f(&c)
        return c.check()
    }
    #expect(with { $0.ssid = "" } == .invalidSsid)
    #expect(with { $0.ssid = "123456789012345678901234567890123" } == .invalidSsid)
    #expect(with { $0.passphrase = "short" } == .invalidPassphrase)
    #expect(with { $0.passphrase = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef" } == nil)
    #expect(with { $0.passphrase = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg" } == .invalidPassphrase)
    #expect(with { $0.latDeg = 90.5 } == .invalidSite)
    #expect(with { $0.lonDeg = .nan } == .invalidSite)
    #expect(with { $0.altM = 9_001 } == .invalidSite)
    #expect(with { $0.lonDeg = -180 } == nil)
    // Limits are UTF-8 bytes, as on the device: 4 × 2-byte "é" is 8 bytes.
    #expect(with { $0.passphrase = "éééé" } == nil)
    #expect(with { $0.passphrase = "ééé" } == .invalidPassphrase)
}
