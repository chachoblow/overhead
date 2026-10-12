//! Fixed, north-up pinhole camera above the observer; lens zoom only.
//! Satellite ECEF positions and the WGS-84 limb share this camera. This is
//! presentation geometry, not a replacement for true observer look angles.

use overhead_core::{CoordinateError, EcefPosition, GeodeticPosition};

/// Along the observer's geodetic local up, relative to the observer, not MSL.
pub const CAMERA_HEIGHT_KM: f64 = 75_000.0;
pub const NEAR_PLANE_KM: f64 = 1.0;
/// Full angle across the viewport width/height; provisional simulator settings.
pub const DEFAULT_FOV_DEG: f64 = 20.0;
pub const MIN_FOV_DEG: f64 = 0.1;
pub const MAX_FOV_DEG: f64 = 100.0;
// Same WGS-84 ellipsoid as core. Deliberately no change to the verified engine.
const EARTH_A_KM: f64 = 6378.137;
const EARTH_B_KM: f64 = EARTH_A_KM * (1.0 - 1.0 / 298.257_223_563);

type Vector = [f64; 3];
fn dot(a: Vector, b: Vector) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn finite(v: Vector) -> Result<Vector, CoordinateError> {
    if v.iter().all(|x| x.is_finite()) {
        Ok(v)
    } else {
        Err(CoordinateError::NonFinite)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraError {
    Coordinates(CoordinateError),
    /// Extreme observer heights can put Earth across/behind the near plane or
    /// make its projected ellipse numerically unrepresentable. Never hide this.
    InvalidEarthView,
}
impl From<CoordinateError> for CameraError {
    fn from(value: CoordinateError) -> Self {
        Self::Coordinates(value)
    }
}
impl core::fmt::Display for CameraError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Coordinates(e) => e.fmt(f),
            Self::InvalidEarthView => f.write_str("observer height cannot form a finite Earth view wholly beyond the camera near plane"),
        }
    }
}

/// Camera-relative kilometres. Depth increases along the viewing direction.
/// Cached independently of FOV and of the immutable physical sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraPoint {
    pub east_km: f64,
    pub north_km: f64,
    pub depth_km: f64,
}

/// Normalized viewport coordinates, east/right and north/up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewPoint {
    pub east: f64,
    pub north: f64,
}

pub struct Camera {
    observer: Vector,
    east: Vector,
    north: Vector,
    up: Vector,
    limb: EarthLimb,
}

impl Camera {
    pub fn new(observer: GeodeticPosition) -> Result<Self, CameraError> {
        let site = observer.to_ecef()?.km();
        let (sin_lat, mut cos_lat) = libm::sincos(observer.latitude_rad);
        if observer.latitude_rad.abs() == core::f64::consts::FRAC_PI_2 {
            cos_lat = 0.0;
        }
        let (sin_lon, cos_lon) = libm::sincos(observer.longitude_rad);
        let east = [-sin_lon, cos_lon, 0.0];
        let north = [-sin_lat * cos_lon, -sin_lat * sin_lon, cos_lat];
        let up = [cos_lat * cos_lon, cos_lat * sin_lon, sin_lat];
        let position = finite(core::array::from_fn(|i| site[i] + CAMERA_HEIGHT_KM * up[i]))?;
        let limb = EarthLimb::from_camera(position, east, north, up)?;
        Ok(Self {
            observer: site,
            east,
            north,
            up,
            limb,
        })
    }

    /// Actual ECEF position, not a geodetic footprint. Clipping is deferred to
    /// View so valid behind-camera points aren't reported as physical failures.
    pub fn project(&self, target: EcefPosition) -> Result<CameraPoint, CoordinateError> {
        let target = finite(target.km())?;
        let delta = finite(core::array::from_fn(|i| target[i] - self.observer[i]))?;
        let [east_km, north_km, depth_km] = finite([
            dot(delta, self.east),
            dot(delta, self.north),
            CAMERA_HEIGHT_KM - dot(delta, self.up),
        ])?;
        Ok(CameraPoint {
            east_km,
            north_km,
            depth_km,
        })
    }

    pub fn earth_limb(&self, view: View) -> EarthLimb {
        EarthLimb {
            north_center: self.limb.north_center / view.half_span,
            east_radius: self.limb.east_radius / view.half_span,
            north_radius: self.limb.north_radius / view.half_span,
        }
    }
}

