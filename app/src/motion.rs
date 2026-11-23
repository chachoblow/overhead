//! Presentation state for 0089's motion cues, owned by the host beside the
//! app and kept out of both [`RadarApp`]'s true state and the renderer:
//!
//! - **Trails:** past positions of every entry, recorded from the app's own
//!   samples (no extra propagation) in two rings: one snapshot per 20 s for
//!   the last ~4.5 min and one per 2 min for the last ~30 min, the longest
//!   four-step trail the lens allows. Each point keeps its own sample UTC, so
//!   staggered frames (0077) interpolate exactly.
//! - **Selected orbit:** one period centred on now, as TEME points at fixed
//!   time slots, propagated a caller-limited number per frame (nearest now
//!   first) and rolled forward one slot at a time as now advances. Drawn in
//!   the Earth's orientation of the last few updates (re-projected a share of
//!   points per update), so the selected marker sits on it.
//!
//! Both answer only for the frame they were last updated with (same UTC,
//! catalogue generation and, for the orbit, selection); otherwise they yield
//! nothing, never stale geometry.
use alloc::vec::Vec;
use overhead_core::{
    EcefPosition, TemePosition,
    sgp4::chrono::{NaiveDateTime, TimeDelta},
    teme_to_ecef,
};

use crate::{
    RadarApp,
    projection::{CameraPoint, MAX_FOV_DEG, ViewPoint},
};

/// Steps behind each marker (0089).
pub const TRAIL_STEPS: usize = 4;
/// Orbit line segments over one period (radar lab: 160).
pub const ORBIT_SEGMENTS: usize = 160;

/// Seconds per trail step at a lens (radar lab `trailDt`): one minute at 20°,
/// scaling with the half-angle tangent, i.e. with the screen's size in km.
pub fn trail_step_s(fov_deg: f64) -> f64 {
    (60.0 * libm::tan(fov_deg.to_radians() / 2.0) / libm::tan(10f64.to_radians())).clamp(1.0, 900.0)
}

/// (snapshot spacing s, snapshots kept). Interpolating between snapshots of
/// spacing S misplaces a LEO point by ~0.001·S² km, under a pixel at every
/// lens for these tiers; the coarse tier reaches the 100° lens's trail.
const TIERS: [(i64, usize); 2] = [(20, 14), (120, 16)];

#[derive(Clone, Copy)]
struct Point {
    /// ECEF km; NaN where the entry's sample had failed.
    km: [f32; 3],
    /// Sample UTC relative to the snapshot's base, ms (staggering: <= 0).
    offset_ms: i32,
}

/// One snapshot ring: `bases[k]` and `points[k * n..][..n]` per slot.
struct Tier {
    step: TimeDelta,
    len: usize,
    bases: Vec<NaiveDateTime>,
    points: Vec<Point>,
    /// Slot of the newest snapshot; `count` valid snapshots end there.
    head: usize,
    count: usize,
}

impl Tier {
    fn new((step_s, len): (i64, usize)) -> Self {
        Self {
            step: TimeDelta::seconds(step_s),
            len,
            bases: Vec::new(),
            points: Vec::new(),
            head: 0,
            count: 0,
        }
    }

    fn clear(&mut self) {
        self.count = 0;
    }

    /// Snapshot `k` back from the newest (0 = newest).
    fn slot(&self, k: usize) -> usize {
        (self.head + self.len - k) % self.len
    }

