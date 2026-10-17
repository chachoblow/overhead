//! Clockless, platform-independent radar application state (`no_std` + alloc).
//! Physical samples retain explicit failures and a common UTC; projection,
//! zoom and identification never change those samples. No scheduling policy.
#![no_std]
extern crate alloc;

use alloc::vec::Vec;
use overhead_core::{
    CoordinateError, EcefPosition, GeodeticPosition, LookAngles, ObservationError, PropagateError,
    TimeError,
    catalogue::{Catalogue, CatalogueEntry},
    ecef_to_geodetic, ecef_to_look_angles,
    sgp4::chrono::NaiveDateTime,
    validate_utc_time,
};
pub mod identification;
pub mod projection;
use identification::LabelZoom;
use projection::{Camera, CameraError, CameraPoint, EarthLimb, EdgeIndicator, View, ViewPoint};

#[derive(Debug, Clone, PartialEq)]
pub enum TrackingError {
    Propagation(PropagateError),
    Coordinates(CoordinateError),
    Observation(ObservationError),
}

impl core::fmt::Display for TrackingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Propagation(e) => write!(f, "propagation: {e}"),
            Self::Coordinates(e) => write!(f, "coordinates: {e}"),
            Self::Observation(e) => write!(f, "observation: {e}"),
        }
    }
}

/// True state; timestamp belongs to the containing app frame, not the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalState {
    pub ecef: EcefPosition,
    pub geodetic: GeodeticPosition,
    pub look: LookAngles,
    /// Instantaneous magnitude of the propagated TEME velocity, in km/s.
    /// Not Earth-fixed ground speed, observer-relative or presentation motion.
    /// A nonfinite magnitude is unavailable to readouts, not a new tracking failure.
    pub teme_speed_km_s: f64,
}

/// Camera-relative position is cached separately from the physical state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub physical: PhysicalState,
    pub camera: CameraPoint,
}

impl Sample {
    pub fn above_horizon(&self) -> bool {
        self.physical.look.elevation_rad > 0.0
    }
    pub fn marker(&self, view: View) -> Option<ViewPoint> {
        if !self.above_horizon() {
            return None;
        }
        view.locate(self.camera)
    }
    pub fn edge_indicator(&self, view: View) -> Option<EdgeIndicator> {
        if !self.above_horizon() {
            return None;
        }
        view.edge_indicator(self.camera)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub catalogue: usize,
    pub above_horizon: usize,
    pub visible: usize,
    pub failed: usize,
}

pub struct RadarApp {
    catalogue: Catalogue,
    observer: GeodeticPosition,
    camera: Camera,
    view: View,
    time: Option<NaiveDateTime>,
    samples: Vec<Result<Sample, TrackingError>>,
    selected: Option<u64>,
    label_zoom: LabelZoom,
    label_visible: bool,
}

impl RadarApp {
    pub fn new(
        catalogue: Catalogue,
        observer: GeodeticPosition,
        view: View,
    ) -> Result<Self, CameraError> {
        let camera = Camera::new(observer)?;
        let samples = Vec::with_capacity(catalogue.entries().len());
        let label_zoom = LabelZoom::default();
        Ok(Self {
            catalogue,
            observer,
            camera,
            view,
            time: None,
            samples,
            selected: None,
            label_zoom,
            label_visible: label_zoom.initially_visible(view),
        })
    }

    /// Configure label zoom policy, resetting its history using the current lens.
    /// Intended for construction; it never changes selection or physical state.
    pub fn with_label_zoom(mut self, policy: LabelZoom) -> Self {
        self.label_zoom = policy;
        self.label_visible = policy.initially_visible(self.view);
        self
    }

    /// Presentation-only gate for the wide renderer. The selected diamond and
    /// baseline diagnostics do not use this gate.
    pub fn selected_label_visible(&self) -> bool {
        self.label_visible && self.selected.is_some()
    }