/// Exact projected ellipsoid outline, before rasterization. Its east center
/// is zero by rotational symmetry; the north center need not be the aim point.
/// Coordinates returned by Camera::earth_limb are normalized viewport units.
#[derive(Debug, Clone, Copy)]
pub struct EarthLimb {
    north_center: f64,
    east_radius: f64,
    north_radius: f64,
}
impl EarthLimb {
    fn from_camera(
        c: Vector,
        east: Vector,
        north: Vector,
        up: Vector,
    ) -> Result<Self, CameraError> {
        // Require the whole ellipsoid beyond the same near plane as markers.
        let support = libm::sqrt(
            EARTH_A_KM * EARTH_A_KM * (up[0] * up[0] + up[1] * up[1])
                + EARTH_B_KM * EARTH_B_KM * up[2] * up[2],
        );
        if dot(c, up) - support < NEAR_PLANE_KM {
            return Err(CameraError::InvalidEarthView);
        }
        // Scale ECEF to a unit sphere. Ray c+t*(east*x+north*y-up) is
        // tangent when (c.d)^2 - (c.c-1)*(d.d) = 0. North-up and the
        // ellipsoid's rotational symmetry eliminate x and xy terms.
        // Ray/sphere quadratic reference: pbr-book.org/4ed/Shapes/Spheres.
        let scale = |v: Vector| [v[0] / EARTH_A_KM, v[1] / EARTH_A_KM, v[2] / EARTH_B_KM];
        let (c, e, n, u) = (scale(c), scale(east), scale(north), scale(up));
        let k = dot(c, c) - 1.0;
        let cn = dot(c, n);
        let cu = dot(c, u);
        let xx = -k * dot(e, e);
        let yy = cn * cn - k * dot(n, n);
        let y = 2.0 * (k * dot(n, u) - cn * cu);
        let constant = cu * cu - k * dot(u, u);
        let north_center = -y / (2.0 * yy);
        let level = constant - yy * north_center * north_center;
        let east_radius = libm::sqrt(-level / xx);
        let north_radius = libm::sqrt(-level / yy);
        if ![north_center, east_radius, north_radius]
            .iter()
            .all(|v| v.is_finite())
            || east_radius <= 0.0
            || north_radius <= 0.0
            || xx >= 0.0
            || yy >= 0.0
        {
            return Err(CameraError::InvalidEarthView);
        }
        Ok(Self {
            north_center,
            east_radius,
            north_radius,
        })
    }

    /// East coordinates at a row; paired with north_at for gap-free outline
    /// rasterization bounded by viewport width/height, even at extreme zoom.
    pub fn east_at(self, north: f64) -> Option<[f64; 2]> {
        let y = (north - self.north_center) / self.north_radius;
        if !y.is_finite() || y.abs() > 1.0 {
            return None;
        }
        let x = self.east_radius * libm::sqrt((1.0 - y * y).max(0.0));
        Some([-x, x])
    }

    pub fn north_at(self, east: f64) -> Option<[f64; 2]> {
        let x = east / self.east_radius;
        if !x.is_finite() || x.abs() > 1.0 {
            return None;
        }
        let y = self.north_radius * libm::sqrt((1.0 - x * x).max(0.0));
        Some([self.north_center - y, self.north_center + y])
    }
}

