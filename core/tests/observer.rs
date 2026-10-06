//! Independent pymap3d/Skyfield references and analytic topocentric geometry.
//! Provenance, model differences, and gates: fixtures/README.md.

use core::f64::consts::{FRAC_PI_2, PI, TAU};
use overhead_core::{
    CoordinateError, EcefPosition, GeodeticPosition, LookAngles, ObservationError, Satellite,
    ecef_to_look_angles,
};
use serde_json::Value;
use sgp4::chrono::NaiveDateTime;

fn site(lat: f64, lon: f64, height: f64) -> GeodeticPosition {
    GeodeticPosition {
        latitude_rad: lat.to_radians(),
        longitude_rad: lon.to_radians(),
        altitude_km: height,
    }
}

fn fixture_site(value: &Value) -> GeodeticPosition {
    site(
        value["latitude_deg"].as_f64().unwrap(),
        value["longitude_deg"].as_f64().unwrap(),
        value["altitude_km"].as_f64().unwrap(),
    )
}

fn references() -> Value {
    serde_json::from_str(include_str!("fixtures/observer.json")).unwrap()
}

fn angle_error(a: f64, b: f64) -> f64 {
    ((a - b + PI).rem_euclid(TAU) - PI).abs()
}

fn assert_angles(actual: LookAngles, range: f64, az: Option<f64>, el: f64) {
    assert!((actual.range_km - range).abs() < 1e-9, "{actual:?}");
    assert!((actual.elevation_rad - el).abs() < 1e-12, "{actual:?}");
    match (actual.azimuth_rad, az) {
        (Some(a), Some(b)) => {
            assert!((0.0..TAU).contains(&a), "{actual:?}");
            assert!(angle_error(a, b) < 1e-12, "{actual:?}, expected {b}");
        }
        (None, None) => {}
        _ => panic!("azimuth: {actual:?}, expected {az:?}"),
    }
}

fn offset(observer: GeodeticPosition, delta: [f64; 3]) -> EcefPosition {
    let origin = observer.to_ecef().unwrap().km();
    EcefPosition::from_km(core::array::from_fn(|i| origin[i] + delta[i]))
}

#[test]
fn isolated_geometry_matches_pymap3d() {
    let references = references();
    let cases = references["geometry"].as_array().unwrap();
    assert_eq!(cases.len(), 27);
    for case in cases {
        let target = EcefPosition::from_km(core::array::from_fn(|i| {
            case["ecef_km"][i].as_f64().unwrap()
        }));
        let actual = ecef_to_look_angles(target, fixture_site(&case["observer"])).unwrap();
        assert!(
            (actual.range_km - case["range_km"].as_f64().unwrap()).abs() < 1e-8,
            "{case}: {actual:?}"
        );
        assert!(
            angle_error(
                actual.azimuth_rad.unwrap(),
                case["azimuth_deg"].as_f64().unwrap().to_radians()
            ) < 1e-9_f64.to_radians(),
            "{case}: {actual:?}"
        );
        assert!(
            (actual.elevation_rad.to_degrees() - case["elevation_deg"].as_f64().unwrap()).abs()
                < 1e-9,
            "{case}: {actual:?}"
        );
    }
}

#[test]
fn fixed_iss_pipeline_matches_skyfield() {
    let elements: Vec<sgp4::Elements> =
        serde_json::from_str(include_str!("fixtures/iss-25544.json")).unwrap();
    let satellite = Satellite::from_elements(&elements[0]).unwrap();
    let references = references();
    let cases = references["iss_pipeline"].as_array().unwrap();
    assert_eq!(cases.len(), 12);
    let mut max_error = [0.0_f64; 3];
    let mut above = 0;
    for case in cases {
        let time =
            NaiveDateTime::parse_from_str(case["utc"].as_str().unwrap(), "%Y-%m-%dT%H:%M:%S%.f")
                .unwrap();
        let state = satellite.state_at(time).unwrap();
        let ecef = state.to_ecef().unwrap();
        let actual = ecef_to_look_angles(ecef, fixture_site(&case["observer"])).unwrap();
        let errors = [
            (actual.range_km - case["range_km"].as_f64().unwrap()).abs(),
            angle_error(
                actual.azimuth_rad.unwrap(),
                case["azimuth_deg"].as_f64().unwrap().to_radians(),
            )
            .to_degrees(),
            (actual.elevation_rad.to_degrees() - case["elevation_deg"].as_f64().unwrap()).abs(),
        ];
        for i in 0..3 {
            max_error[i] = max_error[i].max(errors[i]);
        }
        assert!(errors[0] < 0.1, "range: {case}: {actual:?}");
        assert!(errors[1] < 0.01, "azimuth: {case}: {actual:?}");
        assert!(errors[2] < 0.01, "elevation: {case}: {actual:?}");
        above += usize::from(actual.elevation_rad > 0.0);
    }
    assert!(above > 0 && above < cases.len());
    eprintln!("Skyfield max errors [km, az deg, el deg]: {max_error:?}");
}

