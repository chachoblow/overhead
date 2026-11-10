import CoreBluetooth
import Foundation
import Observation

/// A device seen advertising the setup service.
public struct DiscoveredDevice: Identifiable, Hashable, Sendable {
    public let id: UUID
    public var name: String
    public var rssi: Int
    public var lastSeen: Date
}

public enum SetupError: LocalizedError, Equatable {
    case bluetoothUnavailable
    case connectFailed
    case pairingFailed(String)
    case serviceMissing
    case mtuTooSmall(Int)
    case linkLost
    case timedOut(String)
    case device(ErrorCode)
    case malformed(DecodeError)
    case unexpected(String)
    case notConnected

    public var errorDescription: String? {
        switch self {
        case .bluetoothUnavailable: "Bluetooth is off or not allowed for Overhead."
        case .connectFailed: "Couldn't connect to the Overhead. Move closer and try again."
        case let .pairingFailed(why): "Pairing didn't finish (\(why)). Check the code on the Overhead's display and try again."
        case .serviceMissing: "This device doesn't offer the Overhead setup service. Turn Bluetooth off and on in Settings, then try again."
        case let .mtuTooSmall(n): "The Bluetooth link is too small (\(n) bytes)."
        case .linkLost: "The connection to the Overhead was lost."
        case let .timedOut(what): "The Overhead didn't answer (\(what))."
        case let .device(code): "The Overhead refused the request (\(code))."
        case let .malformed(e): "The Overhead sent something this app can't read (\(e))."
        case let .unexpected(what): "Unexpected reply from the Overhead (\(what))."
        case .notConnected: "Not connected."
        }
    }
}

/// What a `Configure` came to.
public enum ConfigureOutcome: Equatable, Sendable {
    /// The device answered.
    case result(ConfigureResult)
    /// The link dropped after `Testing`; read `Info.lastResult` after reconnecting.
    case lostWhileTesting
}

/// The 0087 setup service over CoreBluetooth: discovery, connection with
/// retries, pairing (the device asks for LE Secure Connections with a passkey
/// on connect, so iOS prompts for the code shown on the Sharp), and requests
/// answered one at a time.
@MainActor
@Observable
public final class SetupCentral: NSObject {
    public enum Power: Equatable { case unknown, on, off, unauthorized, unsupported }
    public enum Link: Equatable {
        case idle
        case connecting(attempt: Int)
        /// Connected; waiting for pairing and the service.
        case pairing
        case ready(name: String)
    }

    public private(set) var power: Power = .unknown
    public private(set) var devices: [DiscoveredDevice] = []
    public private(set) var link: Link = .idle
    public private(set) var isScanning = false

    // Timeouts (seconds).
    static let connectTimeout = 12.0
    static let connectAttempts = 3
    /// Covers the user reading and typing the passkey (pairing itself ~4.4 s).
    static let pairingTimeout = 120.0
    static let infoTimeout = 10.0
    /// The device may rescan when its cache is older than 120 s.
    static let scanTimeout = 30.0
    static let testingAckTimeout = 10.0
    /// Testing a network takes ~10–20 s on the device.
    static let resultTimeout = 60.0

    nonisolated private static let service = CBUUID(string: SetupProtocol.serviceUUID)
    nonisolated private static let requestChar = CBUUID(string: SetupProtocol.requestUUID)
    nonisolated private static let responseChar = CBUUID(string: SetupProtocol.responseUUID)

    @ObservationIgnored private var central: CBCentralManager!
    @ObservationIgnored private var peripherals: [UUID: CBPeripheral] = [:]
    @ObservationIgnored private var peripheral: CBPeripheral?
    @ObservationIgnored private var request: CBCharacteristic?
    @ObservationIgnored private var response: CBCharacteristic?
    @ObservationIgnored private var powerWaiters: [CheckedContinuation<Void, Never>] = []

