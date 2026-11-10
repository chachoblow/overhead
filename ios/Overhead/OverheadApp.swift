import OverheadSetupKit
import SwiftUI

@main
struct OverheadApp: App {
    @State private var central = SetupCentral()
    @State private var location = LocationProvider()

    var body: some Scene {
        WindowGroup {
            DeviceListView()
                .environment(central)
                .environment(location)
        }
    }
}