#[test]
fn equatorial_cardinals_horizon_zenith_nadir_and_below_horizon() {
    // At Greenwich/equator ECEF X is up, Y is east, Z is north.
    let observer = site(0.0, 0.0, 0.0);
    for (delta, az, el, range) in [
        ([0.0, 0.0, 1000.0], Some(0.0), 0.0, 1000.0),
        ([0.0, 1000.0, 0.0], Some(FRAC_PI_2), 0.0, 1000.0),
        ([0.0, 0.0, -1000.0], Some(PI), 0.0, 1000.0),
        ([0.0, -1000.0, 0.0], Some(3.0 * FRAC_PI_2), 0.0, 1000.0),
        ([1000.0, 0.0, 0.0], None, FRAC_PI_2, 1000.0),
        ([-1000.0, 0.0, 0.0], None, -FRAC_PI_2, 1000.0),
        ([300.0, 400.0, 0.0], Some(FRAC_PI_2), 0.6_f64.asin(), 500.0),
        ([-300.0, 0.0, 400.0], Some(0.0), -0.6_f64.asin(), 500.0),
    ] {
        let actual = ecef_to_look_angles(offset(observer, delta), observer).unwrap();
        assert_angles(actual, range, az, el);
    }
}

#[test]
fn azimuth_quadrants_and_north_wrap_are_canonical() {
    let observer = site(0.0, 0.0, 0.0);
    for degrees in [0.0_f64, 45.0, 135.0, 225.0, 315.0, 359.999999] {
        let az = degrees.to_radians();
        let target = offset(observer, [0.0, 1000.0 * az.sin(), 1000.0 * az.cos()]);
        assert_angles(
            ecef_to_look_angles(target, observer).unwrap(),
            1000.0,
            Some(az),
            0.0,
        );
    }
    for east in [-0.0, -1e-100, 1e-100] {
        let actual = ecef_to_look_angles(offset(observer, [0.0, east, 1000.0]), observer).unwrap();
        let az = actual.azimuth_rad.unwrap();
        assert!(!az.is_sign_negative());
        assert!((0.0..TAU).contains(&az));
        assert!(angle_error(az, 0.0) < 1e-12);
    }
}

#[test]
fn poles_use_supplied_longitude_for_the_local_basis() {
    for lat in [-90.0_f64, 90.0] {
        for (lon, north, east) in [
            (0.0, [-lat.signum() * 1000.0, 0.0, 0.0], [0.0, 1000.0, 0.0]),
            (
                90.0,
                [0.0, -lat.signum() * 1000.0, 0.0],
                [-1000.0, 0.0, 0.0],
            ),
        ] {
            let observer = site(lat, lon, 0.5);
            for (delta, az) in [(north, 0.0), (east, FRAC_PI_2)] {
                assert_angles(
                    ecef_to_look_angles(offset(observer, delta), observer).unwrap(),
                    1000.0,
                    Some(az),
                    0.0,
                );
            }
            assert_angles(
                ecef_to_look_angles(offset(observer, [0.0, 0.0, lat.signum() * 500.0]), observer)
                    .unwrap(),
                500.0,
                None,
                FRAC_PI_2,
            );
        }
    }
}

#[test]
fn ellipsoid_normal_is_vertical_across_latitudes_and_heights() {
    for lat in [-90.0, -89.999999, -45.0, 0.0, 39.007, 89.999999, 90.0] {
        for lon in [-180.0, -104.883, 0.0, 123.0, 180.0] {
            for height in [-0.43, 0.0, 2.187] {
                let observer = site(lat, lon, height);
                for distance in [-10.0_f64, 400.0, 35786.0] {
                    let target = site(lat, lon, height + distance).to_ecef().unwrap();
                    assert_angles(
                        ecef_to_look_angles(target, observer).unwrap(),
                        distance.abs(),
                        None,
                        distance.signum() * FRAC_PI_2,
                    );
                }
            }
        }
    }
}