    fn record(&mut self, app: &RadarApp, now: NaiveDateTime) {
        // A step back (a small clock resync) drops only the snapshots now in
        // the future; a gap would interpolate across missing samples.
        while self.count > 0 && self.bases[self.head] > now {
            self.head = self.slot(1);
            self.count -= 1;
        }
        if self.count > 0 {
            let since = now - self.bases[self.head];
            if since < self.step {
                return;
            }
            if since > self.step * 2 {
                self.clear();
            }
        }
        let n = app.samples.len();
        // `bases` too: with no entries `points` is legitimately empty, but
        // the snapshot still needs its slots (a frame before any catalogue).
        if self.bases.len() != self.len || self.points.len() != self.len * n {
            self.bases = alloc::vec![now; self.len];
            self.points = alloc::vec![
                Point {
                    km: [f32::NAN; 3],
                    offset_ms: 0
                };
                self.len * n
            ];
            self.count = 0;
        }
        self.head = if self.count == 0 {
            0
        } else {
            (self.head + 1) % self.len
        };
        self.count = (self.count + 1).min(self.len);
        self.bases[self.head] = now;
        let times = app.sample_times();
        for (i, result) in app.samples.iter().enumerate() {
            let offset = (times[i] - now).num_milliseconds();
            self.points[self.head * n + i] = match (result, i32::try_from(offset)) {
                (Ok(s), Ok(offset_ms)) => Point {
                    km: s.physical.ecef.km().map(|v| v as f32),
                    offset_ms,
                },
                _ => Point {
                    km: [f32::NAN; 3],
                    offset_ms: 0,
                },
            };
        }
    }

    /// Entry `i`'s snapshots, newest first, as (UTC, km); failures skipped.
    fn history(&self, i: usize, n: usize) -> impl Iterator<Item = (NaiveDateTime, [f64; 3])> + '_ {
        (0..self.count).filter_map(move |k| {
            let slot = self.slot(k);
            let p = self.points.get(slot * n + i)?;
            let km = p.km.map(f64::from);
            km.iter().all(|v| v.is_finite()).then(|| {
                (
                    self.bases[slot] + TimeDelta::milliseconds(i64::from(p.offset_ms)),
                    km,
                )
            })
        })
    }
}

#[derive(Clone, Copy)]
struct TrailCameras {
    at: NaiveDateTime,
    step: TimeDelta,
    points: [Option<CameraPoint>; TRAIL_STEPS],
}

/// Trail step at the app's current lens, whole milliseconds.
fn trail_step(app: &RadarApp) -> TimeDelta {
    TimeDelta::milliseconds(libm::round(trail_step_s(app.view().fov_deg()) * 1000.0) as i64)
}

/// A trail: `points[0]` is the marker, then up to [`TRAIL_STEPS`] past
/// positions, in normalized (unclipped) viewport coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trail {
    pub points: [ViewPoint; TRAIL_STEPS + 1],
    pub len: usize,
}

impl Trail {
    pub fn points(&self) -> &[ViewPoint] {
        &self.points[..self.len]
    }
}

#[derive(Clone, Copy)]
struct OrbitSlot {
    slot: i64,
    /// TEME km; `None` where propagation failed (not retried).
    teme: Option<[f64; 3]>,
    /// Camera-relative position in the Earth's orientation of a recent
    /// update; `None` where hidden by the Earth or unavailable. Cached
    /// because soft-float projection of every point each frame cost ~10 ms
    /// on the S3; refreshed [`ORBIT_REFRESH`] slots per update.
    camera: Option<CameraPoint>,
}

/// Orbit slots re-projected per update, so the whole line follows the
/// Earth's rotation within ~8 frames (well under a pixel at any lens).
const ORBIT_REFRESH: usize = 20;

/// TEME→ECEF rotation at `now`, as the images of the TEME x and y axes
/// (z is unchanged).
fn rotation(now: NaiveDateTime) -> Option<([f64; 3], [f64; 3])> {
    let x = teme_to_ecef(TemePosition::from_km([1.0, 0.0, 0.0]), now).ok()?;
    let y = teme_to_ecef(TemePosition::from_km([0.0, 1.0, 0.0]), now).ok()?;
    Some((x.km(), y.km()))
}

