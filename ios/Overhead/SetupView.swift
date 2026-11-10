import OverheadSetupKit
import SwiftUI

/// One device: connect and pair, show its status, then send Wi-Fi and location.
struct SetupView: View {
    let device: DiscoveredDevice

    @Environment(SetupCentral.self) private var central
    @Environment(LocationProvider.self) private var location

    @State private var info: Info?
    @State private var connectError: String?
    @State private var networks: [Network] = []
    @State private var scanOK = true
    @State private var scanning = false
    @State private var ssid = ""
    @State private var otherNetwork = false
    @State private var passphrase = ""
    @State private var showPassphrase = false
    @State private var manualLocation = false
    @State private var manualSite = ""
    @State private var formError: String?
    @State private var job: Job?

    enum Job: Equatable {
        case testing
        case reconnecting
        case finished(ConfigureResult)
        /// The link dropped during the test and the device didn't come back
        /// to setup, which is what it does after saving.
        case vanished
        case failed(String)
    }

    var body: some View {
        Group {
            if let info {
                form(info)
            } else {
                connecting
            }
        }
        .navigationTitle(device.name)
        .navigationBarTitleDisplayMode(.inline)
        .task { await connect() }
        .onDisappear { central.disconnect() }
        .sheet(item: Binding(get: { job.map(JobBox.init) }, set: { job = $0?.job })) { box in
            JobSheet(job: box.job, onDone: { done(box.job) })
                .interactiveDismissDisabled(box.job == .testing || box.job == .reconnecting)
        }
    }

    // MARK: Connecting

    private var connecting: some View {
        VStack(spacing: 20) {
            if let connectError {
                Image(systemName: "exclamationmark.triangle")
                    .font(.largeTitle)
                Text(connectError)
                    .multilineTextAlignment(.center)
                Button("Try Again") { Task { await connect() } }
                    .buttonStyle(.borderedProminent)
            } else {
                ProgressView()
                switch central.link {
                case let .connecting(attempt):
                    Text(attempt > 1 ? "Connecting (try \(attempt))…" : "Connecting…")
                case .pairing, .ready:
                    Text("Pairing…")
                    Text("When your iPhone asks, enter the 6-digit code shown on the Overhead's display.")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                        .multilineTextAlignment(.center)
                case .idle:
                    Text("Connecting…")
                }
            }
        }
        .padding(32)
    }

    private func connect() async {
        connectError = nil
        info = nil
        do {
            try await central.connect(device.id)
            info = try await central.getInfo()
            if location.site == nil { location.request() }
            await scan()
        } catch {
            connectError = error.localizedDescription
        }
    }

    // MARK: Form

    @ViewBuilder
    private func form(_ info: Info) -> some View {
        Form {
            Section("Wi-Fi") {
                if scanning && networks.isEmpty {
                    HStack(spacing: 12) {
                        ProgressView()
                        Text("Asking the Overhead for networks…").foregroundStyle(.secondary)
                    }
                }
                if !scanOK {
                    Text("The Overhead's scan failed. Try again, or type the network name.")
                        .foregroundStyle(.secondary)
                }
                ForEach(networks, id: \.self) { n in
                    Button {
                        ssid = n.ssid
                        otherNetwork = false
                    } label: {
                        HStack {
                            Image(systemName: "checkmark")
                                .opacity(!otherNetwork && ssid == n.ssid ? 1 : 0)
                            Text(n.ssid.isEmpty ? "(hidden)" : n.ssid)
                                .foregroundStyle(n.security.supported ? .primary : .secondary)
                            Spacer()
                            if n.security == .open {
                                Text("No password").font(.caption).foregroundStyle(.secondary)
                            } else if n.security == .unsupported {
                                Text("Not supported").font(.caption).foregroundStyle(.secondary)
                            } else {
                                Image(systemName: "lock.fill").font(.caption).foregroundStyle(.secondary)
                            }
                            SignalBars(rssi: Int(n.rssiDbm))
                        }
                    }
                    .disabled(!n.security.supported || n.ssid.isEmpty)
                }
                Button {
                    otherNetwork = true
                    ssid = ""
                } label: {
                    HStack {
                        Image(systemName: "checkmark").opacity(otherNetwork ? 1 : 0)
                        Text("Other network…").foregroundStyle(.primary)
                    }
                }
                if otherNetwork {
                    TextField("Network name", text: $ssid)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                }
                HStack {
                    Group {
                        if showPassphrase {
                            TextField("Password", text: $passphrase)
                        } else {
                            SecureField("Password", text: $passphrase)
                        }
                    }
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    Button {
                        showPassphrase.toggle()
                    } label: {
                        Image(systemName: showPassphrase ? "eye.slash" : "eye")
                    }
                    .buttonStyle(.borderless)
                }
                Button("Scan Again") { Task { await scan() } }
                    .disabled(scanning)
            }

            Section {
                if manualLocation {
                    TextField("Latitude, longitude", text: $manualSite)
                        .keyboardType(.numbersAndPunctuation)
                        .autocorrectionDisabled()
                } else {
                    phoneLocation
                }
                Toggle("Enter coordinates", isOn: $manualLocation)
            } header: {
                Text("Location")
            } footer: {
                Text("The Overhead tracks satellites from here. It's stored on the device only.")
            }

            Section {
                Button {
                    Task { await save() }
                } label: {
                    Text("Save to Overhead").frame(maxWidth: .infinity)
                }
                .disabled(ssid.isEmpty || passphrase.isEmpty)
                if let formError {
                    Text(formError).foregroundStyle(.red)
                }
            }

            StatusSection(info: info)
        }
    }

