//! Identical allocation-free host/target reference selection and strict gates.
//! Preparation only: no UART protocol, timers, stack painting, or device execution.
//! Fixture provenance/selection: tools/fixtures/m2-numerical/README.md.
use core::f64::consts::{FRAC_PI_2, PI, TAU};
use overhead_core::{
    EcefPosition, GeodeticPosition, LookAngles, Satellite, TemePosition, ecef_to_geodetic,
    ecef_to_look_angles,
    sgp4::chrono::{NaiveDateTime, TimeDelta},
    teme_to_ecef,
};

struct StateReference {
    minutes: i64,
    position: [f64; 3],
    velocity: [f64; 3],
}
struct RotationReference {
    time: NaiveDateTime,
    teme: [f64; 3],
    ecef: [f64; 3],
}
struct GeodeticReference {
    site: [f64; 3],
    ecef: [f64; 3],
}
struct ObserverReference {
    site: [f64; 3],
    target: [f64; 3],
    time: Option<NaiveDateTime>,
    range: f64,
    azimuth_deg: f64,
    elevation_deg: f64,
}
include!(concat!(env!("OUT_DIR"), "/numerical.rs"));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Teme,
    Rotation,
    Geodetic,
    Geometry,
    IssPipeline,
    VanguardPipeline,
}
/// A row, not a count of scalar comparisons. Order is fixed by SELECTION.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Check {
    pub family: Family,
    pub index: usize,
}
pub const SELECTION: [(Family, usize); 6] = [
    (Family::Teme, 12),
    (Family::Rotation, 9),
    (Family::Geodetic, 13),
    (Family::Geometry, 27),
    (Family::IssPipeline, 12),
    (Family::VanguardPipeline, 2),
];
pub const CHECK_COUNT: usize = 75;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Failure {
    Operation(&'static str),
    /// All tolerances are strict. Norm comparisons use squared kilometres.
    Tolerance {
        metric: &'static str,
        error: f64,
        limit: f64,
    },
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub checked: usize,
    pub failed: usize,
}

fn below(metric: &'static str, error: f64, limit: f64) -> Result<(), Failure> {
    if error.is_finite() && error >= 0.0 && error < limit {
        Ok(())
    } else {
        Err(Failure::Tolerance {
            metric,
            error,
            limit,
        })
    }
}
fn require(ok: bool, reason: &'static str) -> Result<(), Failure> {
    if ok {
        Ok(())
    } else {
        Err(Failure::Operation(reason))
    }
}
fn position(actual: [f64; 3], expected: [f64; 3]) -> Result<(), Failure> {
    // Equivalent strict Euclidean <1m gate for these bounded finite fixtures;
    // avoids adding a math dependency solely for the diagnostic norm.
    let d = core::array::from_fn::<_, 3, _>(|i| actual[i] - expected[i]);
    below(
        "position-norm-km2",
        d.iter().map(|x| x * x).sum(),
        COORDINATE_POSITION_NORM_KM * COORDINATE_POSITION_NORM_KM,
    )
}
fn site(v: [f64; 3]) -> GeodeticPosition {
    GeodeticPosition {
        latitude_rad: v[0].to_radians(),
        longitude_rad: v[1].to_radians(),
        altitude_km: v[2],
    }
}
fn wrapped_error(a: f64, b: f64, period: f64) -> f64 {
    // Match ((a - b + half).rem_euclid(period) - half).abs() in no_std.
    let remainder = (a - b + period / 2.0) % period;
    let positive = if remainder < 0.0 {
        remainder + period
    } else {
        remainder
    };
    (positive - period / 2.0).abs()
}
fn satellite(index: usize) -> Result<Satellite, Failure> {
    Satellite::from_elements(&reference_elements(index))
        .map_err(|_| Failure::Operation("initialize"))
}
fn geodetic(actual: GeodeticPosition, expected: GeodeticPosition) -> Result<(), Failure> {
    below(
        "latitude-deg",
        (actual.latitude_rad - expected.latitude_rad)
            .to_degrees()
            .abs(),
        GEODETIC_ANGLE_DEG,
    )?;
    below(
        "height-km",
        (actual.altitude_km - expected.altitude_km).abs(),
        GEODETIC_HEIGHT_KM,
    )?;
    if expected.latitude_rad.abs() != FRAC_PI_2 {
        below(
            "longitude-deg",
            wrapped_error(
                (actual.longitude_rad - expected.longitude_rad).to_degrees(),
                0.0,
                360.0,
            ),
            GEODETIC_ANGLE_DEG,
        )?;
    }
    require(
        (-FRAC_PI_2..=FRAC_PI_2).contains(&actual.latitude_rad),
        "latitude domain",
    )?;
    require(
        (-PI..PI).contains(&actual.longitude_rad),
        "longitude domain",
    )
}
fn observer(
    actual: LookAngles,
    expected: &ObserverReference,
    pipeline: bool,
) -> Result<(), Failure> {
    let (range_tol, angle_tol) = if pipeline {
        (PIPELINE_RANGE_KM, PIPELINE_ANGLE_DEG)
    } else {
        (GEOMETRY_RANGE_KM, GEOMETRY_ANGLE_DEG)
    };
    below(
        "range-km",
        (actual.range_km - expected.range).abs(),
        range_tol,
    )?;
    let az = actual
        .azimuth_rad
        .ok_or(Failure::Operation("unexpected undefined azimuth"))?;
    require((0.0..TAU).contains(&az), "azimuth domain")?;
    // Match the existing isolated test's radian comparison, avoiding an extra
    // degree conversion at its 1e-9-degree gate.
    let az_error = wrapped_error(az, expected.azimuth_deg.to_radians(), TAU);
    if pipeline {
        below("azimuth-deg", az_error.to_degrees(), angle_tol)?;
    } else {
        below("azimuth-rad", az_error, angle_tol.to_radians())?;
    }
    below(
        "elevation-deg",
        (actual.elevation_rad.to_degrees() - expected.elevation_deg).abs(),
        angle_tol,
    )
}
fn check(check: Check) -> Result<(), Failure> {
    let i = check.index;
    match check.family {
        Family::Teme => {
            let r = &TEME[i];
            let satellite = satellite(i / 3)?;
            let time = satellite.epoch() + TimeDelta::minutes(r.minutes);
            let state = satellite
                .state_at(time)
                .map_err(|_| Failure::Operation("TEME propagation"))?;
            require(
                state.datetime() == time && state.minutes_since_epoch() == r.minutes as f64,
                "TEME timestamp",
            )?;
            for axis in 0..3 {
                below(
                    "position-axis-km",
                    (state.position().km()[axis] - r.position[axis]).abs(),
                    TEME_POSITION_AXIS_KM,
                )?;
                below(
                    "velocity-axis-km/s",
                    (state.velocity()[axis] - r.velocity[axis]).abs(),
                    TEME_VELOCITY_AXIS_KM_S,
                )?;
            }
            Ok(())
        }
        Family::Rotation => {
            let r = rotation_reference(i);
            let actual = teme_to_ecef(TemePosition::from_km(r.teme), r.time)
                .map_err(|_| Failure::Operation("rotation"))?;
            position(actual.km(), r.ecef)
        }
        Family::Geodetic => {
            let r = geodetic_reference(i);
            let expected = site(r.site);
            geodetic(
                ecef_to_geodetic(EcefPosition::from_km(r.ecef))
                    .map_err(|_| Failure::Operation("inverse geodetic"))?,
                expected,
            )?;
            position(
                expected
                    .to_ecef()
                    .map_err(|_| Failure::Operation("forward geodetic"))?
                    .km(),
                r.ecef,
            )
        }
        Family::Geometry | Family::IssPipeline => {
            let pipeline = check.family == Family::IssPipeline;
            let r = if pipeline {
                pipeline_reference(i)
            } else {
                geometry_reference(i)
            };
            let target = if let Some(time) = r.time {
                satellite(4)?
                    .state_at(time)
                    .map_err(|_| Failure::Operation("ISS propagation"))?
                    .to_ecef()
                    .map_err(|_| Failure::Operation("ISS rotation"))?
            } else {
                EcefPosition::from_km(r.target)
            };
            let actual = ecef_to_look_angles(target, site(r.site))
                .map_err(|_| Failure::Operation("observer"))?;
            observer(actual, &r, pipeline)
        }
        Family::VanguardPipeline => {
            // The two existing composed coordinate checks: epoch and +360 min.
            let r = rotation_reference(3 + i);
            let actual = satellite(0)?
                .state_at(r.time)
                .map_err(|_| Failure::Operation("Vanguard propagation"))?
                .to_ecef()
                .map_err(|_| Failure::Operation("Vanguard rotation"))?;
            position(actual.km(), r.ecef)?;
            let restored = ecef_to_geodetic(actual)
                .map_err(|_| Failure::Operation("Vanguard inverse geodetic"))?
                .to_ecef()
                .map_err(|_| Failure::Operation("Vanguard forward geodetic"))?;
            position(restored.km(), actual.km())
        }
    }
}

/// Runs every selected row even after a failure; callers must reject any failed
/// or missing row. Callback/formatting is outside future measured workloads.
pub fn run(mut report: impl FnMut(Check, Result<(), Failure>)) -> Summary {
    let mut summary = Summary::default();
    for (family, count) in SELECTION {
        for index in 0..count {
            let id = Check { family, index };
            let result = check(id);
            summary.checked += 1;
            summary.failed += usize::from(result.is_err());
            report(id, result);
        }
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_shared_selection_passes() {
        let mut seen = 0;
        let summary = run(|id, result| {
            seen += 1;
            assert_eq!(result, Ok(()), "{id:?}");
        });
        assert_eq!(seen, CHECK_COUNT);
        assert_eq!(
            summary,
            Summary {
                checked: CHECK_COUNT,
                failed: 0
            }
        );
    }
    #[test]
    fn gates_are_strict_and_fail_closed() {
        for limit in [
            TEME_POSITION_AXIS_KM,
            TEME_VELOCITY_AXIS_KM_S,
            GEOMETRY_ANGLE_DEG,
            PIPELINE_RANGE_KM,
        ] {
            assert!(below("test", limit.next_down(), limit).is_ok());
            for error in [limit, limit.next_up(), f64::NAN, f64::INFINITY, -1.0] {
                assert!(below("test", error, limit).is_err());
            }
        }
        assert!(position([f64::NAN, 0.0, 0.0], [0.0; 3]).is_err());
        assert!(position([0.001, 0.0, 0.0], [0.0; 3]).is_err());
        assert!(position([0.0008, 0.0008, 0.0], [0.0; 3]).is_err());
        assert!(position([0.0005, 0.0005, 0.0], [0.0; 3]).is_ok());
    }
    #[test]
    fn wrap_and_pole_exception_match_existing_contract() {
        assert!(wrapped_error(359.999, 0.001, 360.0) < 0.0020000001);
        assert_eq!(wrapped_error(PI, -PI, TAU), 0.0);
        assert!(geodetic(site([90.0, 0.0, 1.0]), site([90.0, 123.0, 1.0])).is_ok());
        assert!(geodetic(site([89.0, 0.0, 1.0]), site([89.0, 123.0, 1.0])).is_err());
    }
}