/// Borderless square is the current UI default. The circular comparison modes
/// share clipping; Square admits corners at the same horizontal/vertical scale.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Viewport {
    Circle,
    BorderlessCircle,
    #[default]
    Square,
}
impl Viewport {
    pub fn contains(self, point: ViewPoint) -> bool {
        if !point.east.is_finite() || !point.north.is_finite() {
            return false;
        }
        match self {
            Self::Circle | Self::BorderlessCircle => libm::hypot(point.east, point.north) <= 1.0,
            Self::Square => point.east.abs() <= 1.0 && point.north.abs() <= 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    // tan(full FOV / 2), allowing geometric magnification steps, not linear
    // degree steps. x_screen = (east/depth) / half_span.
    half_span: f64,
    viewport: Viewport,
}
impl View {
    pub fn new(fov_deg: f64) -> Result<Self, &'static str> {
        if !fov_deg.is_finite() || !(MIN_FOV_DEG..=MAX_FOV_DEG).contains(&fov_deg) {
            return Err("field of view must be in [0.1, 100] degrees");
        }
        Ok(Self {
            half_span: libm::tan(fov_deg.to_radians() / 2.0),
            viewport: Viewport::default(),
        })
    }
    pub fn with_viewport(mut self, viewport: Viewport) -> Self {
        self.viewport = viewport;
        self
    }
    pub fn viewport(self) -> Viewport {
        self.viewport
    }
    pub fn fov_deg(self) -> f64 {
        (2.0 * libm::atan(self.half_span)).to_degrees()
    }
    /// Positive steps magnify 1.2x, without translating the camera. Clamp input
    /// before exponentiation so even an extreme wheel event stays finite.
    pub fn zoom(&mut self, steps: i32) {
        let factor = libm::pow(1.2, f64::from(steps.clamp(-64, 64)));
        self.half_span = (self.half_span / factor).clamp(
            libm::tan(MIN_FOV_DEG.to_radians() / 2.0),
            libm::tan(MAX_FOV_DEG.to_radians() / 2.0),
        );
    }
    /// Include near-plane equality and marker centers on the viewport edge.
    /// Glyph pixels are additionally clipped by the renderer. No far plane.
    pub fn locate(self, point: CameraPoint) -> Option<ViewPoint> {
        if ![point.east_km, point.north_km, point.depth_km]
            .iter()
            .all(|v| v.is_finite())
            || point.depth_km < NEAR_PLANE_KM
        {
            return None;
        }
        let east = (point.east_km / point.depth_km) / self.half_span;
        let north = (point.north_km / point.depth_km) / self.half_span;
        let point = ViewPoint { east, north };
        self.viewport.contains(point).then_some(point)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn geo(lat: f64, lon: f64, height: f64) -> GeodeticPosition {
        GeodeticPosition {
            latitude_rad: lat.to_radians(),
            longitude_rad: lon.to_radians(),
            altitude_km: height,
        }
    }
    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }
    fn target(camera: &Camera, east: f64, north: f64, up: f64) -> EcefPosition {
        EcefPosition::from_km(core::array::from_fn(|i| {
            camera.observer[i] + east * camera.east[i] + north * camera.north[i] + up * camera.up[i]
        }))
    }

    #[test]
    fn aim_cardinals_dateline_and_polar_frames() {
        for site in [
            geo(0.0, 0.0, 0.0),
            geo(39.007, -104.883, 2.187),
            geo(0.0, 180.0, -1.0),
            geo(90.0, 123.0, 0.0),
            geo(-90.0, -70.0, 0.0),
        ] {
            let camera = Camera::new(site).unwrap();
            let view = View::new(DEFAULT_FOV_DEG).unwrap();
            for (e, n, u) in [
                (0.0, 0.0, 0.0),
                (100.0, 0.0, 400.0),
                (-100.0, 0.0, 400.0),
                (0.0, 100.0, 400.0),
                (0.0, -100.0, 400.0),
            ] {
                let p = camera.project(target(&camera, e, n, u)).unwrap();
                near(p.east_km, e);
                near(p.north_km, n);
                near(p.depth_km, CAMERA_HEIGHT_KM - u);
                let screen = view.locate(p).unwrap();
                near(screen.east, e / (CAMERA_HEIGHT_KM - u) / view.half_span);
                near(screen.north, n / (CAMERA_HEIGHT_KM - u) / view.half_span);
            }
        }
        let a = Camera::new(geo(0.0, 180.0, 0.0)).unwrap();
        let b = Camera::new(geo(0.0, -180.0, 0.0)).unwrap();
        let p = geo(1.0, -179.0, 1000.0).to_ecef().unwrap();
        near(a.project(p).unwrap().east_km, b.project(p).unwrap().east_km);
    }

    #[test]
    fn altitude_changes_projection_and_markers_can_clear_the_limb() {
        let camera = Camera::new(geo(0.0, 0.0, 0.0)).unwrap();
        let view = View::new(60.0).unwrap();
        let low = view
            .locate(
                camera
                    .project(geo(0.0, 10.0, 400.0).to_ecef().unwrap())
                    .unwrap(),
            )
            .unwrap();
        let high = view
            .locate(
                camera
                    .project(geo(0.0, 10.0, 20_200.0).to_ecef().unwrap())
                    .unwrap(),
            )
            .unwrap();
        assert!(high.east > low.east * 4.0);
        let outside = target(&camera, 12_000.0, 0.0, 20_000.0);
        assert!(
            overhead_core::ecef_to_look_angles(outside, geo(0.0, 0.0, 0.0))
                .unwrap()
                .elevation_rad
                > 0.0
        );
        let marker = view.locate(camera.project(outside).unwrap()).unwrap();
        assert!(marker.east > camera.earth_limb(view).east_at(0.0).unwrap()[1]);
        // Directly overhead objects still share the center, regardless of height.
        for altitude in [400.0, 20_200.0, 35_786.0] {
            let marker = view
                .locate(camera.project(target(&camera, 0.0, 0.0, altitude)).unwrap())
                .unwrap();
            near(marker.east, 0.0);
            near(marker.north, 0.0);
        }
    }

    #[test]
    fn lens_zoom_scales_satellites_and_earth_without_changing_depth() {
        let camera = Camera::new(geo(39.0, -105.0, 2.0)).unwrap();
        let point = camera
            .project(target(&camera, 100.0, 200.0, 20_000.0))
            .unwrap();
        let mut view = View::new(DEFAULT_FOV_DEG).unwrap();
        let before = view.locate(point).unwrap();
        let limb = camera.earth_limb(view);
        view.zoom(1);
        let after = view.locate(point).unwrap();
        near(after.east, before.east * 1.2);
        near(after.north, before.north * 1.2);
        near(camera.earth_limb(view).east_radius, limb.east_radius * 1.2);
        near(
            camera.earth_limb(view).north_center,
            limb.north_center * 1.2,
        );
        view.zoom(-1);
        near(view.fov_deg(), DEFAULT_FOV_DEG);
        view.zoom(i32::MAX);
        near(view.fov_deg(), MIN_FOV_DEG);
        view.zoom(i32::MIN);
        near(view.fov_deg(), MAX_FOV_DEG);
        let wide = camera.earth_limb(view);
        assert!((0.065..0.067).contains(&wide.east_radius));
    }

    #[test]
    fn comparison_viewports_preserve_lens_scale_and_clip_corners() {
        assert_eq!(
            View::new(DEFAULT_FOV_DEG).unwrap().viewport(),
            Viewport::Square
        );
        for fov in [MIN_FOV_DEG, DEFAULT_FOV_DEG, MAX_FOV_DEG] {
            let circle = View::new(fov).unwrap().with_viewport(Viewport::Circle);
            let borderless = circle.with_viewport(Viewport::BorderlessCircle);
            let square = circle.with_viewport(Viewport::Square);
            for (east, north) in [
                (0.0, 0.0),
                (1.0, 0.0),
                (0.8, 0.8),
                (-1.0, -1.0),
                (1.001, 0.0),
            ] {
                let point = CameraPoint {
                    east_km: east * circle.half_span,
                    north_km: north * circle.half_span,
                    depth_km: 1.0,
                };
                assert_eq!(circle.locate(point), borderless.locate(point));
                assert_eq!(
                    square.locate(point).is_some(),
                    east.abs() <= 1.0 && north.abs() <= 1.0
                );
                if let Some(p) = circle.locate(point) {
                    assert_eq!(square.locate(point), Some(p));
                }
                for view in [circle, borderless, square] {
                    assert!(
                        view.locate(CameraPoint {
                            depth_km: 0.999,
                            ..point
                        })
                        .is_none()
                    );
                    assert!(
                        view.locate(CameraPoint {
                            east_km: f64::NAN,
                            ..point
                        })
                        .is_none()
                    );
                    assert!(
                        view.locate(CameraPoint {
                            north_km: f64::INFINITY,
                            ..point
                        })
                        .is_none()
                    );
                    let mut zoomed = view;
                    zoomed.zoom(1);
                    assert_eq!(zoomed.viewport(), view.viewport());
                    near(view.fov_deg(), fov);
                }
            }
        }
    }

    #[test]
    fn circular_near_plane_behind_camera_edges_and_nonfinite_inputs() {
        let camera = Camera::new(geo(0.0, 0.0, 0.0)).unwrap();
        for fov in [MIN_FOV_DEG, DEFAULT_FOV_DEG, MAX_FOV_DEG] {
            let view = View::new(fov).unwrap().with_viewport(Viewport::Circle);
            for depth in [-1000.0, 0.0, 0.999] {
                let p = camera
                    .project(target(&camera, 0.0, 0.0, CAMERA_HEIGHT_KM - depth))
                    .unwrap();
                assert!(view.locate(p).is_none());
            }
            assert!(
                view.locate(CameraPoint {
                    east_km: 0.0,
                    north_km: 0.0,
                    depth_km: NEAR_PLANE_KM
                })
                .is_some()
            );
            let edge = CameraPoint {
                east_km: view.half_span,
                north_km: 0.0,
                depth_km: 1.0,
            };
            assert!(view.locate(edge).is_some());
            // Circular clipping, not merely a square frustum.
            assert!(
                view.locate(CameraPoint {
                    east_km: edge.east_km * 0.6,
                    north_km: edge.east_km * 0.7,
                    ..edge
                })
                .is_some()
            );
            assert!(
                view.locate(CameraPoint {
                    east_km: edge.east_km * 0.8,
                    north_km: edge.east_km * 0.8,
                    ..edge
                })
                .is_none()
            );
            assert!(
                view.locate(CameraPoint {
                    east_km: edge.east_km * 1.000001,
                    ..edge
                })
                .is_none()
            );
            for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                for p in [
                    CameraPoint {
                        east_km: bad,
                        ..edge
                    },
                    CameraPoint {
                        north_km: bad,
                        ..edge
                    },
                    CameraPoint {
                        depth_km: bad,
                        ..edge
                    },
                ] {
                    assert!(view.locate(p).is_none());
                }
                assert_eq!(
                    camera.project(EcefPosition::from_km([bad, 0.0, 0.0])),
                    Err(CoordinateError::NonFinite)
                );
            }
        }
        for fov in [-1.0, 0.0, 0.099, 100.01, f64::NAN, f64::INFINITY] {
            assert!(View::new(fov).is_err());
        }
        assert!(Camera::new(geo(91.0, 0.0, 0.0)).is_err());
        assert!(Camera::new(geo(0.0, 181.0, 0.0)).is_err());
        assert!(Camera::new(geo(0.0, 0.0, -CAMERA_HEIGHT_KM)).is_err());
        assert!(Camera::new(geo(0.0, 0.0, f64::MAX)).is_err());
    }