    @ViewBuilder
    private var phoneLocation: some View {
        switch location.state {
        case .idle:
            Button("Use This iPhone's Location") { location.request() }
        case .locating:
            HStack(spacing: 12) {
                ProgressView()
                Text("Finding this iPhone's location…").foregroundStyle(.secondary)
            }
        case .found:
            if let s = location.site {
                LabeledContent("This iPhone", value: String(format: "%.4f, %.4f", s.lat, s.lon))
            }
            Button("Update") { location.request() }
        case .denied:
            Text("Location access is off for Overhead. Allow it in Settings, or enter coordinates.")
                .foregroundStyle(.secondary)
        case let .failed(why):
            Text("Couldn't get a location (\(why)).").foregroundStyle(.secondary)
            Button("Try Again") { location.request() }
        }
    }

    private func scan() async {
        scanning = true
        defer { scanning = false }
        do {
            let (list, ok) = try await central.scanNetworks()
            networks = list
            scanOK = ok
        } catch {
            formError = error.localizedDescription
        }
    }

    // MARK: Saving

    private func site() -> (Double, Double, Double)? {
        if manualLocation {
            let parts = manualSite.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
            guard parts.count == 2, let lat = Double(parts[0]), let lon = Double(parts[1]) else { return nil }
            return (lat, lon, 0)
        }
        return location.site.map { ($0.lat, $0.lon, $0.alt) }
    }

    private func save() async {
        formError = nil
        guard let (lat, lon, alt) = site() else {
            formError = manualLocation
                ? "Enter the location as latitude, longitude (for example 51.5, -0.125)."
                : "Waiting for this iPhone's location."
            return
        }
        let c = Configure(ssid: ssid, passphrase: passphrase, latDeg: lat, lonDeg: lon, altM: alt)
        if let problem = c.check() {
            formError = problem.message
            return
        }
        job = .testing
        do {
            switch try await central.configure(c) {
            case let .result(r):
                job = .finished(r)
            case .lostWhileTesting:
                await recover()
            }
        } catch {
            job = .failed(error.localizedDescription)
        }
    }

    /// The link dropped while the device tested the network: if it comes back
    /// to setup, reconnect (pairing again) and read the result it kept.
    private func recover() async {
        job = .reconnecting
        let since = Date()
        guard await central.waitForAdvertising(device.id, since: since, timeout: 60) else {
            job = .vanished
            return
        }
        do {
            try await central.connect(device.id)
            let i = try await central.getInfo()
            info = i
            if let r = i.lastResult {
                job = .finished(r)
            } else {
                job = .failed("The Overhead has no result for that attempt. Try saving again.")
            }
        } catch {
            job = .failed(error.localizedDescription)
        }
    }

    private func done(_ finished: Job) {
        job = nil
        switch finished {
        case .finished(.saved), .vanished:
            central.disconnect()
        case .failed:
            if case .ready = central.link {} else { info = nil; Task { await connect() } }
        default:
            break
        }
    }
}

private struct JobBox: Identifiable {
    let job: SetupView.Job
    var id: String { "\(job)" }
}

private struct JobSheet: View {
    let job: SetupView.Job
    let onDone: () -> Void

    var body: some View {
        VStack(spacing: 20) {
            Spacer()
            switch job {
            case .testing:
                ProgressView().controlSize(.large)
                Text("Testing the network…").font(.headline)
                Text("The Overhead is joining your Wi-Fi to check it. This takes up to 20 seconds.")
                    .foregroundStyle(.secondary)
            case .reconnecting:
                ProgressView().controlSize(.large)
                Text("Reconnecting…").font(.headline)
                Text("The connection dropped while the Overhead tested the network. If your iPhone asks, enter the new code from its display.")
                    .foregroundStyle(.secondary)
            case let .finished(r):
                Image(systemName: r == .saved ? "checkmark.circle.fill" : "xmark.circle.fill")
                    .font(.system(size: 56))
                    .foregroundStyle(r == .saved ? .green : .red)
                Text(r.message)
                Button(r == .saved ? "Done" : "Back", action: onDone).buttonStyle(.borderedProminent)
            case .vanished:
                Image(systemName: "checkmark.circle")
                    .font(.system(size: 56))
                Text("The Overhead left setup, which it does after saving. If its display shows the radar, you're done.")
                Button("Done", action: onDone).buttonStyle(.borderedProminent)
            case let .failed(why):
                Image(systemName: "exclamationmark.triangle").font(.system(size: 56))
                Text(why)
                Button("Back", action: onDone).buttonStyle(.borderedProminent)
            }
            Spacer()
        }
        .multilineTextAlignment(.center)
        .padding(32)
        .presentationDetents([.medium])
    }
}

/// Read-only device status (0087).
struct StatusSection: View {
    let info: Info

    var body: some View {
        Section("Status") {
            LabeledContent("Version", value: "\(info.version) (build \(info.build))")
            LabeledContent("Channel", value: info.channel.label)
            LabeledContent("In setup because", value: info.reason.label)
            LabeledContent("Wi-Fi", value: info.wifiStored ? (info.ssid.isEmpty ? "Saved" : info.ssid) : "Not saved")
            LabeledContent("Location", value: info.siteStored ? "Saved" : "Not saved")
            LabeledContent("Last data fetch", value: lastFetch)
            if info.failedBuild != 0 {
                LabeledContent("Last failed update", value: "Build \(info.failedBuild)")
            }
            if let r = info.lastResult {
                LabeledContent("Last attempt", value: r == .saved ? "Saved" : "\(r)")
            }
            LabeledContent("MAC", value: info.macString)
        }
    }

    private var lastFetch: String {
        guard info.lastFetchUnixS > 0 else { return "Never" }
        let d = Date(timeIntervalSince1970: TimeInterval(info.lastFetchUnixS))
        return d.formatted(.relative(presentation: .named))
    }
}