    private enum Op: Hashable { case connect, services, characteristics, subscribe, write, response }
    @ObservationIgnored private var waiters: [Op: CheckedContinuation<Void, Error>] = [:]
    @ObservationIgnored private var tokens: [Op: Int] = [:]
    @ObservationIgnored private var nextToken = 0
    @ObservationIgnored private var inbox: [Response] = []

    public override init() {
        super.init()
        central = CBCentralManager(delegate: self, queue: .main)
    }

    // MARK: Discovery

    public func startScan() {
        devices = []
        isScanning = true
        if power == .on { beginScan() }
    }

    public func stopScan() {
        isScanning = false
        if central.isScanning { central.stopScan() }
    }

    /// Scans until `id` advertises again after `since` (a device that saved
    /// restarts out of setup and never does). True when it reappeared.
    public func waitForAdvertising(_ id: UUID, since: Date, timeout: Double) async -> Bool {
        startScan()
        defer { stopScan() }
        let end = Date().addingTimeInterval(timeout)
        while Date() < end {
            if devices.contains(where: { $0.id == id && $0.lastSeen > since }) { return true }
            try? await Task.sleep(for: .milliseconds(500))
            if Task.isCancelled { return false }
        }
        return false
    }

    private func beginScan() {
        // No service filter: the service UUID is in the scan response, the
        // name in the advertisement; match either.
        central.scanForPeripherals(withServices: nil, options: [
            CBCentralManagerScanOptionAllowDuplicatesKey: true,
        ])
    }

    private func waitForPower() async throws {
        if power == .on { return }
        if power == .unknown {
            await withCheckedContinuation { powerWaiters.append($0) }
        }
        guard power == .on else { throw SetupError.bluetoothUnavailable }
    }

    // MARK: Connection

    /// Connects (retrying), pairs and subscribes. iOS shows the passkey prompt.
    public func connect(_ id: UUID) async throws {
        try await waitForPower()
        disconnect()
        let known = peripherals[id] ?? central.retrievePeripherals(withIdentifiers: [id]).first
        guard let p = known else { throw SetupError.connectFailed }
        peripherals[id] = p
        p.delegate = self
        peripheral = p
        inbox = []

        var connected = false
        for attempt in 1...Self.connectAttempts {
            link = .connecting(attempt: attempt)
            // A cancelled attempt's disconnect may have cleared it.
            peripheral = p
            central.connect(p, options: nil)
            do {
                try await wait(.connect, timeout: Self.connectTimeout, what: "connect")
                connected = true
                break
            } catch {
                central.cancelPeripheralConnection(p)
                if Task.isCancelled { break }
                try? await Task.sleep(for: .milliseconds(500))
            }
        }
        guard connected else {
            fail()
            throw SetupError.connectFailed
        }

        do {
            link = .pairing
            p.discoverServices([Self.service])
            try await wait(.services, timeout: Self.infoTimeout, what: "services")
            guard let svc = p.services?.first(where: { $0.uuid == Self.service }) else {
                throw SetupError.serviceMissing
            }
            p.discoverCharacteristics([Self.requestChar, Self.responseChar], for: svc)
            try await wait(.characteristics, timeout: Self.infoTimeout, what: "characteristics")
            guard let req = svc.characteristics?.first(where: { $0.uuid == Self.requestChar }),
                  let resp = svc.characteristics?.first(where: { $0.uuid == Self.responseChar })
            else { throw SetupError.serviceMissing }
            request = req
            response = resp
            // Needs an authenticated link: completes once the user has
            // entered the passkey and pairing finished.
            p.setNotifyValue(true, for: resp)
            try await wait(.subscribe, timeout: Self.pairingTimeout, what: "pairing")
            let mtu = p.maximumWriteValueLength(for: .withoutResponse)
            guard mtu >= SetupProtocol.maxFrame else { throw SetupError.mtuTooSmall(mtu + 3) }
            link = .ready(name: p.name ?? devices.first(where: { $0.id == id })?.name ?? "Overhead")
        } catch {
            disconnect()
            throw error
        }
    }

