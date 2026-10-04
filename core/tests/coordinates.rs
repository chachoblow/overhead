//! Independent ERFA/pymap3d fixtures plus analytic geometry and edge cases.
//! Provenance, generation commands, conventions, and gates: fixtures/README.md.

use core::f64::consts::{FRAC_PI_2, PI};
use overhead_core::{CoordinateError, GeodeticPosition, Satellite, ecef_to_geodetic, teme_to_ecef};
use serde_json::Value;
use sgp4::chrono::{NaiveDate, NaiveDateTime, TimeDelta};

const KM_TOL: f64 = 1e-3; // 1 metre
const DEG_TOL: f64 = 1e-6;

fn utc(text: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S%.f").unwrap()
}

fn references() -> Value {
    serde_json::from_str(include_str!("fixtures/coordinates.json")).unwrap()
}

fn vector(value: &Value) -> [f64; 3] {
    core::array::from_fn(|i| value[i].as_f64().unwrap())
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2])
}

fn assert_position(actual: [f64; 3], expected: [f64; 3]) {
    let error = distance(actual, expected);
    assert!(
        error < KM_TOL,
        "error {error} km: actual {actual:?}, expected {expected:?}"
    );
}

fn geodetic(lat: f64, lon: f64, height: f64) -> GeodeticPosition {
    GeodeticPosition {
        latitude_rad: lat.to_radians(),
        longitude_rad: lon.to_radians(),
        altitude_km: height,
    }
}

fn assert_geodetic(actual: GeodeticPosition, expected: GeodeticPosition) {
    let lat_error = (actual.latitude_rad - expected.latitude_rad)
        .to_degrees()
        .abs();
    assert!(lat_error < DEG_TOL, "latitude: {actual:?} != {expected:?}");
    assert!(
        (actual.altitude_km - expected.altitude_km).abs() < KM_TOL,
        "altitude: {actual:?} != {expected:?}"
    );
    // Longitude is arbitrary at a pole; near-pole cases still check it.
    if expected.latitude_rad.abs() != FRAC_PI_2 {
        let lon_error = ((actual.longitude_rad - expected.longitude_rad).to_degrees() + 180.0)
            .rem_euclid(360.0)
            - 180.0;
        assert!(
            lon_error.abs() < DEG_TOL,
            "longitude: {actual:?} != {expected:?}"
        );
    }
    assert!((-FRAC_PI_2..=FRAC_PI_2).contains(&actual.latitude_rad));
    assert!((-PI..PI).contains(&actual.longitude_rad));
}

#[test]
fn teme_rotation_matches_erfa_at_independent_timestamps() {
    for case in references()["rotations"].as_array().unwrap() {
        let teme = vector(&case["teme_km"]);
        let ecef = teme_to_ecef(teme, utc(case["utc"].as_str().unwrap())).unwrap();
        assert_position(ecef, vector(&case["ecef_km"]));
        // A pure Z rotation preserves radius and the polar component.
        assert!((distance(ecef, [0.0; 3]) - distance(teme, [0.0; 3])).abs() < 1e-9);
        assert_eq!(ecef[2], teme[2]);
    }
}

#[test]
fn geodetic_conversions_match_independent_pymap3d_positions() {
    for case in references()["geodetic"].as_array().unwrap() {
        let expected = geodetic(
            case["latitude_deg"].as_f64().unwrap(),
            case["longitude_deg"].as_f64().unwrap(),
            case["altitude_km"].as_f64().unwrap(),
        );
        let ecef = vector(&case["ecef_km"]);
        assert_geodetic(ecef_to_geodetic(ecef).unwrap(), expected);
        assert_position(expected.to_ecef().unwrap(), ecef);
    }
}

