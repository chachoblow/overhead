//! Propagation verification against independent reference vectors.
//!
//! Reference data: the SGP4 verification set from Vallado et al.,
//! "Revisiting Spacetrack Report #3" (AIAA 2006-6753), as distributed with
//! the reference implementation (SGP4-VER.TLE and expected ephemerides) and
//! checked into the sgp4 crate as tests/test_cases.toml. Positions are TEME,
//! km; velocities km/s; times are minutes since the element epoch.
//!
//! Tolerances per docs/decisions/0006-sgp4-crate-and-conventions.md:
//! 1e-6 km in position, 1e-9 km/s in velocity, at every time step.

use overhead_core::{PropagateError, Satellite};
use sgp4::chrono::TimeDelta;

/// A reference state: (minutes since epoch, position km, velocity km/s).
type RefState = (f64, [f64; 3], [f64; 3]);

fn satellite_from_tle(line1: &str, line2: &str) -> Satellite {
    let elements = sgp4::Elements::from_tle(None, line1.as_bytes(), line2.as_bytes())
        .expect("reference TLE must parse");
    Satellite::from_elements(&elements).expect("reference TLE must validate")
}

/// Propagates to epoch + minutes via the explicit-timestamp API and checks
/// the state against the reference vector.
fn assert_matches_reference(satellite: &Satellite, states: &[RefState]) {
    for (minutes, position, velocity) in states {
        // Whole minutes: exact in both f64 and datetime arithmetic.
        let datetime = satellite.epoch() + TimeDelta::minutes(*minutes as i64);
        let state = satellite
            .state_at(datetime)
            .unwrap_or_else(|error| panic!("propagation to t={minutes} min failed: {error}"));
        assert_eq!(state.minutes_since_epoch(), *minutes);
        assert_eq!(state.datetime(), datetime);
        for axis in 0..3 {
            assert!(
                (state.position().km()[axis] - position[axis]).abs() < 1.0e-6,
                "t={minutes} min: position[{axis}] = {}, reference = {}",
                state.position().km()[axis],
                position[axis],
            );
            assert!(
                (state.velocity()[axis] - velocity[axis]).abs() < 1.0e-9,
                "t={minutes} min: velocity[{axis}] = {}, reference = {}",
                state.velocity()[axis],
                velocity[axis],
            );
        }
    }
}

/// Satellite 00005 (Vanguard 1, "TEME example"): near-Earth SGP4 path.
#[test]
fn near_earth_matches_vallado_vectors() {
    let satellite = satellite_from_tle(
        "1 00005U 58002B   00179.78495062  .00000023  00000-0  28098-4 0  4753",
        "2 00005  34.2682 348.7242 1859667 331.7664  19.3264 10.82419157413667",
    );
    assert_eq!(satellite.norad_id(), 5);
    #[rustfmt::skip]
    let states: &[RefState] = &[
        (0.0, [7022.46529266, -1400.08296755, 0.03995155], [1.893841015, 6.405893759, 4.534807250]),
        (360.0, [-7154.03120202, -3783.17682504, -3536.19412294], [4.741887409, -4.151817765, -2.093935425]),
        (720.0, [-7134.59340119, 6531.68641334, 3260.27186483], [-4.113793027, -2.911922039, -2.557327851]),
        (1080.0, [5568.53901181, 4492.06992591, 3863.87641983], [-4.209106476, 5.159719888, 2.744852980]),
        (1440.0, [-938.55923943, -6268.18748831, -4294.02924751], [7.536105209, -0.427127707, 0.989878080]),
        (1800.0, [-9680.56121728, 2802.47771354, 124.10688038], [-0.905874102, -4.659467970, -3.227347517]),
        (2160.0, [190.19796988, 7746.96653614, 5110.00675412], [-6.112325142, 1.527008184, -0.139152358]),
        (2520.0, [5579.55640116, -3995.61396789, -1518.82108966], [4.767927483, 5.123185301, 4.276837355]),
        (2880.0, [-8650.73082219, -1914.93811525, -3007.03603443], [3.067165127, -4.828384068, -2.515322836]),
        (3240.0, [-5429.79204164, 7574.36493792, 3747.39305236], [-4.999442110, -1.800561422, -2.229392830]),
        (3600.0, [6759.04583722, 2001.58198220, 2783.55192533], [-2.180993947, 6.402085603, 3.644723952]),
        (3960.0, [-3791.44531559, -5712.95617894, -4533.48630714], [6.668817493, -2.516382327, -0.082384354]),
        (4320.0, [-9060.47373569, 4658.70952502, 813.68673153], [-2.232832783, -4.110453490, -3.157345433]),
    ];
    assert_matches_reference(&satellite, states);
}