fn orbit_camera(
    app: &RadarApp,
    (ex, ey): ([f64; 3], [f64; 3]),
    [x, y, z]: [f64; 3],
) -> Option<CameraPoint> {
    let ecef = EcefPosition::from_km([ex[0] * x + ey[0] * y, ex[1] * x + ey[1] * y, z]);
    if app.camera().hidden_by_earth(ecef) {
        return None;
    }
    app.camera().project(ecef).ok()
}

struct Orbit {
    /// Selected NORAD ID and catalogue generation the points belong to.
    owner: Option<(u64, u64)>,
    /// Slot `s` is UTC `t0 + s * step`.
    t0: NaiveDateTime,
    step: TimeDelta,
    ring: Vec<OrbitSlot>,
    /// Slot nearest the last update's UTC.
    centre: i64,
    /// Next ring index to re-project.
    refresh: usize,
}

impl Orbit {
    const HALF: i64 = (ORBIT_SEGMENTS / 2) as i64;

    fn clear(&mut self) {
        self.owner = None;
    }

    fn index(slot: i64) -> usize {
        slot.rem_euclid(ORBIT_SEGMENTS as i64 + 1) as usize
    }

    /// The window's slots, nearest the centre first.
    fn by_priority(centre: i64) -> impl Iterator<Item = i64> {
        (0..=Self::HALF).flat_map(move |d| {
            let pair = [centre + d, centre - d];
            pair.into_iter().take(if d == 0 { 1 } else { 2 })
        })
    }

    /// Propagates at most `budget` points; returns how many it did.
    fn update(&mut self, app: &RadarApp, now: NaiveDateTime, budget: usize) -> usize {
        let (Some((entry, _)), Some(period)) = (app.selected(), app.selected_period_minutes())
        else {
            self.clear();
            return 0;
        };
        let owner = (entry.elements().norad_id, app.catalogue_generation());
        let step_ms = libm::round(period * 60_000.0 / ORBIT_SEGMENTS as f64);
        if self.owner != Some(owner) {
            // Periods are minutes to days; anything else has no usable orbit.
            if !(1.0..=1e10).contains(&step_ms) {
                self.clear();
                return 0;
            }
            self.owner = Some(owner);
            self.t0 = now;
            self.step = TimeDelta::milliseconds(step_ms as i64);
            self.ring.clear();
            self.ring.resize(
                ORBIT_SEGMENTS + 1,
                OrbitSlot {
                    slot: i64::MIN,
                    teme: None,
                    camera: None,
                },
            );
        }
        let Some(rotation) = rotation(now) else {
            self.clear();
            return 0;
        };
        let elapsed = (now - self.t0).num_milliseconds() as f64;
        self.centre = libm::round(elapsed / self.step.num_milliseconds() as f64) as i64;
        let mut done = 0;
        for slot in Self::by_priority(self.centre) {
            if done == budget {
                break;
            }
            let k = Self::index(slot);
            if self.ring[k].slot == slot {
                continue;
            }
            let at = self.t0 + self.step * slot as i32;
            let teme = entry
                .satellite()
                .state_at(at)
                .ok()
                .map(|s| s.position().km());
            self.ring[k] = OrbitSlot {
                slot,
                teme,
                camera: teme.and_then(|p| orbit_camera(app, rotation, p)),
            };
            done += 1;
        }
        for _ in 0..ORBIT_REFRESH {
            let k = self.refresh % self.ring.len();
            if let Some(p) = self.ring[k].teme {
                self.ring[k].camera = orbit_camera(app, rotation, p);
            }
            self.refresh = (k + 1) % self.ring.len();
        }
        done
    }
}

/// Trail history and selected orbit; see the module docs.
pub struct Motion {
    tiers: [Tier; 2],
    /// Per entry: the camera points of its last computed trail, keyed by
    /// the sample UTC and step it was computed for. Recomputed in `update`
    /// only for in-view markers whose sample or lens step changed (~1 a
    /// frame under 0077's stagger), not for every trail every frame.
    trail_cache: Vec<Option<TrailCameras>>,
    orbit: Orbit,
    /// Frame UTC and catalogue generation of the last update.
    frame: Option<(NaiveDateTime, u64)>,
}