#[test]
fn grid_round_trips_surface_leo_geo_and_high_orbits() {
    for lat in [
        -90.0, -89.999999, -60.0, -45.0, -1e-8, 0.0, 1e-8, 45.0, 60.0, 89.999999, 90.0,
    ] {
        for lon in [
            -180.0,
            -179.999999,
            -90.0,
            -1e-8,
            0.0,
            1e-8,
            90.0,
            179.999999,
            180.0,
        ] {
            for height in [-0.43, 0.0, 2.187, 420.0, 35786.0, 100000.0] {
                let original = geodetic(lat, lon, height);
                let ecef = original.to_ecef().unwrap();
                let restored = ecef_to_geodetic(ecef).unwrap();
                assert_geodetic(restored, original);
                assert_position(restored.to_ecef().unwrap(), ecef);
            }
        }
    }
}

#[test]
fn axes_use_wgs84_and_defined_pole_and_antimeridian_conventions() {
    let a = 6378.137;
    let b = 6_356.752_314_245_179;
    for (ecef, expected) in [
        ([a, 0.0, 0.0], geodetic(0.0, 0.0, 0.0)),
        ([0.0, a, 0.0], geodetic(0.0, 90.0, 0.0)),
        ([0.0, -a, 0.0], geodetic(0.0, -90.0, 0.0)),
        ([-a, 0.0, 0.0], geodetic(0.0, -180.0, 0.0)),
        ([-a, -0.0, 0.0], geodetic(0.0, -180.0, 0.0)),
        ([0.0, 0.0, b + 420.0], geodetic(90.0, 0.0, 420.0)),
        ([0.0, 0.0, -b - 420.0], geodetic(-90.0, 0.0, 420.0)),
    ] {
        let result = ecef_to_geodetic(ecef).unwrap();
        assert_geodetic(result, expected);
        assert_eq!(result.longitude_rad, expected.longitude_rad);
    }
    // Any input longitude at an exact pole produces the exact polar axis.
    for lat in [-90.0, 90.0] {
        let ecef = geodetic(lat, 123.0, 0.0).to_ecef().unwrap();
        assert_eq!(&ecef[..2], &[0.0, 0.0]);
        assert_eq!(ecef_to_geodetic(ecef).unwrap().longitude_rad, 0.0);
    }
}

#[test]
fn subsecond_time_is_not_truncated() {
    let t0 = utc("2026-10-04T12:43:41.000000");
    let t1 = t0 + TimeDelta::milliseconds(500);
    let r0 = teme_to_ecef([7000.0, 0.0, 0.0], t0).unwrap();
    let r1 = teme_to_ecef([7000.0, 0.0, 0.0], t1).unwrap();
    let movement = distance(r0, r1);
    assert!(
        (0.25..0.26).contains(&movement),
        "half-second movement {movement} km"
    );
}

#[test]
fn rotation_is_continuous_across_calendar_boundaries() {
    for text in [
        "2000-02-28T23:59:59",
        "2000-02-29T23:59:59",
        "2100-02-28T23:59:59",
        "2026-12-31T23:59:59",
    ] {
        let t = utc(text);
        let r0 = teme_to_ecef([7000.0, 0.0, 0.0], t).unwrap();
        let r1 = teme_to_ecef([7000.0, 0.0, 0.0], t + TimeDelta::seconds(1)).unwrap();
        assert!((0.510..0.511).contains(&distance(r0, r1)));
    }
}

#[test]
fn propagated_vanguard_positions_compose_with_reference_rotations() {
    let elements = sgp4::Elements::from_tle(
        None,
        b"1 00005U 58002B   00179.78495062  .00000023  00000-0  28098-4 0  4753",
        b"2 00005  34.2682 348.7242 1859667 331.7664  19.3264 10.82419157413667",
    )
    .unwrap();
    let satellite = Satellite::from_elements(&elements).unwrap();
    let mut checked = 0;
    for case in references()["rotations"].as_array().unwrap() {
        if !case["name"].as_str().unwrap().starts_with("vanguard_") {
            continue;
        }
        let time = utc(case["utc"].as_str().unwrap());
        let state = satellite.state_at(time).unwrap();
        let ecef = teme_to_ecef(state.position, time).unwrap();
        assert_position(ecef, vector(&case["ecef_km"]));
        let geodetic = ecef_to_geodetic(ecef).unwrap();
        assert_position(geodetic.to_ecef().unwrap(), ecef);
        checked += 1;
    }
    assert_eq!(checked, 2);
}

