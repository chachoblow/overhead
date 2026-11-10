import CoreLocation
import Observation

/// One-shot location from the phone for the device's site (0087).
@MainActor
@Observable
final class LocationProvider: NSObject {
    enum State: Equatable {
        case idle
        case locating
        case found(CLLocation)
        case denied
        case failed(String)
    }

    private(set) var state: State = .idle
    @ObservationIgnored private let manager = CLLocationManager()

    override init() {
        super.init()
        manager.delegate = self
        manager.desiredAccuracy = kCLLocationAccuracyHundredMeters
    }

    func request() {
        switch manager.authorizationStatus {
        case .notDetermined:
            state = .locating
            manager.requestWhenInUseAuthorization()
        case .denied, .restricted:
            state = .denied
        default:
            state = .locating
            manager.requestLocation()
        }
    }

    /// Latitude, longitude and height above the WGS-84 ellipsoid (what the
    /// device's site wants; 0 when the phone has no vertical fix).
    var site: (lat: Double, lon: Double, alt: Double)? {
        guard case let .found(l) = state else { return nil }
        let alt = l.verticalAccuracy >= 0 ? l.ellipsoidalAltitude : 0
        return (l.coordinate.latitude, l.coordinate.longitude, alt)
    }
}

extension LocationProvider: CLLocationManagerDelegate {
    nonisolated func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
        let status = manager.authorizationStatus
        MainActor.assumeIsolated {
            guard state == .locating || state == .denied else { return }
            switch status {
            case .authorizedWhenInUse, .authorizedAlways:
                state = .locating
                manager.requestLocation()
            case .denied, .restricted:
                state = .denied
            default:
                break
            }
        }
    }

    nonisolated func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        guard let l = locations.last else { return }
        MainActor.assumeIsolated { state = .found(l) }
    }

    nonisolated func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {
        let why = error.localizedDescription
        MainActor.assumeIsolated {
            if case .found = state { return }
            state = .failed(why)
        }
    }
}