    #[test]
    fn ellipsoid_limb_matches_analytic_axis_views_and_core_radii() {
        near(geo(0.0, 0.0, 0.0).to_ecef().unwrap().km()[0], EARTH_A_KM);
        near(geo(90.0, 0.0, 0.0).to_ecef().unwrap().km()[2], EARTH_B_KM);
        let equator = Camera::new(geo(0.0, 0.0, 0.0)).unwrap().limb;
        let distance = CAMERA_HEIGHT_KM + EARTH_A_KM;
        let divisor = libm::sqrt(distance * distance - EARTH_A_KM * EARTH_A_KM);
        near(equator.east_radius, EARTH_A_KM / divisor);
        near(equator.north_radius, EARTH_B_KM / divisor);
        near(equator.north_center, 0.0);
        let polar = Camera::new(geo(90.0, 123.0, 0.0)).unwrap().limb;
        let distance = CAMERA_HEIGHT_KM + EARTH_B_KM;
        let radius = EARTH_A_KM / libm::sqrt(distance * distance - EARTH_B_KM * EARTH_B_KM);
        near(polar.east_radius, radius);
        near(polar.north_radius, radius);
        assert!(equator.east_at(f64::NAN).is_none());
        assert!(equator.north_at(f64::INFINITY).is_none());
    }

