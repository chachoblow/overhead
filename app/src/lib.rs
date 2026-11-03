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
    sgp4::chrono::{NaiveDateTime, TimeDelta},
    validate_utc_time,
};
pub mod category;
use category::Category;
pub mod identification;
pub mod projection;
pub mod visibility;
use identification::LabelZoom;
use projection::{
    Camera, CameraError, CameraPoint, EarthLimb, EdgeIndicator, View, ViewPoint, Viewport,
};
use visibility::Visibility;

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

/// Element freshness policy for fetched catalogues (0075). An explicit tuning
/// input; apps without one never call their data old or expire it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Freshness {
    /// A fetched catalogue whose fetch time is more than this before the frame
    /// UTC is stale: still tracked, but flagged (`OLD DATA`).
    pub stale_after: TimeDelta,
    /// A catalogue whose newest element epoch is more than this before the
    /// frame UTC is unloaded, so the radar reports `NO DATA` until a new load.
    pub expire_after: TimeDelta,
}

impl Freshness {
    /// 0075: stale after 24 h without a successful fetch (three missed 8-hour
    /// refreshes); expired once the newest epoch is more than 7 days old.
    pub const DEVICE: Self = Self {
        stale_after: TimeDelta::hours(24),
        expire_after: TimeDelta::days(7),
    };
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub catalogue: usize,
    pub above_horizon: usize,
    pub visible: usize,
    pub failed: usize,
}

pub struct RadarApp {
    /// `None` until a catalogue is loaded; core catalogues are never empty.
    catalogue: Option<Catalogue>,
    group_categories: Vec<Category>,
    observer: GeodeticPosition,
    camera: Camera,
    view: View,
    time: Option<NaiveDateTime>,
    samples: Vec<Result<Sample, TrackingError>>,
    /// UTC each sample was computed at: the frame UTC after a full update,
    /// up to the staggered window older for objects not yet revisited.
    sampled_at: Vec<NaiveDateTime>,
    /// Next entry the staggered update recomputes (round-robin).
    cursor: usize,
    /// Operating limit applied to each load (0077); `None` keeps everything.
    object_limit: Option<usize>,
    /// Entries the limit dropped from the current catalogue.
    dropped: usize,
    selected: Option<u64>,
    label_zoom: LabelZoom,
    label_visible: bool,
    freshness: Option<Freshness>,
    /// UTC at which the loaded catalogue was fetched, if it was.
    fetched_at: Option<NaiveDateTime>,
    /// Newest element epoch of the loaded catalogue, for expiry.
    newest_epoch: Option<NaiveDateTime>,
}

impl RadarApp {
    pub fn new(
        catalogue: Catalogue,
        observer: GeodeticPosition,
        view: View,
    ) -> Result<Self, CameraError> {
        let mut app = Self::awaiting_catalogue(observer, view)?;
        app.samples = Vec::with_capacity(catalogue.entries().len());
        app.newest_epoch = newest_epoch(&catalogue);
        app.catalogue = Some(catalogue);
        Ok(app)
    }

    /// An app with no catalogue yet (for example before a device fetch
    /// completes, or after it failed). Frames are valid but hold no objects, so
    /// counts report an empty load and no horizon claim is possible.
    pub fn awaiting_catalogue(observer: GeodeticPosition, view: View) -> Result<Self, CameraError> {
        let camera = Camera::new(observer)?;
        let label_zoom = LabelZoom::default();
        Ok(Self {
            catalogue: None,
            group_categories: Vec::new(),
            observer,
            camera,
            view,
            time: None,
            samples: Vec::new(),
            sampled_at: Vec::new(),
            cursor: 0,
            object_limit: None,
            dropped: 0,
            selected: None,
            label_zoom,
            label_visible: label_zoom.initially_visible(view),
            freshness: None,
            fetched_at: None,
            newest_epoch: None,
        })
    }

    /// Replace the whole catalogue, keeping observer, view, roles and label
    /// state. A current frame is recomputed at its own UTC so no sample from
    /// the previous catalogue survives. The selected ID is kept only if the new
    /// catalogue still tracks it under the usual rules (0049); otherwise
    /// selection falls back to first acquisition. A plain load has no fetch
    /// time, so it is never called stale.
    pub fn load_catalogue(&mut self, catalogue: Catalogue) {
        self.load(catalogue, None);
    }

    /// As [`Self::load_catalogue`], recording the UTC the elements were fetched
    /// at so the freshness policy can flag them as stale (0075).
    pub fn load_fetched_catalogue(&mut self, catalogue: Catalogue, fetched_at: NaiveDateTime) {
        self.load(catalogue, Some(fetched_at));
    }

