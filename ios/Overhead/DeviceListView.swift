import OverheadSetupKit
import SwiftUI

/// Finds devices whose setup window is open.
struct DeviceListView: View {
    @Environment(SetupCentral.self) private var central
    @State private var selected: DiscoveredDevice?

    var body: some View {
        NavigationStack {
            List {
                switch central.power {
                case .off:
                    Label("Turn on Bluetooth to find your Overhead.", systemImage: "antenna.radiowaves.left.and.right.slash")
                case .unauthorized:
                    Label("Allow Bluetooth for Overhead in Settings.", systemImage: "lock")
                case .unsupported:
                    Label("This device has no Bluetooth LE.", systemImage: "xmark.octagon")
                case .unknown, .on:
                    EmptyView()
                }

                Section {
                    if central.devices.isEmpty && central.power == .on {
                        HStack(spacing: 12) {
                            ProgressView()
                            Text("Looking for an Overhead…")
                                .foregroundStyle(.secondary)
                        }
                    }
                    ForEach(central.devices) { device in
                        Button {
                            selected = device
                        } label: {
                            HStack {
                                Image(systemName: "dot.radiowaves.left.and.right")
                                Text(device.name)
                                    .foregroundStyle(.primary)
                                Spacer()
                                SignalBars(rssi: device.rssi)
                            }
                        }
                    }
                } header: {
                    Text("Nearby")
                } footer: {
                    Text("""
                    An Overhead offers setup when it has no Wi-Fi saved, when it can't join its network, \
                    or when you hold the knob down while plugging it in until the display says to let go. \
                    Setup closes after 10 minutes without a phone.
                    """)
                }
            }
            .navigationTitle("Overhead")
            .navigationDestination(item: $selected) { device in
                SetupView(device: device)
            }
        }
        .onAppear { central.startScan() }
        .onChange(of: selected) { _, now in
            if now == nil { central.startScan() } else { central.stopScan() }
        }
    }
}

struct SignalBars: View {
    let rssi: Int

    var body: some View {
        let bars = rssi >= -60 ? 3 : rssi >= -75 ? 2 : 1
        Image(systemName: "cellularbars", variableValue: Double(bars) / 3)
            .foregroundStyle(.secondary)
            .accessibilityLabel("\(rssi) dBm")
    }
}