#[test]
fn antimeridian_endpoints_have_equivalent_look_angles() {
    let target = EcefPosition::from_km([-7000.0, 2000.0, -3500.0]);
    let a = ecef_to_look_angles(target, site(30.0, -180.0, 0.4)).unwrap();
    let b = ecef_to_look_angles(target, site(30.0, 180.0, 0.4)).unwrap();
    assert_angles(a, b.range_km, b.azimuth_rad, b.elevation_rad);
}

#[test]
fn vertical_singularity_tolerance_does_not_hide_nearby_directions() {
    let observer = site(0.0, 0.0, 0.0);
    for up in [-1000.0, 1000.0] {
        for (east, undefined) in [(1e-10, true), (1e-8, false)] {
            let actual = ecef_to_look_angles(offset(observer, [up, east, 0.0]), observer).unwrap();
            assert_eq!(actual.azimuth_rad.is_none(), undefined);
            if let Some(az) = actual.azimuth_rad {
                assert_eq!(az, FRAC_PI_2);
            }
            assert!(actual.elevation_rad.abs() <= FRAC_PI_2);
            assert_eq!(actual.elevation_rad.signum(), up.signum());
        }
    }
}

#[test]
fn coincident_positions_are_errors_but_tiny_nonzero_ranges_are_not() {
    for observer in [site(39.007, -104.883, 2.187), site(90.0, 123.0, 0.0)] {
        assert_eq!(
            ecef_to_look_angles(observer.to_ecef().unwrap(), observer),
            Err(ObservationError::CoincidentPositions)
        );
    }
    let observer = site(0.0, 0.0, 0.0);
    let actual = ecef_to_look_angles(offset(observer, [0.0, 0.0, 1e-200]), observer).unwrap();
    assert_eq!(actual.range_km, 1e-200);
    assert_eq!(actual.azimuth_rad, Some(0.0));
    assert_eq!(actual.elevation_rad, 0.0);
}

#[test]
fn rejects_non_finite_target_and_observer_components() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for axis in 0..3 {
            let mut target = [7000.0, 100.0, 200.0];
            target[axis] = bad;
            assert_eq!(
                ecef_to_look_angles(EcefPosition::from_km(target), site(0.0, 0.0, 0.0)),
                Err(CoordinateError::NonFinite.into())
            );
            let mut fields = [0.0; 3];
            fields[axis] = bad;
            let observer = GeodeticPosition {
                latitude_rad: fields[0],
                longitude_rad: fields[1],
                altitude_km: fields[2],
            };
            assert_eq!(
                ecef_to_look_angles(EcefPosition::from_km([7000.0, 0.0, 0.0]), observer),
                Err(CoordinateError::NonFinite.into())
            );
        }
    }
}

#[test]
fn rejects_invalid_observer_angles_and_overflow() {
    for (observer, error) in [
        (site(90.001, 0.0, 0.0), CoordinateError::InvalidLatitude),
        (site(-90.001, 0.0, 0.0), CoordinateError::InvalidLatitude),
        (site(0.0, 180.001, 0.0), CoordinateError::InvalidLongitude),
        (site(0.0, -180.001, 0.0), CoordinateError::InvalidLongitude),
    ] {
        assert_eq!(
            ecef_to_look_angles(EcefPosition::from_km([7000.0, 0.0, 0.0]), observer),
            Err(error.into())
        );
    }
    // Norm overflow and subtraction overflow, respectively.
    for (target, observer) in [
        ([f64::MAX; 3], site(0.0, 0.0, 0.0)),
        ([f64::MAX, 0.0, 0.0], site(0.0, 0.0, -f64::MAX)),
    ] {
        assert_eq!(
            ecef_to_look_angles(EcefPosition::from_km(target), observer),
            Err(CoordinateError::NonFinite.into())
        );
    }
    // Large finite norms should not overflow just from squaring components.
    let actual = ecef_to_look_angles(
        EcefPosition::from_km([f64::MAX / 2.0; 3]),
        site(45.0, 45.0, 0.0),
    )
    .unwrap();
    assert!(actual.range_km.is_finite());
    assert!(actual.azimuth_rad.unwrap().is_finite());
    assert!(actual.elevation_rad.is_finite());
}