    fn load(&mut self, mut catalogue: Catalogue, fetched_at: Option<NaiveDateTime>) {
        self.dropped = match self.object_limit {
            Some(max) => {
                let roles = &self.group_categories;
                catalogue.limit(max, |e| Category::from_groups(e.groups(), roles).priority())
            }
            None => 0,
        };
        self.newest_epoch = newest_epoch(&catalogue);
        self.catalogue = Some(catalogue);
        self.fetched_at = fetched_at;
        match self.time {
            // Already validated when the frame was made.
            Some(time) => {
                let _ = self.update(time);
            }
            None => {
                self.samples.clear();
                self.sampled_at.clear();
                self.selected = None;
            }
        }
    }

    /// Configure the operating object limit (0077), applied to each later
    /// load: higher category role first, then lower NORAD ID. Intended for
    /// construction, before the first load.
    pub fn with_object_limit(mut self, max: usize) -> Self {
        self.object_limit = Some(max);
        self
    }

    /// Entries the object limit dropped from the loaded catalogue.
    pub fn dropped_objects(&self) -> usize {
        self.dropped
    }

    /// Configure the freshness policy. Intended for construction.
    pub fn with_freshness(mut self, freshness: Freshness) -> Self {
        self.freshness = Some(freshness);
        self
    }

    /// The loaded catalogue's fetch time, if it was fetched.
    pub fn fetched_at(&self) -> Option<NaiveDateTime> {
        self.fetched_at
    }

    /// True when a frame exists and its UTC is more than the policy's
    /// `stale_after` past the catalogue's fetch time. Tracking is unchanged;
    /// this only drives the `OLD DATA` notice and clears on the next load.
    pub fn data_stale(&self) -> bool {
        match (self.freshness, self.time, self.fetched_at) {
            (Some(f), Some(time), Some(fetched)) => {
                self.catalogue.is_some() && time - fetched > f.stale_after
            }
            _ => false,
        }
    }

    fn entries(&self) -> &[CatalogueEntry] {
        self.catalogue.as_ref().map_or(&[], Catalogue::entries)
    }

    /// Supply explicit roles indexed by the catalogue's source-group indices.
    /// Omitted/out-of-range entries contribute Unknown. Intended for construction;
    /// it changes neither physical samples nor selection. No per-ID role cache.
    pub fn with_group_categories(mut self, categories: Vec<Category>) -> Self {
        self.group_categories = categories;
        self
    }