    /// Compute a complete synchronous frame at caller-supplied UTC. Each entry
    /// gets a new result; a failed update cannot retain an old successful marker.
    /// Invalid UTC invalidates the entire previous frame before returning Err.
    pub fn update(&mut self, time: NaiveDateTime) -> Result<(), TimeError> {
        self.time = None;
        self.samples.clear();
        if let Err(error) = validate_utc_time(time) {
            self.selected = None;
            return Err(error);
        }
        for entry in self.catalogue.entries() {
            self.samples
                .push(sample(entry, time, self.observer, &self.camera));
        }
        self.time = Some(time);
        self.reconcile_selection();
        Ok(())
    }

    pub fn time(&self) -> Option<NaiveDateTime> {
        self.time
    }
    pub fn observer(&self) -> GeodeticPosition {
        self.observer
    }
    pub fn view(&self) -> View {
        self.view
    }
    pub fn earth_limb(&self) -> EarthLimb {
        self.camera.earth_limb(self.view)
    }
    pub fn selected_id(&self) -> Option<u64> {
        self.selected
    }

    /// Entries in ascending NORAD order; empty until a valid-time update.
    pub fn objects(
        &self,
    ) -> impl Iterator<Item = (&CatalogueEntry, &Result<Sample, TrackingError>)> {
        self.catalogue.entries().iter().zip(&self.samples)
    }

    pub fn markers(&self) -> impl Iterator<Item = (&CatalogueEntry, &Sample, ViewPoint)> {
        self.objects().filter_map(|(entry, result)| {
            let sample = result.as_ref().ok()?;
            Some((entry, sample, sample.marker(self.view)?))
        })
    }

    /// One candidate per eligible off-screen object, even if arrows overlap.
    /// Indicators are not visible markers and do not participate in selection.
    pub fn indicators(&self) -> impl Iterator<Item = (&CatalogueEntry, EdgeIndicator)> {
        self.objects().filter_map(|(entry, result)| {
            Some((entry, result.as_ref().ok()?.edge_indicator(self.view)?))
        })
    }

    pub fn selected(&self) -> Option<(&CatalogueEntry, &Sample)> {
        self.objects().find_map(|(entry, result)| {
            if Some(entry.elements().norad_id) == self.selected {
                Some((entry, result.as_ref().ok()?))
            } else {
                None
            }
        })
    }

    pub fn counts(&self) -> Counts {
        let mut counts = Counts {
            catalogue: self.catalogue.entries().len(),
            ..Counts::default()
        };
        for (_, result) in self.objects() {
            match result {
                Ok(s) => {
                    counts.above_horizon += usize::from(s.above_horizon());
                    counts.visible += usize::from(s.marker(self.view).is_some());
                }
                Err(_) => counts.failed += 1,
            }
        }
        counts
    }

    pub fn zoom(&mut self, steps: i32) {
        self.view.zoom(steps);
        self.label_visible = self.label_zoom.after_zoom(self.view, self.label_visible);
        self.reconcile_selection();
    }

    /// Minimal inspect behavior: keep the selected visible ID, otherwise use
    /// the first visible NORAD ID. No ranking, dwell timer or production policy.
    fn reconcile_selection(&mut self) {
        if !self
            .markers()
            .any(|(e, _, _)| Some(e.elements().norad_id) == self.selected)
        {
            let first = self.markers().next().map(|(e, _, _)| e.elements().norad_id);
            self.selected = first;
        }
    }