/// Satellite 11801 (original STR#3 SDP4 test): deep-space path, 12 h orbit,
/// high eccentricity.
#[test]
fn deep_space_matches_vallado_vectors() {
    let satellite = satellite_from_tle(
        "1 11801U          80230.29629788  .01431103  00000-0  14311-1 0    13",
        "2 11801  46.7916 230.4354 7318036  47.4722  10.4117  2.28537848    13",
    );
    #[rustfmt::skip]
    let states: &[RefState] = &[
        (0.0, [7473.37102491, 428.94748312, 5828.74846783], [5.107155391, 6.444680305, -0.186133297]),
        (360.0, [-3305.22148694, 32410.84323331, -24697.16974954], [-1.301137319, -1.151315600, -0.283335823]),
        (720.0, [14271.29083858, 24110.44309009, -4725.76320143], [-0.320504528, 2.679841539, -2.084054355]),
        (1080.0, [-9990.05800009, 22717.34212448, -23616.88515553], [-1.016674392, -2.290267981, 0.728923337]),
        (1440.0, [9787.87836256, 33753.32249667, -15030.79874625], [-1.094251553, 0.923589906, -1.522311008]),
    ];
    assert_matches_reference(&satellite, states);
}

/// Satellite 33333 (verification set): propagation succeeds at t=20 min but
/// must fail with a negative semi-latus rectum at t=25 min.
#[test]
fn reports_divergence_after_epoch() {
    let satellite = satellite_from_tle(
        "1 33333U 05037B   05333.02012661  .25992681  00000-0  24476-3 0  1532",
        "2 33333  96.4736 157.9986 9950000 244.0492 110.6523  4.00004038 10700",
    );
    assert_matches_reference(
        &satellite,
        &[(
            20.0,
            [23876.96955477, -37275.65263893, -8113.95104473],
            [0.589108130, -0.767768418, -0.260379679],
        )],
    );
    let at_25 = satellite.state_at(satellite.epoch() + TimeDelta::minutes(25));
    assert!(
        matches!(
            at_25,
            Err(PropagateError::Sgp4(
                sgp4::Error::NegativeSemiLatusRectum { t }
            )) if t == 25.0
        ),
        "expected negative semi-latus rectum at t=25, got {at_25:?}",
    );
}

/// Satellite 33334 (verification set): propagation must fail at the epoch
/// itself with a diverging perturbed eccentricity.
#[test]
fn reports_divergence_at_epoch() {
    let satellite = satellite_from_tle(
        "1 33334U 78066F   06174.85818871  .00000620  00000-0  10000-3 0  6806",
        "2 33334  68.4714 236.1303 5602877 123.7484 302.5767  0.00001000 67521",
    );
    let at_epoch = satellite.state_at(satellite.epoch());
    assert!(
        matches!(
            at_epoch,
            Err(PropagateError::Sgp4(
                sgp4::Error::OutOfRangePerturbedEccentricity { t, .. }
            )) if t == 0.0
        ),
        "expected diverging perturbed eccentricity at epoch, got {at_epoch:?}",
    );
}

/// Finite but corrupt inputs can overflow inside SGP4 without an upstream error.
#[test]
fn non_finite_predictions_are_errors() {
    let sets: Vec<sgp4::Elements> =
        serde_json::from_str(include_str!("fixtures/iss-25544.json")).unwrap();
    for field in ["mean_motion", "drag_term"] {
        let mut elements = sets[0].clone();
        match field {
            "mean_motion" => elements.mean_motion = 1e300,
            _ => elements.drag_term = 1e300,
        }
        let satellite = Satellite::from_elements(&elements).unwrap();
        let result = satellite.state_at(satellite.epoch());
        assert_eq!(result, Err(PropagateError::NonFinite), "{field}");
    }
}

/// The checked-in ISS fixture must propagate to physically plausible LEO
/// states at the fixed test timestamps (see tests/fixtures/README.md).
#[test]
fn iss_fixture_propagates_to_plausible_leo_states() {
    let mut sets: Vec<sgp4::Elements> =
        serde_json::from_str(include_str!("fixtures/iss-25544.json")).unwrap();
    let satellite = Satellite::from_elements(&sets.pop().unwrap()).unwrap();
    for minutes in [0, 90, 1440] {
        let state = satellite
            .state_at(satellite.epoch() + TimeDelta::minutes(minutes))
            .expect("ISS fixture must propagate");
        let radius = state
            .position()
            .km()
            .iter()
            .map(|x| x * x)
            .sum::<f64>()
            .sqrt();
        let speed = state.velocity().iter().map(|x| x * x).sum::<f64>().sqrt();
        // ISS orbit: ~420 km altitude, ~7.66 km/s.
        assert!(
            (6700.0..6900.0).contains(&radius),
            "t={minutes} min: geocentric radius {radius} km is not ISS-like",
        );
        assert!(
            (7.4..7.9).contains(&speed),
            "t={minutes} min: speed {speed} km/s is not ISS-like",
        );
    }
}