    /// No selection is distinct from a selected but unclassified object.
    pub fn selected_category(&self) -> Option<Category> {
        Some(Category::from_groups(
            self.selected()?.0.groups(),
            &self.group_categories,
        ))
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

    /// Discard tracking when UTC is no longer trusted, without inventing an
    /// invalid timestamp. Retain catalogue, observer and presentation settings.
    /// The renderer can now show NO FRAME instead of stale markers/readouts.
    pub fn invalidate_frame(&mut self) {
        self.time = None;
        self.samples.clear();
        self.sampled_at.clear();
        self.selected = None;
    }

    /// Validate UTC and apply expiry before any sample is computed.
    fn check_frame_time(&mut self, time: NaiveDateTime) -> Result<(), TimeError> {
        if let Err(error) = validate_utc_time(time) {
            self.invalidate_frame();
            return Err(error);
        }
        if let (Some(f), Some(newest)) = (self.freshness, self.newest_epoch)
            && time - newest > f.expire_after
        {
            // Expired elements are unloaded rather than shown as current; only
            // a new load brings markers back.
            self.catalogue = None;
            self.fetched_at = None;
            self.newest_epoch = None;
            self.dropped = 0;
            self.selected = None;
            self.samples.clear();
            self.sampled_at.clear();
        }
        Ok(())
    }

    fn resample(&mut self, index: usize, time: NaiveDateTime) {
        let entries = self.catalogue.as_ref().map_or(&[][..], Catalogue::entries);
        self.samples[index] = sample(&entries[index], time, self.observer, &self.camera);
        self.sampled_at[index] = time;
    }

    fn selected_index(&self) -> Option<usize> {
        let id = self.selected?;
        self.entries()
            .binary_search_by_key(&id, |e| e.elements().norad_id)
            .ok()
    }

    /// Staggered frame (0077): the selected object is recomputed at `time`,
    /// and the others round-robin so each is within `window` of the frame
    /// time. A first frame, a new catalogue or a step of `window` or more in
    /// either direction falls back to a complete [`Self::update`]. A smaller
    /// backward step (a small clock resync, 0063 note) is staggered like a
    /// forward one by its magnitude. Markers of objects not revisited this
    /// frame stay where they were up to `window` away; readouts use only the
    /// selected object, which is always current.
    pub fn update_staggered(
        &mut self,
        time: NaiveDateTime,
        window: TimeDelta,
    ) -> Result<(), TimeError> {
        let previous = self.time;
        self.check_frame_time(time)?;
        let n = self.entries().len();
        let Some(previous) = previous else {
            return self.update(time);
        };
        let elapsed = (time - previous).abs();
        if self.samples.len() != n || elapsed >= window || window <= TimeDelta::zero() {
            return self.update(time);
        }
        let (elapsed_us, window_us) = (
            elapsed.num_microseconds().unwrap_or(i64::MAX),
            window.num_microseconds().unwrap_or(i64::MAX),
        );
        // ceil(n * elapsed / window), so a full cycle completes within the window.
        let count = ((n as i128 * elapsed_us as i128 + window_us as i128 - 1) / window_us as i128)
            .clamp(0, n as i128) as usize;
        for k in 0..count {
            self.resample((self.cursor + k) % n, time);
        }
        if n > 0 {
            self.cursor = (self.cursor + count) % n;
        }
        if let Some(index) = self.selected_index() {
            self.resample(index, time);
        }
        self.time = Some(time);
        let before = self.selected;
        self.reconcile_selection();
        if self.selected != before
            && let Some(index) = self.selected_index()
        {
            self.resample(index, time);
            // The fresh sample can change eligibility; settle once more.
            self.reconcile_selection();
        }
        Ok(())
    }

    /// Oldest sample UTC in the current frame, if any (telemetry/tests).
    pub fn oldest_sample_utc(&self) -> Option<NaiveDateTime> {
        self.sampled_at.iter().min().copied()
    }

    /// Visibility of the selected object at its own sample UTC (0078).
    pub fn selected_visibility(&self) -> Option<Visibility> {
        let index = self.selected_index()?;
        let sample = self.samples.get(index)?.as_ref().ok()?;
        visibility::classify(sample.physical.ecef, self.observer, self.sampled_at[index])
    }

    /// Compute a complete synchronous frame at caller-supplied UTC. Each entry
    /// gets a new result; a failed update cannot retain an old successful marker.
    /// Invalid UTC invalidates the entire previous frame before returning Err.
    pub fn update(&mut self, time: NaiveDateTime) -> Result<(), TimeError> {
        self.time = None;
        self.samples.clear();
        self.sampled_at.clear();
        self.check_frame_time(time)?;
        // Disjoint field borrows keep the samples allocation reused per frame.
        let entries = self.catalogue.as_ref().map_or(&[][..], Catalogue::entries);
        for entry in entries {
            self.samples
                .push(sample(entry, time, self.observer, &self.camera));
            self.sampled_at.push(time);
        }
        self.cursor = 0;
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
        self.entries().iter().zip(&self.samples)
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

    /// Nominal period of the selected element set, in minutes (1440 / rev/day).
    /// Not a propagated instantaneous period, pass duration or return countdown.
    /// Selection gates availability; the same elements give the same value at
    /// every frame, observer and zoom. No extra propagation or retained scalar.
    pub fn selected_period_minutes(&self) -> Option<f64> {
        nominal_period_minutes(self.selected()?.0.elements().mean_motion)
    }

    pub fn counts(&self) -> Counts {
        let mut counts = Counts {
            catalogue: self.entries().len(),
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

    /// Wide: retain a successfully tracked above-horizon ID across viewport
    /// exits; other comparison modes retain their visible-only selection.
    /// Initial acquisition and genuine loss use the first visible NORAD ID.
    fn reconcile_selection(&mut self) {
        let retained = if self.view.viewport() == Viewport::Rectangle {
            self.objects().any(|(entry, result)| {
                Some(entry.elements().norad_id) == self.selected
                    && result.as_ref().is_ok_and(Sample::above_horizon)
            })
        } else {
            self.markers()
                .any(|(entry, _, _)| Some(entry.elements().norad_id) == self.selected)
        };
        if !retained {
            let first = self
                .markers()
                .next()
                .map(|(entry, _, _)| entry.elements().norad_id);
            self.selected = first;
        }
    }

    /// Tab always chooses an in-view marker; with none, wide selection persists.
    pub fn select_next(&mut self) {
        let next = self
            .markers()
            .map(|(e, _, _)| e.elements().norad_id)
            .find(|id| self.selected.is_none_or(|selected| *id > selected))
            .or_else(|| self.markers().next().map(|(e, _, _)| e.elements().norad_id));
        if next.is_some() || self.view.viewport() != Viewport::Rectangle {
            self.selected = next;
        }
    }
}

fn newest_epoch(catalogue: &Catalogue) -> Option<NaiveDateTime> {
    catalogue
        .entries()
        .iter()
        .map(|e| e.satellite().epoch())
        .max()
}

fn nominal_period_minutes(mean_motion_rev_day: f64) -> Option<f64> {
    if !mean_motion_rev_day.is_finite() || mean_motion_rev_day <= 0.0 {
        return None;
    }
    let period = 1440.0 / mean_motion_rev_day;
    (period.is_finite() && period > 0.0).then_some(period)
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
    fn nominal_period_uses_element_mean_motion_and_rejects_unavailable_results() {
        for (motion, period) in [(16.0, 90.0), (2.0, 720.0), (1.0, 1440.0), (0.5, 2880.0)] {
            assert_eq!(nominal_period_minutes(motion), Some(period));
        }
        for invalid in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -1.0,
            -0.0,
            0.0,
            f64::from_bits(1),
        ] {
            assert_eq!(nominal_period_minutes(invalid), None);
        }
        assert!(nominal_period_minutes(f64::MAX).unwrap() > 0.0);
    }

    #[test]
    fn selected_period_is_element_metadata_and_follows_frame_availability() {
        let mut app = app();
        assert_eq!(app.selected_period_minutes(), None);
        let time = app.entries()[0].satellite().epoch();
        let expected = 1440.0 / app.entries()[0].elements().mean_motion;
        app.update(time).unwrap();
        assert_eq!(app.selected_period_minutes(), Some(expected));
        app.update(time + Duration::seconds(30)).unwrap();
        assert_eq!(app.selected_period_minutes(), Some(expected));
        app.zoom(-8);
        assert_eq!(app.selected_period_minutes(), Some(expected));
        let invalid = NaiveDate::from_ymd_opt(2101, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert!(app.update(invalid).is_err());
        assert_eq!(app.selected_period_minutes(), None);
        app.update(time).unwrap();
        assert_eq!(app.selected_period_minutes(), Some(expected));
    }

    #[test]
    fn category_follows_selection_not_frame_motion_zoom_or_label_visibility() {
        let mut app = app().with_group_categories(alloc::vec![Category::Station]);
        let time = app.entries()[0].satellite().epoch();
        assert_eq!(app.selected_category(), None);
        app.update(time).unwrap();
        assert_eq!(app.selected_category(), Some(Category::Station));
        let counts = app.counts();
        let samples = app.samples.clone();
        app = app.with_group_categories(alloc::vec![Category::Navigation]);
        assert_eq!(app.selected_category(), Some(Category::Navigation));
        assert_eq!(app.samples, samples);
        assert_eq!(app.counts(), counts);
        app.zoom(-100);
        assert!(!app.selected_label_visible());
        assert_eq!(app.selected_category(), Some(Category::Navigation));
        app.update(time + Duration::seconds(30)).unwrap();
        assert_eq!(app.selected_category(), Some(Category::Navigation));
        let invalid = NaiveDate::from_ymd_opt(2101, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert!(app.update(invalid).is_err());
        assert_eq!(app.selected_category(), None);
        app.update(time).unwrap();
        assert_eq!(app.selected_category(), Some(Category::Navigation));
        app = app.with_group_categories(Vec::new());
        assert_eq!(app.selected_category(), Some(Category::Unknown));
    }

    fn visual() -> Catalogue {
        let elements: Vec<OmmElements> = serde_json::from_str(include_str!(
            "../../firmware/fixtures/celestrak-visual-20261103.json"
        ))
        .unwrap();
        let mut builder = CatalogueBuilder::new();
        for (record, e) in elements.into_iter().enumerate() {
            builder.push(RecordOrigin { group: 0, record }, e);
        }
        builder.finish().unwrap()
    }

    fn visual_app() -> RadarApp {
        // Mid-latitude site; the fixture's epochs end 2026-11-02T22:18Z.
        let observer = GeodeticPosition {
            latitude_rad: 40f64.to_radians(),
            longitude_rad: (-105f64).to_radians(),
            altitude_km: 1.6,
        };
        let view = View::rectangular(100.0, 400.0 / 240.0).unwrap();
        let mut app = RadarApp::awaiting_catalogue(observer, view).unwrap();
        app.load_catalogue(visual());
        app
    }

    fn visual_time() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, 3)
            .unwrap()
            .and_hms_opt(2, 0, 0)
            .unwrap()
    }

    #[test]
    fn staggered_update_keeps_selected_current_and_all_within_window() {
        let mut app = visual_app();
        let window = TimeDelta::seconds(1);
        let start = visual_time();
        // First frame is complete.
        app.update_staggered(start, window).unwrap();
        assert_eq!(app.oldest_sample_utc(), Some(start));
        assert_eq!(app.counts().catalogue, 156);
        let mut full = visual_app();
        let step = TimeDelta::milliseconds(50);
        let mut time = start;
        for frame in 1..=100 {
            time += step;
            app.update_staggered(time, window).unwrap();
            full.update(time).unwrap();
            // Every sample is within the window and the selected one is exact.
            assert!(
                time - app.oldest_sample_utc().unwrap() < window,
                "frame {frame}"
            );
            let index = app.selected_index().unwrap();
            assert_eq!(app.sampled_at[index], time);
            let fresh = sample(&app.entries()[index], time, app.observer, &app.camera);
            assert_eq!(app.samples[index], fresh);
            // Counts can differ from a complete frame only through lag.
            assert_eq!(app.counts().catalogue, full.counts().catalogue);
        }
        // 20 frames per window: ceil(156 / 20) = 8 objects per frame.
        assert_eq!(app.cursor, (100 * 8) % 156);
    }

    #[test]
    fn staggered_update_falls_back_to_complete_frames() {
        let mut app = visual_app();
        let window = TimeDelta::seconds(1);
        let t = visual_time();
        app.update_staggered(t, window).unwrap();
        app.update_staggered(t + TimeDelta::milliseconds(50), window)
            .unwrap();
        assert_eq!(app.oldest_sample_utc(), Some(t));
        // A full window either way and a zero window are complete frames.
        for (time, window) in [
            (t - TimeDelta::milliseconds(950), window),
            (t + TimeDelta::seconds(1), window),
            (t + TimeDelta::milliseconds(1050), TimeDelta::zero()),
        ] {
            app.update_staggered(time, window).unwrap();
            assert_eq!(app.oldest_sample_utc(), Some(time));
        }
        // Same UTC again recomputes nothing but the selected object.
        let again = t + TimeDelta::milliseconds(1050);
        let cursor = app.cursor;
        app.update_staggered(again, window).unwrap();
        assert_eq!(app.cursor, cursor);
        // Loading a catalogue recomputes the whole frame at its own UTC.
        app.update_staggered(again + TimeDelta::milliseconds(50), window)
            .unwrap();
        app.load_catalogue(visual());
        assert_eq!(
            app.oldest_sample_utc(),
            Some(again + TimeDelta::milliseconds(50))
        );
        // Invalid UTC invalidates the frame like `update`.
        let invalid = NaiveDate::from_ymd_opt(2101, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert!(app.update_staggered(invalid, window).is_err());
        assert_eq!(app.time(), None);
        assert_eq!(app.oldest_sample_utc(), None);
        assert_eq!(app.selected_id(), None);
    }

    #[test]
    fn small_backward_step_is_staggered_and_stays_within_window() {
        let mut app = visual_app();
        let window = TimeDelta::seconds(1);
        let step = TimeDelta::milliseconds(50);
        let mut time = visual_time();
        app.update_staggered(time, window).unwrap();
        for _ in 0..30 {
            time += step;
            app.update_staggered(time, window).unwrap();
        }
        let selected = app.selected_id();
        assert!(selected.is_some());
        // A −100 ms resync: ceil(156 × 0.1) = 16 objects, not all 156.
        let cursor = app.cursor;
        time -= TimeDelta::milliseconds(100);
        app.update_staggered(time, window).unwrap();
        assert_eq!(app.cursor, (cursor + 16) % 156);
        assert_eq!(app.selected_id(), selected);
        assert_ne!(app.oldest_sample_utc(), Some(time));
        for frame in 0..=40 {
            let index = app.selected_index().unwrap();
            assert_eq!(app.sampled_at[index], time);
            assert!(
                app.sampled_at.iter().all(|&s| (time - s).abs() < window),
                "frame {frame}"
            );
            time += step;
            app.update_staggered(time, window).unwrap();
        }
    }

    #[test]
    fn object_limit_applies_to_loads_with_roles_first() {
        let observer = visual_app().observer;
        let view = visual_app().view;
        // Group 0 has no role: lowest NORAD IDs are kept.
        let mut app = RadarApp::awaiting_catalogue(observer, view)
            .unwrap()
            .with_object_limit(100);
        app.load_catalogue(visual());
        assert_eq!(app.counts().catalogue, 100);
        assert_eq!(app.dropped_objects(), 56);
        let ids: Vec<u64> = app
            .entries()
            .iter()
            .map(|e| e.elements().norad_id)
            .collect();
        let mut all: Vec<u64> = visual()
            .entries()
            .iter()
            .map(|e| e.elements().norad_id)
            .collect();
        all.truncate(100);
        assert_eq!(ids, all);
        // A Station role on the only group changes nothing: all equal.
        let mut app = RadarApp::awaiting_catalogue(observer, view)
            .unwrap()
            .with_group_categories(alloc::vec![Category::Station])
            .with_object_limit(156);
        app.load_catalogue(visual());
        assert_eq!((app.counts().catalogue, app.dropped_objects()), (156, 0));
    }

    #[test]
    fn selected_visibility_follows_selection_and_frame() {
        let mut app = visual_app();
        assert_eq!(app.selected_visibility(), None);
        // 2026-11-03 02:00Z is 19:00 MST at the site: dark sky.
        app.update(visual_time()).unwrap();
        let v = app.selected_visibility().unwrap();
        assert!(matches!(v, Visibility::Visible | Visibility::Eclipsed));
        // 19:00Z is local noon: daylight for any selection.
        let noon = visual_time() + TimeDelta::hours(17);
        app.update(noon).unwrap();
        if app.selected_id().is_some() {
            assert_eq!(app.selected_visibility(), Some(Visibility::Daylight));
        }
        app.invalidate_frame();
        assert_eq!(app.selected_visibility(), None);
    }

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
        let entry = &app.entries()[0];
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
        let later = app.entries()[0]
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
        let time = app.entries()[0].satellite().epoch();
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
    fn explicit_time_loss_clears_tracking_and_recovers_without_changing_configuration() {
        let mut app = app().with_group_categories(alloc::vec![Category::Station]);
        let time = app.entries()[0].satellite().epoch();
        app.update(time).unwrap();
        let observer = app.observer();
        let view = app.view();
        assert!(app.selected().is_some());
        app.invalidate_frame();
        assert_eq!(app.time(), None);
        assert_eq!(app.objects().count(), 0);
        assert_eq!(app.markers().count(), 0);
        assert_eq!(app.indicators().count(), 0);
        assert_eq!(app.selected_id(), None);
        assert_eq!(app.selected_category(), None);
        assert_eq!(app.selected_period_minutes(), None);
        assert_eq!(app.counts().catalogue, 1);
        assert_eq!(app.observer(), observer);
        assert_eq!(app.view(), view);
        app.invalidate_frame(); // repeated loss is harmless
        app.update(time + Duration::seconds(1)).unwrap();
        assert_eq!(app.time(), Some(time + Duration::seconds(1)));
        assert!(app.selected().is_some());
        assert_eq!(app.selected_category(), Some(Category::Station));
    }

    #[test]
    fn failed_samples_and_invalid_time_never_leave_old_markers() {
        let mut app = app();
        let time = app.entries()[0].satellite().epoch();
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
            assert_eq!(
                app.selected_period_minutes(),
                Some(1440.0 / app.selected().unwrap().0.elements().mean_motion)
            );
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
        let time = app.entries()[0].satellite().epoch();
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
    fn wide_selection_survives_zoom_and_replay_motion_but_not_horizon_or_failure() {
        let mut square = app();
        let time = square.entries()[0].satellite().epoch();
        square.update(time).unwrap();
        let mut wide = RadarApp::new(
            app().catalogue.unwrap(),
            square.observer,
            View::rectangular(20.0, 173.0 / 117.0).unwrap(),
        )
        .unwrap()
        .with_group_categories(alloc::vec![Category::Station]);
        wide.update(time + Duration::seconds(30)).unwrap();
        square.update(time + Duration::seconds(30)).unwrap();
        let id = wide.selected_id();
        let original = wide.selected().unwrap().1.physical;
        wide.zoom(64);
        assert_eq!(wide.counts().visible, 0);
        assert_eq!(wide.selected_id(), id);
        assert_eq!(wide.selected().unwrap().1.physical, original);
        assert_eq!(wide.selected_category(), Some(Category::Station));
        wide.select_next(); // Nothing in view: no-op, not selection loss.
        assert_eq!(wide.selected_id(), id);
        wide.update(time + Duration::seconds(60)).unwrap();
        assert_eq!(wide.counts().visible, 0);
        assert_eq!(wide.selected_id(), id);
        assert_ne!(wide.selected().unwrap().1.physical, original);
        wide.zoom(-64);
        assert_eq!(wide.selected_id(), id);
        assert_eq!(wide.counts().visible, 1);

        // Motion out of the viewport without zoom keeps the current ID.
        wide.samples[0].as_mut().unwrap().camera = CameraPoint {
            east_km: 100_000.0,
            north_km: 0.0,
            depth_km: 1.0,
        };
        wide.reconcile_selection();
        assert_eq!(wide.selected_id(), id);
        assert_eq!(wide.counts().visible, 0);
        // Even invalid camera depth is not a physical horizon or tracking loss.
        wide.samples[0].as_mut().unwrap().camera.depth_km = -1.0;
        wide.reconcile_selection();
        assert_eq!(wide.selected_id(), id);
        assert_eq!(wide.indicators().count(), 0);
        wide.samples[0]
            .as_mut()
            .unwrap()
            .physical
            .look
            .elevation_rad = 0.0;
        wide.reconcile_selection();
        assert_eq!(wide.selected_id(), None);
        assert_eq!(wide.selected_category(), None);
        wide.update(time).unwrap();
        assert_eq!(wide.selected_id(), id);
        wide.samples[0] = Err(TrackingError::Observation(
            ObservationError::CoincidentPositions,
        ));
        wide.reconcile_selection();
        assert_eq!(wide.selected_id(), None);
        wide.update(time).unwrap();
        assert_eq!(wide.selected_id(), id);
        let invalid = NaiveDate::from_ymd_opt(2101, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert!(wide.update(invalid).is_err());
        assert_eq!(wide.selected_id(), None);

        square.zoom(64);
        assert_eq!(square.selected_id(), None);
    }

    #[test]
    fn wide_tab_and_loss_fallback_use_only_in_view_markers() {
        let elements: Vec<OmmElements> =
            serde_json::from_str(include_str!("../../tools/fixtures/m2-pool/pool.json")).unwrap();
        let mut builder = CatalogueBuilder::new();
        for (record, e) in elements.into_iter().take(2).enumerate() {
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
        let ids: Vec<_> = app
            .entries()
            .iter()
            .map(|e| e.elements().norad_id)
            .collect();
        for result in &mut app.samples {
            let sample = result.as_mut().unwrap();
            sample.physical.look.elevation_rad = 0.1;
            sample.camera = CameraPoint {
                east_km: 0.0,
                north_km: 0.0,
                depth_km: 10_000.0,
            };
        }
        app.reconcile_selection();
        assert_eq!(app.selected_id(), Some(ids[0]));
        app.select_next(); // Manual choice is sticky even when it moves off-screen.
        assert_eq!(app.selected_id(), Some(ids[1]));
        app.samples[1].as_mut().unwrap().camera.east_km = 100_000.0;
        app.reconcile_selection();
        assert_eq!(app.selected_id(), Some(ids[1]));
        assert_eq!(app.counts().visible, 1);
        app.select_next(); // Off-screen ID wraps to the only visible marker.
        assert_eq!(app.selected_id(), Some(ids[0]));
        app.select_next();
        assert_eq!(app.selected_id(), Some(ids[0]));
        app.samples[1].as_mut().unwrap().camera.east_km = 0.0;
        app.select_next();
        assert_eq!(app.selected_id(), Some(ids[1]));
        app.samples[1].as_mut().unwrap().physical.look.elevation_rad = 0.0;
        app.reconcile_selection();
        assert_eq!(app.selected_id(), Some(ids[0])); // Genuine loss: first visible.
        app.samples[0].as_mut().unwrap().camera.east_km = 100_000.0;
        app.reconcile_selection(); // No new automatic off-screen acquisition.
        assert_eq!(app.selected_id(), Some(ids[0]));
        app.samples[0].as_mut().unwrap().physical.look.elevation_rad = 0.0;
        app.reconcile_selection();
        assert_eq!(app.selected_id(), None);
        app.samples[1].as_mut().unwrap().physical.look.elevation_rad = 0.1;
        app.reconcile_selection();
        assert_eq!(app.selected_id(), Some(ids[1]));
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
        app.selected = None; // No initial off-screen auto-acquisition.
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
        let selected = app.entries()[3].elements().norad_id;
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
        let time = app.entries()[0].satellite().epoch();
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
        let time = app.entries()[0].satellite().epoch();
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

    #[test]
    fn awaiting_catalogue_frames_are_valid_but_empty_until_loaded() {
        let loaded = app();
        let time = loaded.entries()[0].satellite().epoch();
        let mut empty =
            RadarApp::awaiting_catalogue(loaded.observer, View::new(20.0).unwrap()).unwrap();
        assert_eq!(empty.time(), None);
        empty.update(time).unwrap();
        assert_eq!(empty.time(), Some(time));
        assert_eq!(empty.counts(), Counts::default());
        assert_eq!(empty.objects().count(), 0);
        assert_eq!(empty.selected_id(), None);
        assert!(empty.selected().is_none());

        // Loading mid-frame resamples at the frame's own UTC and acquires.
        empty.load_catalogue(app().catalogue.unwrap());
        assert_eq!(empty.time(), Some(time));
        let mut reference = app();
        reference.update(time).unwrap();
        assert_eq!(empty.samples, reference.samples);
        assert_eq!(empty.counts(), reference.counts());
        assert_eq!(empty.selected_id(), Some(25544));
    }

    #[test]
    fn loading_without_a_frame_keeps_no_frame_and_replaces_old_samples() {
        let mut loaded = app();
        let time = loaded.entries()[0].satellite().epoch();
        loaded.update(time).unwrap();
        loaded.invalidate_frame();
        loaded.load_catalogue(app().catalogue.unwrap());
        assert_eq!(loaded.time(), None);
        assert_eq!(loaded.objects().count(), 0);
        assert_eq!(loaded.selected_id(), None);
        loaded.update(time).unwrap();
        assert_eq!(loaded.selected_id(), Some(25544));
    }
    fn renumbered(id: u64) -> Catalogue {
        let json = include_str!("../../core/tests/fixtures/iss-25544.json");
        let renumbered = json.replace("25544", &alloc::format!("{id}"));
        let mut elements: Vec<OmmElements> = serde_json::from_str(&renumbered).unwrap();
        let mut builder = CatalogueBuilder::new();
        builder.push(
            RecordOrigin {
                group: 0,
                record: 0,
            },
            elements.remove(0),
        );
        builder.finish().unwrap()
    }

    #[test]
    fn refresh_keeps_a_still_tracked_selection_and_falls_back_otherwise() {
        let mut app = app();
        let time = app.entries()[0].satellite().epoch();
        app.update(time).unwrap();
        assert_eq!(app.selected_id(), Some(25544));
        app.load_fetched_catalogue(app_catalogue(), time);
        assert_eq!(app.selected_id(), Some(25544));
        // The selected ID is gone: first acquisition of the new load.
        app.load_fetched_catalogue(renumbered(90_000), time);
        assert_eq!(app.selected_id(), Some(90_000));
        let mut reference = RadarApp::new(renumbered(90_000), app.observer, app.view).unwrap();
        reference.update(time).unwrap();
        assert_eq!(app.samples, reference.samples);
    }

    fn app_catalogue() -> Catalogue {
        app().catalogue.unwrap()
    }

    #[test]
    fn fetched_data_turns_stale_after_the_policy_and_clears_on_a_new_fetch() {
        let policy = Freshness {
            stale_after: Duration::hours(24),
            expire_after: Duration::days(7),
        };
        let base = app();
        let epoch = base.entries()[0].satellite().epoch();
        let mut app = RadarApp::awaiting_catalogue(base.observer, base.view)
            .unwrap()
            .with_freshness(policy);
        app.update(epoch).unwrap();
        assert!(!app.data_stale()); // nothing loaded
        app.load_fetched_catalogue(app_catalogue(), epoch);
        assert_eq!(app.fetched_at(), Some(epoch));
        app.update(epoch + Duration::hours(24)).unwrap();
        assert!(!app.data_stale()); // exactly at the threshold
        app.update(epoch + Duration::hours(24) + Duration::seconds(1))
            .unwrap();
        assert!(app.data_stale());
        assert_eq!(app.counts().catalogue, 1); // still tracked
        app.invalidate_frame();
        assert!(!app.data_stale()); // no frame, no claim
        app.update(epoch + Duration::hours(30)).unwrap();
        app.load_fetched_catalogue(app_catalogue(), epoch + Duration::hours(30));
        assert!(!app.data_stale());
        // A plain load has no fetch time and is never stale.
        app.load_catalogue(app_catalogue());
        app.update(epoch + Duration::days(6)).unwrap();
        assert_eq!(app.fetched_at(), None);
        assert!(!app.data_stale());
        // Without a policy fetched data is never called stale or expired.
        let mut plain = base;
        plain.load_fetched_catalogue(app_catalogue(), epoch);
        plain.update(epoch + Duration::days(30)).unwrap();
        assert!(!plain.data_stale());
        assert_eq!(plain.counts().catalogue, 1);
    }

    #[test]
    fn expired_elements_unload_until_the_next_load() {
        let base = app();
        let epoch = base.entries()[0].satellite().epoch();
        let mut app = RadarApp::new(app_catalogue(), base.observer, base.view)
            .unwrap()
            .with_freshness(Freshness::DEVICE);
        app.update(epoch + Duration::days(7)).unwrap();
        assert_eq!(app.counts().catalogue, 1); // exactly at the threshold
        app.update(epoch + Duration::days(7) + Duration::seconds(1))
            .unwrap();
        assert_eq!(app.counts(), Counts::default());
        assert_eq!(app.selected_id(), None);
        assert_eq!(app.fetched_at(), None);
        assert!(!app.data_stale());
        // Going back in time does not resurrect them; a new load does.
        app.update(epoch).unwrap();
        assert_eq!(app.counts().catalogue, 0);
        app.load_fetched_catalogue(app_catalogue(), epoch);
        assert_eq!(app.counts().catalogue, 1);
        assert_eq!(app.selected_id(), Some(25544));
    }
}