    pub fn select_next(&mut self) {
        let next = self
            .markers()
            .map(|(e, _, _)| e.elements().norad_id)
            .find(|id| self.selected.is_none_or(|selected| *id > selected))
            .or_else(|| self.markers().next().map(|(e, _, _)| e.elements().norad_id));
        self.selected = next;
    }
}

fn sample(
    entry: &CatalogueEntry,
    time: NaiveDateTime,
    observer: GeodeticPosition,
    camera: &Camera,
) -> Result<Sample, TrackingError> {
    let state = entry
        .satellite()
        .state_at(time)
        .map_err(TrackingError::Propagation)?;
    let ecef = state.to_ecef().map_err(TrackingError::Coordinates)?;
    let geodetic = ecef_to_geodetic(ecef).map_err(TrackingError::Coordinates)?;
    let look = ecef_to_look_angles(ecef, observer).map_err(TrackingError::Observation)?;
    let camera = camera.project(ecef).map_err(TrackingError::Coordinates)?;
    Ok(Sample {
        physical: PhysicalState {
            ecef,
            geodetic,
            look,
            teme_speed_km_s: teme_speed(state.velocity()),
        },
        camera,
    })
}

// Scaled hypot avoids overflow/underflow from squaring finite components.
// If the magnitude itself is unrepresentable, leave it nonfinite so only the
// speed readout is suppressed; do not change successful position/selection.
fn teme_speed([x, y, z]: [f64; 3]) -> f64 {
    libm::hypot(libm::hypot(x, y), z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use overhead_core::{
        OmmElements,
        catalogue::{CatalogueBuilder, RecordOrigin},
        sgp4::chrono::{Duration, NaiveDate},
    };

    #[test]
    fn speed_magnitude_is_unsigned_and_handles_extreme_components() {
        assert_eq!(teme_speed([0.0; 3]), 0.0);
        assert_eq!(teme_speed([3.0, -4.0, 12.0]), 13.0);
        assert_eq!(teme_speed([-3.0, 4.0, -12.0]), 13.0);
        for scale in [1e-200, 1e200] {
            let speed = teme_speed([3.0 * scale, -4.0 * scale, 12.0 * scale]);
            assert!((speed / scale - 13.0).abs() < 1e-12);
        }
        assert_eq!(teme_speed([f64::MAX, 0.0, 0.0]), f64::MAX);
        assert!(!teme_speed([f64::MAX; 3]).is_finite());
    }

    fn app() -> RadarApp {
        let elements: Vec<OmmElements> =
            serde_json::from_str(include_str!("../../core/tests/fixtures/iss-25544.json")).unwrap();
        let mut builder = CatalogueBuilder::new();
        for (record, e) in elements.into_iter().enumerate() {
            builder.push(RecordOrigin { group: 0, record }, e);
        }
        let catalogue = builder.finish().unwrap();
        let entry = &catalogue.entries()[0];
        let ecef = entry
            .satellite()
            .state_at(entry.satellite().epoch())
            .unwrap()
            .to_ecef()
            .unwrap();
        let mut observer = ecef_to_geodetic(ecef).unwrap();
        observer.altitude_km = 0.0;
        RadarApp::new(catalogue, observer, View::new(20.0).unwrap()).unwrap()
    }

    #[test]
    fn real_state_matches_engine_and_zoom_never_changes_truth() {
        let mut app = app();
        assert_eq!(app.time(), None);
        assert_eq!(app.markers().count(), 0);
        let entry = &app.catalogue.entries()[0];
        let time = entry.satellite().epoch();
        let state = entry.satellite().state_at(time).unwrap();
        let ecef = state.to_ecef().unwrap();
        let reference_speed = state.velocity().iter().map(|v| v * v).sum::<f64>().sqrt();
        app.update(time).unwrap();
        let physical = app.samples[0].as_ref().unwrap().physical;
        assert_eq!(physical.ecef, ecef);
        assert_eq!(physical.geodetic, ecef_to_geodetic(ecef).unwrap());
        assert!((physical.teme_speed_km_s - reference_speed).abs() < 1e-12);
        assert_eq!(
            physical.look,
            ecef_to_look_angles(ecef, app.observer).unwrap()
        );
        assert_eq!(
            app.counts(),
            Counts {
                catalogue: 1,
                above_horizon: 1,
                visible: 1,
                failed: 0
            }
        );
        assert_eq!(app.selected_id(), Some(25544));
        for step in [10, -20, 64, -64] {
            app.zoom(step);
            app.select_next();
            assert_eq!(app.samples[0].as_ref().unwrap().physical, physical);
        }
        let old_marker = app.markers().next().unwrap().2;
        app.update(time + Duration::seconds(30)).unwrap();
        assert_ne!(app.markers().next().unwrap().2, old_marker);
        assert_ne!(app.samples[0].as_ref().unwrap().physical.ecef, ecef);
        let speed = app.samples[0].as_ref().unwrap().physical.teme_speed_km_s;
        let later = app.catalogue.entries()[0]
            .satellite()
            .state_at(time + Duration::seconds(30))
            .unwrap();
        let reference = later.velocity().iter().map(|v| v * v).sum::<f64>().sqrt();
        assert!((speed - reference).abs() < 1e-12);
        assert_ne!(speed, physical.teme_speed_km_s);
        assert_eq!(app.time(), Some(time + Duration::seconds(30)));
    }

    #[test]
    fn horizon_is_strict_and_independent_of_zoom() {
        let mut app = app();
        let time = app.catalogue.entries()[0].satellite().epoch();
        app.update(time).unwrap();
        let mut s = *app.samples[0].as_ref().unwrap();
        s.camera = CameraPoint {
            east_km: 500.0,
            north_km: 0.0,
            depth_km: 30_000.0,
        };
        for elevation in [-0.1, 0.0, -0.0] {
            s.physical.look.elevation_rad = elevation;
            for fov in [0.1, 20.0, 100.0] {
                assert!(s.marker(View::new(fov).unwrap()).is_none());
                assert!(
                    s.edge_indicator(View::rectangular(fov, 173.0 / 117.0).unwrap())
                        .is_none()
                );
            }
        }
        s.physical.look.elevation_rad = 0.1;
        assert!(s.marker(View::new(0.1).unwrap()).is_none());
        assert!(
            s.edge_indicator(View::rectangular(0.1, 173.0 / 117.0).unwrap())
                .is_some()
        );
        assert!(s.marker(View::new(20.0).unwrap()).is_some());
        s.camera.depth_km = -1.0;
        assert!(s.marker(View::new(100.0).unwrap()).is_none());
        assert!(
            s.edge_indicator(View::rectangular(0.1, 173.0 / 117.0).unwrap())
                .is_none()
        );
    }

    #[test]
    fn failed_samples_and_invalid_time_never_leave_old_markers() {
        let mut app = app();
        let time = app.catalogue.entries()[0].satellite().epoch();
        app.update(time).unwrap();
        assert_eq!(app.markers().count(), 1);
        // Force a real coordinate-stage error for the next sampling pass.
        app.observer.latitude_rad = f64::NAN;
        app.update(time + Duration::seconds(1)).unwrap();
        assert!(matches!(app.samples[0], Err(TrackingError::Observation(_))));
        assert_eq!(app.counts().failed, 1);
        assert_eq!(app.markers().count(), 0);
        assert_eq!(app.selected_id(), None);
        let invalid = NaiveDate::from_ymd_opt(2101, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert!(app.update(invalid).is_err());
        assert_eq!(app.time(), None);
        assert!(app.samples.is_empty());
        assert_eq!(app.selected_id(), None);
    }

    #[test]
    fn mixed_real_catalogue_selection_cycles_and_frame_matches_engine() {
        let elements: Vec<OmmElements> =
            serde_json::from_str(include_str!("../../tools/fixtures/m2-pool/pool.json")).unwrap();
        let mut builder = CatalogueBuilder::new();
        for (record, e) in elements.into_iter().enumerate() {
            builder.push(RecordOrigin { group: 0, record }, e);
        }
        let observer = GeodeticPosition {
            latitude_rad: 39.007_f64.to_radians(),
            longitude_rad: (-104.883_f64).to_radians(),
            altitude_km: 2.187,
        };
        let mut app = RadarApp::new(
            builder.finish().unwrap(),
            observer,
            View::new(100.0).unwrap(),
        )
        .unwrap();
        let time = NaiveDate::from_ymd_opt(2026, 10, 10)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        app.update(time).unwrap();
        let ids: Vec<_> = app
            .markers()
            .map(|(e, _, _)| e.elements().norad_id)
            .collect();
        assert_eq!(ids.len(), 4);
        for expected in ids.iter().cycle().take(ids.len() * 2) {
            assert_eq!(app.selected_id(), Some(*expected));
            app.select_next();
        }
        for (entry, result) in app.objects() {
            let s = result.as_ref().unwrap();
            let state = entry.satellite().state_at(time).unwrap();
            let ecef = state.to_ecef().unwrap();
            let reference = state.velocity().iter().map(|v| v * v).sum::<f64>().sqrt();
            assert!((s.physical.teme_speed_km_s - reference).abs() < 1e-12);
            let other_observer = GeodeticPosition {
                latitude_rad: -0.6,
                longitude_rad: 2.6,
                altitude_km: 0.0,
            };
            let other = sample(
                entry,
                time,
                other_observer,
                &Camera::new(other_observer).unwrap(),
            )
            .unwrap();
            assert_eq!(s.physical.teme_speed_km_s, other.physical.teme_speed_km_s);
            assert_ne!(s.physical.look, other.physical.look);
            assert_eq!(s.physical.ecef, ecef);
            assert_eq!(
                s.physical.look,
                ecef_to_look_angles(ecef, observer).unwrap()
            );
        }
        let selected = app.selected_id();
        app.update(time + Duration::seconds(1)).unwrap();
        assert_eq!(app.selected_id(), selected);

        // Independent projection reference from core's range/elevation/azimuth,
        // rather than the camera's ECEF basis rotation. Zoom only changes lens.
        let original = app.samples.clone();
        for fov in [0.1_f64, 5.0, 20.0, 60.0, 100.0] {
            app.view = View::new(fov).unwrap();
            let half_span = libm::tan(fov.to_radians() / 2.0);
            for (_, result) in app.objects() {
                let s = result.as_ref().unwrap();
                let look = s.physical.look;
                let up = look.range_km * libm::sin(look.elevation_rad);
                let horizontal = look.range_km * libm::cos(look.elevation_rad);
                let depth = projection::CAMERA_HEIGHT_KM - up;
                let azimuth = look.azimuth_rad.unwrap_or(0.0);
                let expected_visible = look.elevation_rad > 0.0
                    && depth >= projection::NEAR_PLANE_KM
                    && (horizontal * libm::sin(azimuth)).abs() / depth <= half_span
                    && (horizontal * libm::cos(azimuth)).abs() / depth <= half_span;
                let marker = s.marker(app.view);
                assert_eq!(marker.is_some(), expected_visible);
                if let Some(marker) = marker {
                    let scale = horizontal / depth / half_span;
                    assert!((marker.east - scale * libm::sin(azimuth)).abs() < 1e-10);
                    assert!((marker.north - scale * libm::cos(azimuth)).abs() < 1e-10);
                }
            }
            assert_eq!(app.counts().above_horizon, 4);
            assert_eq!(app.counts().failed, 0);
            assert_eq!(app.samples, original);
        }
    }

    #[test]
    fn actual_sgp4_failure_replaces_a_previous_success() {
        // Vallado 33333, already independently tested by core: succeeds at
        // +20 minutes and fails at +25. This checks the app's frame lifecycle.
        let elements = overhead_core::sgp4::Elements::from_tle(
            None,
            b"1 33333U 05037B   05333.02012661  .25992681  00000-0  24476-3 0  1532",
            b"2 33333  96.4736 157.9986 9950000 244.0492 110.6523  4.00004038 10700",
        )
        .unwrap();
        let epoch = elements.datetime;
        let checked: OmmElements =
            serde_json::from_str(&serde_json::to_string(&elements).unwrap()).unwrap();
        let mut builder = CatalogueBuilder::new();
        builder.push(
            RecordOrigin {
                group: 0,
                record: 0,
            },
            checked,
        );
        let mut app = RadarApp::new(
            builder.finish().unwrap(),
            GeodeticPosition {
                latitude_rad: 0.0,
                longitude_rad: 0.0,
                altitude_km: 0.0,
            },
            View::new(20.0).unwrap(),
        )
        .unwrap();
        app.update(epoch + Duration::minutes(20)).unwrap();
        assert!(app.samples[0].is_ok());
        app.update(epoch + Duration::minutes(25)).unwrap();
        assert!(matches!(
            app.samples[0],
            Err(TrackingError::Propagation(PropagateError::Sgp4(_)))
        ));
        assert_eq!(app.counts().failed, 1);
        assert_eq!(app.markers().count(), 0);
    }

    #[test]
    fn square_corners_update_counts_and_selection_without_changing_truth() {
        use projection::Viewport;
        let mut app = app();
        let time = app.catalogue.entries()[0].satellite().epoch();
        app.update(time).unwrap();
        // Synthetic camera placement isolates shape from propagation.
        let span = libm::tan(20.0_f64.to_radians() / 2.0);
        app.samples[0].as_mut().unwrap().camera = CameraPoint {
            east_km: span * 0.8,
            north_km: span * 0.8,
            depth_km: 1.0,
        };
        let original = app.samples.clone();
        for viewport in [
            Viewport::Circle,
            Viewport::BorderlessCircle,
            Viewport::Square,
            Viewport::Circle,
        ] {
            app.view = app.view.with_viewport(viewport);
            app.reconcile_selection();
            let expected = usize::from(viewport == Viewport::Square);
            assert_eq!(app.counts().visible, expected);
            assert_eq!(app.markers().count(), expected);
            assert_eq!(app.counts().above_horizon, 1);
            assert_eq!(app.counts().failed, 0);
            assert_eq!(app.selected_id(), (expected == 1).then_some(25544));
            app.select_next();
            assert_eq!(app.selected_id(), (expected == 1).then_some(25544));
            assert_eq!(app.samples, original);
        }
        app.view = app.view.with_viewport(Viewport::Square);
        app.samples[0].as_mut().unwrap().physical.look.elevation_rad = 0.0;
        app.reconcile_selection();
        assert_eq!(app.counts().visible, 0);
        assert_eq!(app.selected_id(), None);
    }

    #[test]
    fn overlapping_indicators_are_per_object_and_do_not_change_counts_or_selection() {
        let elements: Vec<OmmElements> =
            serde_json::from_str(include_str!("../../tools/fixtures/m2-pool/pool.json")).unwrap();
        let mut builder = CatalogueBuilder::new();
        for (record, e) in elements.into_iter().enumerate() {
            builder.push(RecordOrigin { group: 0, record }, e);
        }
        let mut app = RadarApp::new(
            builder.finish().unwrap(),
            GeodeticPosition {
                latitude_rad: 0.0,
                longitude_rad: 0.0,
                altitude_km: 0.0,
            },
            View::rectangular(20.0, 173.0 / 117.0).unwrap(),
        )
        .unwrap();
        let time = NaiveDate::from_ymd_opt(2026, 10, 10)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        app.update(time).unwrap();
        // Deliberately coincident image positions isolate arrow multiplicity.
        for result in &mut app.samples {
            let sample = result.as_mut().unwrap();
            sample.physical.look.elevation_rad = 0.1;
            sample.camera = CameraPoint {
                east_km: 1.0,
                north_km: 0.0,
                depth_km: 1.0,
            };
        }
        app.reconcile_selection();
        assert_eq!(app.indicators().count(), 16);
        assert_eq!(app.counts().visible, 0);
        assert_eq!(app.selected_id(), None);
        let arrows: Vec<_> = app.indicators().collect();
        assert!(arrows.windows(2).all(|pair| pair[0].1 == pair[1].1));
        app.samples[0] = Err(TrackingError::Coordinates(CoordinateError::NonFinite));
        app.samples[1].as_mut().unwrap().physical.look.elevation_rad = 0.0;
        app.samples[2].as_mut().unwrap().camera.depth_km = 0.999;
        app.samples[3].as_mut().unwrap().camera.east_km = 0.0;
        let original = app.samples.clone();
        app.reconcile_selection();
        let selected = app.catalogue.entries()[3].elements().norad_id;
        assert_eq!(app.selected_id(), Some(selected));
        assert_eq!(
            app.counts(),
            Counts {
                catalogue: 16,
                above_horizon: 14,
                visible: 1,
                failed: 1
            }
        );
        assert_eq!(app.indicators().count(), 12);
        app.select_next();
        assert_eq!(app.selected_id(), Some(selected));
        app.zoom(-64);
        assert_eq!(app.indicators().count(), 0);
        assert_eq!(
            app.counts(),
            Counts {
                catalogue: 16,
                above_horizon: 14,
                visible: 13,
                failed: 1
            }
        );
        assert_eq!(app.selected_id(), Some(selected));
        assert_eq!(app.samples, original);
        app.zoom(64);
        assert_eq!(app.indicators().count(), 12);
        assert_eq!(app.samples, original);
        let invalid = NaiveDate::from_ymd_opt(2101, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert!(app.update(invalid).is_err());
        assert_eq!(app.indicators().count(), 0);
        assert_eq!(app.selected_id(), None);
    }

    #[test]
    fn label_zoom_history_survives_frames_selection_and_missing_data() {
        let mut app = app();
        let time = app.catalogue.entries()[0].satellite().epoch();
        app.update(time).unwrap();
        let samples = app.samples.clone();
        let counts = app.counts();
        let selected = app.selected_id();
        assert!(app.selected_label_visible());
        app.zoom(-64);
        assert!(!app.selected_label_visible());
        assert_eq!(app.selected_id(), selected);
        assert_eq!(app.counts(), counts);
        assert_eq!(app.samples, samples);
        // From 100 degrees, three zoom-in steps reach the hysteresis interval.
        app.zoom(3);
        assert!((60.0..70.0).contains(&app.view().fov_deg()));
        assert!(!app.selected_label_visible());
        app.select_next();
        app.update(time).unwrap();
        app.zoom(0);
        assert!(!app.selected_label_visible());
        let invalid = NaiveDate::from_ymd_opt(2101, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert!(app.update(invalid).is_err());
        app.update(time).unwrap();
        assert!(!app.selected_label_visible());
        assert_eq!(app.selected_id(), selected);
        app.zoom(1);
        assert!(app.view().fov_deg() <= 60.0);
        assert!(app.selected_label_visible());
        app.zoom(-1);
        assert!((60.0..70.0).contains(&app.view().fov_deg()));
        assert!(app.selected_label_visible());
        assert_eq!(app.samples, samples);
        assert_eq!(app.counts(), counts);
    }

    #[test]
    fn selection_wraps_and_clears_when_zoom_clips_everything() {
        let mut app = app();
        let time = app.catalogue.entries()[0].satellite().epoch();
        app.update(time).unwrap();
        app.select_next();
        assert_eq!(app.selected_id(), Some(25544));
        app.samples[0].as_mut().unwrap().camera = CameraPoint {
            east_km: 500.0,
            north_km: 0.0,
            depth_km: 30_000.0,
        };
        app.zoom(64);
        assert_eq!(app.selected_id(), None);
        app.select_next();
        assert_eq!(app.selected_id(), None);
        app.zoom(-64);
        assert_eq!(app.selected_id(), Some(25544));
    }
}