impl Default for Motion {
    fn default() -> Self {
        Self::new()
    }
}

impl Motion {
    pub fn new() -> Self {
        Self {
            tiers: TIERS.map(Tier::new),
            trail_cache: Vec::new(),
            orbit: Orbit {
                owner: None,
                t0: NaiveDateTime::MIN,
                step: TimeDelta::zero(),
                ring: Vec::new(),
                centre: 0,
                refresh: 0,
            },
            frame: None,
        }
    }

    /// Follow the app's current frame: record a snapshot when one is due and
    /// propagate at most `orbit_budget` orbit points. Returns the number of
    /// propagations. Without a frame everything is dropped; a new catalogue
    /// drops the history.
    pub fn update(&mut self, app: &RadarApp, orbit_budget: usize) -> usize {
        let Some(now) = app.time() else {
            self.frame = None;
            self.tiers.iter_mut().for_each(Tier::clear);
            self.orbit.clear();
            return 0;
        };
        let generation = app.catalogue_generation();
        let n = app.samples.len();
        if self.frame.is_none_or(|(_, g)| g != generation) || self.trail_cache.len() != n {
            self.tiers.iter_mut().for_each(Tier::clear);
            self.trail_cache.clear();
            self.trail_cache.resize(n, None);
        }
        self.frame = Some((now, generation));
        for tier in &mut self.tiers {
            tier.record(app, now);
        }
        let step = trail_step(app);
        for i in self.trail_heads(app).map(|(i, _)| i).collect::<Vec<_>>() {
            let at = app.sample_times()[i];
            if self.trail_cache[i].is_none_or(|c| c.at != at || c.step != step) {
                self.trail_cache[i] = Some(TrailCameras {
                    at,
                    step,
                    points: self.trail_cameras(app, i, at, step),
                });
            }
        }
        self.orbit.update(app, now, orbit_budget)
    }