    #[test]
    fn oblique_limb_rays_are_tangent_to_the_physical_ellipsoid() {
        // Independent 3D check: closest point along each scaled camera ray
        // lies on the unit sphere, and the ray is perpendicular to its normal.
        for lat in [-89.9, -39.0, 0.0, 39.0, 89.9] {
            let camera = Camera::new(geo(lat, 127.0, 2.187)).unwrap();
            let c: Vector =
                core::array::from_fn(|i| camera.observer[i] + CAMERA_HEIGHT_KM * camera.up[i]);
            let scale = |v: Vector| [v[0] / EARTH_A_KM, v[1] / EARTH_A_KM, v[2] / EARTH_B_KM];
            let c = scale(c);
            for fov in [0.1, 20.0, 100.0] {
                let view = View::new(fov).unwrap();
                let limb = camera.earth_limb(view);
                for step in 0..72 {
                    let angle = f64::from(step) * core::f64::consts::TAU / 72.0;
                    let x = limb.east_radius * libm::cos(angle);
                    let y = limb.north_center + limb.north_radius * libm::sin(angle);
                    let d = scale(core::array::from_fn(|i| {
                        view.half_span * (camera.east[i] * x + camera.north[i] * y) - camera.up[i]
                    }));
                    let t = -dot(c, d) / dot(d, d);
                    assert!(t > NEAR_PLANE_KM);
                    let contact: Vector = core::array::from_fn(|i| c[i] + t * d[i]);
                    near(dot(contact, contact), 1.0);
                    near(dot(contact, d), 0.0);
                }
            }
        }
    }
}