#[test]
fn iss_fixture_composes_at_fixed_timestamps() {
    let sets: Vec<sgp4::Elements> =
        serde_json::from_str(include_str!("fixtures/iss-25544.json")).unwrap();
    let satellite = Satellite::from_elements(&sets[0]).unwrap();
    for minutes in [0, 90, 1440] {
        let time = satellite.epoch() + TimeDelta::minutes(minutes);
        let state = satellite.state_at(time).unwrap();
        let ecef = teme_to_ecef(state.position, time).unwrap();
        let geodetic = ecef_to_geodetic(ecef).unwrap();
        assert!((350.0..500.0).contains(&geodetic.altitude_km));
        assert!(geodetic.latitude_rad.to_degrees().abs() < 52.0);
        assert_position(geodetic.to_ecef().unwrap(), ecef);
    }
}

#[test]
fn rejects_non_finite_inputs_in_every_component() {
    let time = utc("2000-01-01T12:00:00");
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for axis in 0..3 {
            let mut position = [7000.0, 100.0, 200.0];
            position[axis] = bad;
            assert_eq!(
                teme_to_ecef(position, time),
                Err(CoordinateError::NonFinite)
            );
            assert_eq!(ecef_to_geodetic(position), Err(CoordinateError::NonFinite));
            let mut fields = [0.0, 0.0, 0.0];
            fields[axis] = bad;
            let geodetic = GeodeticPosition {
                latitude_rad: fields[0],
                longitude_rad: fields[1],
                altitude_km: fields[2],
            };
            assert_eq!(geodetic.to_ecef(), Err(CoordinateError::NonFinite));
        }
    }
}

#[test]
fn rejects_invalid_angles_center_and_numeric_overflow() {
    for lat in [-90.001, 90.001] {
        assert_eq!(
            geodetic(lat, 0.0, 0.0).to_ecef(),
            Err(CoordinateError::InvalidLatitude)
        );
    }
    for lon in [-180.001, 180.001] {
        assert_eq!(
            geodetic(0.0, lon, 0.0).to_ecef(),
            Err(CoordinateError::InvalidLongitude)
        );
    }
    assert_eq!(
        ecef_to_geodetic([0.0; 3]),
        Err(CoordinateError::EarthCenter)
    );
    assert_eq!(
        ecef_to_geodetic([f64::MAX; 3]),
        Err(CoordinateError::NonFinite)
    );
    assert_eq!(
        teme_to_ecef([f64::MAX; 3], utc("2000-01-01T12:00:00")),
        Err(CoordinateError::NonFinite)
    );
}

#[test]
fn difficult_deep_interior_input_returns_error_instead_of_unbounded_iteration() {
    assert_eq!(
        ecef_to_geodetic([43.0, 0.0, 1.0]),
        Err(CoordinateError::NoConvergence)
    );
}

#[test]
fn unsupported_times_and_explicit_leap_seconds_are_rejected() {
    let leap_second = NaiveDate::from_ymd_opt(2016, 12, 31)
        .unwrap()
        .and_hms_nano_opt(23, 59, 59, 1_000_000_000)
        .unwrap();
    for time in [
        utc("1956-12-31T23:59:59"),
        utc("2101-01-01T00:00:00"),
        leap_second,
    ] {
        assert_eq!(
            teme_to_ecef([7000.0, 0.0, 0.0], time),
            Err(CoordinateError::UnsupportedTime)
        );
    }
}