    /// Unselected in-view above-horizon markers, by entry index.
    fn trail_heads<'a>(&self, app: &'a RadarApp) -> impl Iterator<Item = (usize, ViewPoint)> + 'a {
        let view = app.view();
        let selected = app.selected_id();
        app.objects()
            .enumerate()
            .filter_map(move |(i, (entry, result))| {
                if Some(entry.elements().norad_id) == selected {
                    return None;
                }
                Some((i, result.as_ref().ok()?.marker(view)?))
            })
    }

    /// Camera points of entry `i`'s trail steps, `None` from the first
    /// missing or Earth-hidden one on.
    fn trail_cameras(
        &self,
        app: &RadarApp,
        i: usize,
        at: NaiveDateTime,
        step: TimeDelta,
    ) -> [Option<CameraPoint>; TRAIL_STEPS] {
        let mut points = [None; TRAIL_STEPS];
        for (k, point) in points.iter_mut().enumerate() {
            let Some(km) = self.past(app, i, at - step * (k as i32 + 1)) else {
                break;
            };
            let ecef = EcefPosition::from_km(km);
            if app.camera().hidden_by_earth(ecef) {
                break;
            }
            let Ok(camera) = app.camera().project(ecef) else {
                break;
            };
            *point = Some(camera);
        }
        points
    }

    fn current(&self, app: &RadarApp) -> bool {
        app.time()
            .is_some_and(|t| self.frame == Some((t, app.catalogue_generation())))
    }

    /// True when every slot of the selected orbit's window is propagated.
    pub fn orbit_complete(&self, app: &RadarApp) -> bool {
        self.orbit_current(app)
            && Orbit::by_priority(self.orbit.centre)
                .all(|s| self.orbit.ring[Orbit::index(s)].slot == s)
    }

    fn orbit_current(&self, app: &RadarApp) -> bool {
        self.current(app)
            && app
                .selected_id()
                .is_some_and(|id| self.orbit.owner == Some((id, app.catalogue_generation())))
    }

    /// Entry `i`'s ECEF km at `t`, interpolated between its current sample
    /// and stored snapshots, newest first. Each pair spans at most two of the
    /// older point's snapshot steps.
    fn past(&self, app: &RadarApp, i: usize, t: NaiveDateTime) -> Option<[f64; 3]> {
        let n = app.samples.len();
        let sample = app.samples.get(i)?.as_ref().ok()?;
        let mut newer = (app.sample_times()[i], sample.physical.ecef.km());
        let fine = &self.tiers[0];
        let fine_oldest = fine.history(i, n).last().map(|(t, _)| t);
        let chain = fine.history(i, n).map(|p| (p, fine.step)).chain(
            self.tiers[1]
                .history(i, n)
                .filter(move |(t, _)| fine_oldest.is_none_or(|f| *t < f))
                .map(|p| (p, self.tiers[1].step)),
        );
        for (older, step) in chain {
            if older.0 > newer.0 {
                continue;
            }
            if newer.0 - older.0 > step * 2 {
                return None;
            }
            if older.0 <= t && t <= newer.0 {
                let span = (newer.0 - older.0).num_milliseconds();
                if span == 0 {
                    return Some(newer.1);
                }
                let f = (t - older.0).num_milliseconds() as f64 / span as f64;
                return Some(core::array::from_fn(|j| {
                    older.1[j] + (newer.1[j] - older.1[j]) * f
                }));
            }
            newer = older;
        }
        None
    }

    /// Trails behind each unselected in-view above-horizon marker (0089),
    /// cut at the first missing or Earth-hidden past point. Empty unless the
    /// state follows the app's current frame.
    pub fn trails<'a>(&'a self, app: &'a RadarApp) -> impl Iterator<Item = Trail> + 'a {
        let current = self.current(app);
        let view = app.view();
        let step = trail_step(app);
        self.trail_heads(app)
            .filter(move |_| current)
            .filter_map(move |(i, head)| {
                let at = app.sample_times()[i];
                // A lens or selection change since the update: compute here.
                let cameras = match self.trail_cache.get(i).copied().flatten() {
                    Some(c) if c.at == at && c.step == step => c.points,
                    _ => self.trail_cameras(app, i, at, step),
                };
                let mut trail = Trail {
                    points: [head; TRAIL_STEPS + 1],
                    len: 1,
                };
                for camera in cameras {
                    let Some(point) = camera.and_then(|c| view.project_unclipped(c)) else {
                        break;
                    };
                    trail.points[trail.len] = point;
                    trail.len += 1;
                }
                (trail.len > 1).then_some(trail)
            })
    }

    /// The selected orbit over one period centred on now, in time order, in
    /// normalized (unclipped) viewport coordinates; `None` where a point is
    /// not yet propagated, failed, is hidden by the Earth or cannot be
    /// projected (the line breaks there). Empty unless the state follows the
    /// current frame and selection.
    pub fn orbit<'a>(&'a self, app: &'a RadarApp) -> impl Iterator<Item = Option<ViewPoint>> + 'a {
        let centre = self.orbit.centre;
        let view = app.view();
        let slots = if self.orbit_current(app) {
            centre - Orbit::HALF..=centre + Orbit::HALF
        } else {
            #[allow(clippy::reversed_empty_ranges)]
            {
                1..=0
            }
        };
        slots.map(move |slot| {
            let entry = self.orbit.ring[Orbit::index(slot)];
            view.project_unclipped(entry.camera.filter(|_| entry.slot == slot)?)
        })
    }
}

/// Snapshot span of the coarse tier must reach the widest lens's trail.
const _: () = assert!(TIERS[1].0 * (TIERS[1].1 as i64 - 1) >= 1620 + 120);
const _: () = assert!(MAX_FOV_DEG == 100.0);

#[cfg(test)]
#[path = "motion_tests.rs"]
mod tests;