    public func disconnect() {
        if let p = peripheral { central.cancelPeripheralConnection(p) }
        fail()
    }

    /// Ends the connection state and fails every waiter.
    private func fail(_ error: Error = SetupError.linkLost) {
        peripheral = nil
        request = nil
        response = nil
        link = .idle
        let all = waiters
        waiters = [:]
        tokens = [:]
        for (_, w) in all { w.resume(throwing: error) }
    }

    // MARK: Requests

    public func getInfo() async throws -> Info {
        try await send(.getInfo)
        switch try await nextResponse(timeout: Self.infoTimeout, what: "info") {
        case let .info(i): return i
        case let other: throw unexpected(other)
        }
    }

    /// The device's scanned networks, strongest first, and whether its scan worked.
    public func scanNetworks() async throws -> (networks: [Network], ok: Bool) {
        try await send(.scan)
        var list: [Network] = []
        while true {
            switch try await nextResponse(timeout: Self.scanTimeout, what: "networks") {
            case let .network(n): list.append(n)
            case let .scanDone(_, ok): return (list, ok)
            case let other: throw unexpected(other)
            }
        }
    }

    /// Sends the settings. The device tests the network first (`Testing`),
    /// then answers; on `saved` it restarts and the link ends.
    public func configure(_ c: Configure) async throws -> ConfigureOutcome {
        try await send(.configure(c))
        switch try await nextResponse(timeout: Self.testingAckTimeout, what: "configure") {
        case .testing: break
        case let .configureResult(r): return .result(r)
        case let other: throw unexpected(other)
        }
        do {
            switch try await nextResponse(timeout: Self.resultTimeout, what: "network test") {
            case let .configureResult(r): return .result(r)
            case let other: throw unexpected(other)
            }
        } catch SetupError.linkLost {
            return .lostWhileTesting
        }
    }

    private func unexpected(_ r: Response) -> SetupError {
        if case let .error(code, _) = r { return .device(code) }
        return .unexpected("\(r)")
    }

    private func send(_ r: Request) async throws {
        guard let p = peripheral, let c = request, case .ready = link else { throw SetupError.notConnected }
        inbox = []
        p.writeValue(try r.encode(), for: c, type: .withResponse)
        try await wait(.write, timeout: Self.infoTimeout, what: "write")
    }

    private func nextResponse(timeout: Double, what: String) async throws -> Response {
        if inbox.isEmpty {
            try await wait(.response, timeout: timeout, what: what)
        }
        guard !inbox.isEmpty else { throw SetupError.linkLost }
        return inbox.removeFirst()
    }

    // MARK: Waiting

    private func wait(_ op: Op, timeout: Double, what: String) async throws {
        nextToken += 1
        let token = nextToken
        try await withCheckedThrowingContinuation { (c: CheckedContinuation<Void, Error>) in
            waiters[op]?.resume(throwing: SetupError.unexpected("overlapping \(op)"))
            waiters[op] = c
            tokens[op] = token
            Task { @MainActor [weak self] in
                try? await Task.sleep(for: .seconds(timeout))
                guard let self, self.tokens[op] == token else { return }
                self.finish(op, SetupError.timedOut(what))
            }
        }
    }

    private func finish(_ op: Op, _ error: Error? = nil) {
        guard let w = waiters.removeValue(forKey: op) else { return }
        tokens[op] = nil
        if let error { w.resume(throwing: error) } else { w.resume() }
    }
}

extension SetupCentral: CBCentralManagerDelegate {
    nonisolated public func centralManagerDidUpdateState(_ central: CBCentralManager) {
        let state = central.state
        MainActor.assumeIsolated {
            power = switch state {
            case .poweredOn: .on
            case .poweredOff, .resetting: .off
            case .unauthorized: .unauthorized
            case .unsupported: .unsupported
            default: .unknown
            }
            if power != .unknown {
                let w = powerWaiters
                powerWaiters = []
                w.forEach { $0.resume() }
            }
            if power == .on, isScanning { beginScan() }
            if power != .on { fail(SetupError.bluetoothUnavailable) }
        }
    }

    nonisolated public func centralManager(
        _ central: CBCentralManager, didDiscover peripheral: CBPeripheral,
        advertisementData: [String: Any], rssi RSSI: NSNumber
    ) {
        let name = (advertisementData[CBAdvertisementDataLocalNameKey] as? String) ?? peripheral.name
        let uuids = (advertisementData[CBAdvertisementDataServiceUUIDsKey] as? [CBUUID]) ?? []
        let rssi = RSSI.intValue
        MainActor.assumeIsolated {
            let hasService = uuids.contains(Self.service)
            guard hasService || (name?.hasPrefix(SetupProtocol.namePrefix) ?? false) else { return }
            peripherals[peripheral.identifier] = peripheral
            let shown = name ?? "Overhead"
            if let i = devices.firstIndex(where: { $0.id == peripheral.identifier }) {
                devices[i].rssi = rssi == 127 ? devices[i].rssi : rssi
                devices[i].lastSeen = Date()
                if name != nil { devices[i].name = shown }
            } else {
                devices.append(DiscoveredDevice(id: peripheral.identifier, name: shown, rssi: rssi, lastSeen: Date()))
            }
        }
    }

    nonisolated public func centralManager(_ central: CBCentralManager, didConnect peripheral: CBPeripheral) {
        MainActor.assumeIsolated { finish(.connect) }
    }

    nonisolated public func centralManager(
        _ central: CBCentralManager, didFailToConnect peripheral: CBPeripheral, error: Error?
    ) {
        MainActor.assumeIsolated { finish(.connect, SetupError.connectFailed) }
    }

    nonisolated public func centralManager(
        _ central: CBCentralManager, didDisconnectPeripheral peripheral: CBPeripheral, error: Error?
    ) {
        let id = peripheral.identifier
        MainActor.assumeIsolated {
            guard self.peripheral?.identifier == id else { return }
            // A connect attempt that dropped is retried by `connect`.
            if waiters[.connect] != nil { return }
            fail()
        }
    }
}

extension SetupCentral: CBPeripheralDelegate {
    nonisolated public func peripheral(_ peripheral: CBPeripheral, didDiscoverServices error: Error?) {
        MainActor.assumeIsolated { finish(.services, error.map { _ in SetupError.serviceMissing }) }
    }

    nonisolated public func peripheral(
        _ peripheral: CBPeripheral, didDiscoverCharacteristicsFor service: CBService, error: Error?
    ) {
        MainActor.assumeIsolated { finish(.characteristics, error.map { _ in SetupError.serviceMissing }) }
    }

    nonisolated public func peripheral(
        _ peripheral: CBPeripheral, didUpdateNotificationStateFor characteristic: CBCharacteristic, error: Error?
    ) {
        let why = error?.localizedDescription
        MainActor.assumeIsolated { finish(.subscribe, why.map { SetupError.pairingFailed($0) }) }
    }

    nonisolated public func peripheral(
        _ peripheral: CBPeripheral, didWriteValueFor characteristic: CBCharacteristic, error: Error?
    ) {
        let why = error?.localizedDescription
        MainActor.assumeIsolated { finish(.write, why.map { SetupError.unexpected("write: \($0)") }) }
    }

    nonisolated public func peripheral(
        _ peripheral: CBPeripheral, didUpdateValueFor characteristic: CBCharacteristic, error: Error?
    ) {
        guard characteristic.uuid == SetupCentral.responseChar, error == nil, let data = characteristic.value else { return }
        MainActor.assumeIsolated {
            do {
                inbox.append(try Response.decode(data).message)
            } catch let DecodeError.unknownType(t) where t >= 0x80 {
                return // A newer device's response type: ignored.
            } catch let e as DecodeError {
                finish(.response, SetupError.malformed(e))
                return
            } catch {
                return
            }
            finish(.response)
        }
    }
}
