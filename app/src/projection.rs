//! Presentation-only spherical orthographic ground map (Snyder 20-3/20-4).
//! Reference: https://neacsu.net/geodesy/snyder/5-azimuthal/sect_20/
//! WGS-84 geodetic latitude/longitude are mapped onto a sphere; this is not
//! a physical coordinate conversion or a surface-distance measurement.

use overhead_core::{CoordinateError, GeodeticPosition};

/// Display sphere radius (WGS-84 equatorial radius), not a physical Earth model.
pub const MAP_RADIUS_KM: f64 = 6378.137;
/// Provisional simulator zoom bounds, not production operating limits.
pub const MIN_VIEW_RADIUS_KM: f64 = 50.0;
pub const MAX_VIEW_RADIUS_KM: f64 = 20_000.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapPoint {
    pub east_km: f64,
    pub north_km: f64,
}

/// Normalized coordinates inside the circular viewport, east/right and north/up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewPoint {
    pub east: f64,
    pub north: f64,
}

pub struct GroundMap {
    sin_lat: f64,
    cos_lat: f64,
    longitude: f64,
}

fn latitude_sincos(latitude: f64) -> (f64, f64) {
    let (sin, cos) = libm::sincos(latitude);
    (
        sin,
        if latitude.abs() == core::f64::consts::FRAC_PI_2 {
            0.0
        } else {
            cos
        },
    )
}

impl GroundMap {
    pub fn new(observer: GeodeticPosition) -> Result<Self, CoordinateError> {
        observer.to_ecef()?;
        let (sin_lat, cos_lat) = latitude_sincos(observer.latitude_rad);
        Ok(Self {
            sin_lat,
            cos_lat,
            longitude: observer.longitude_rad,
        })
    }

    /// Ignores target altitude after validation. Returns None on the map's far
    /// hemisphere. This clipping is additional to, not a substitute for, the
    /// observer's true geometric horizon. Supplied longitude defines polar axes.
    pub fn project(&self, ground: GeodeticPosition) -> Result<Option<MapPoint>, CoordinateError> {
        ground.to_ecef()?;
        let (sin_lat, cos_lat) = latitude_sincos(ground.latitude_rad);
        let (sin_lon, cos_lon) = libm::sincos(ground.longitude_rad - self.longitude);
        let facing = self.sin_lat * sin_lat + self.cos_lat * cos_lat * cos_lon;
        if facing < 0.0 {
            return Ok(None);
        }
        Ok(Some(MapPoint {
            east_km: MAP_RADIUS_KM * cos_lat * sin_lon,
            north_km: MAP_RADIUS_KM * (self.cos_lat * sin_lat - self.sin_lat * cos_lat * cos_lon),
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    radius_km: f64,
}

impl View {
    pub fn new(radius_km: f64) -> Result<Self, &'static str> {
        if !radius_km.is_finite() || !(MIN_VIEW_RADIUS_KM..=MAX_VIEW_RADIUS_KM).contains(&radius_km)
        {
            return Err("view radius must be in [50, 20000] projected km");
        }
        Ok(Self { radius_km })
    }

    pub fn radius_km(self) -> f64 {
        self.radius_km
    }

    /// Positive steps zoom in. Clamp input before exponentiation and radius
    /// after scaling; a large wheel event cannot produce infinity or wraparound.
    pub fn zoom(&mut self, steps: i32) {
        let factor = libm::pow(1.2, f64::from(steps.clamp(-64, 64)));
        self.radius_km = (self.radius_km / factor).clamp(MIN_VIEW_RADIUS_KM, MAX_VIEW_RADIUS_KM);
    }

    /// Clip by marker center, including equality. Renderer clips the glyph too.
    pub fn locate(self, point: MapPoint) -> Option<ViewPoint> {
        let distance = libm::hypot(point.east_km, point.north_km);
        if !distance.is_finite() || distance > self.radius_km {
            return None;
        }
        Some(ViewPoint {
            east: point.east_km / self.radius_km,
            north: point.north_km / self.radius_km,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn geo(lat: f64, lon: f64) -> GeodeticPosition {
        GeodeticPosition {
            latitude_rad: lat.to_radians(),
            longitude_rad: lon.to_radians(),
            altitude_km: 0.0,
        }
    }
    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-8, "{a} != {b}");
    }

    #[test]
    fn center_cardinals_limb_and_far_side() {
        let map = GroundMap::new(geo(0.0, 0.0)).unwrap();
        let center = map.project(geo(0.0, 0.0)).unwrap().unwrap();
        near(center.east_km, 0.0);
        near(center.north_km, 0.0);
        for (lat, lon, x, y) in [
            (0.0, 30.0, 0.5, 0.0),
            (0.0, -30.0, -0.5, 0.0),
            (30.0, 0.0, 0.0, 0.5),
            (-30.0, 0.0, 0.0, -0.5),
            (0.0, 90.0, 1.0, 0.0),
        ] {
            let p = map.project(geo(lat, lon)).unwrap().unwrap();
            near(p.east_km, x * MAP_RADIUS_KM);
            near(p.north_km, y * MAP_RADIUS_KM);
        }
        assert!(map.project(geo(0.0, 90.001)).unwrap().is_none());
        assert!(map.project(geo(0.0, 180.0)).unwrap().is_none());
    }

    #[test]
    fn dateline_poles_and_altitude_do_not_move_ground_track() {
        let map = GroundMap::new(geo(0.0, 179.0)).unwrap();
        let mut target = geo(0.0, -179.0);
        let point = map.project(target).unwrap().unwrap();
        near(
            point.east_km,
            MAP_RADIUS_KM * libm::sin(2.0_f64.to_radians()),
        );
        target.altitude_km = 40_000.0;
        assert_eq!(map.project(target).unwrap().unwrap(), point);
        for pole in [-90.0, 90.0] {
            let map = GroundMap::new(geo(pole, 0.0)).unwrap();
            let p = map.project(geo(pole, 123.0)).unwrap().unwrap();
            near(p.east_km, 0.0);
            near(p.north_km, 0.0);
            let p = map
                .project(geo(pole.signum() * 60.0, 90.0))
                .unwrap()
                .unwrap();
            near(p.east_km, MAP_RADIUS_KM * 0.5);
            near(p.north_km, 0.0);
        }
    }

    #[test]
    fn oblique_projection_matches_cartesian_basis() {
        // Independent vector construction: unit ground vector dotted into the
        // observer's east/north basis (not copied scalar projection equations).
        let site = geo(39.007, -104.883);
        let map = GroundMap::new(site).unwrap();
        let unit = |g: GeodeticPosition| {
            [
                libm::cos(g.latitude_rad) * libm::cos(g.longitude_rad),
                libm::cos(g.latitude_rad) * libm::sin(g.longitude_rad),
                libm::sin(g.latitude_rad),
            ]
        };
        let up = unit(site);
        let east = [
            -libm::sin(site.longitude_rad),
            libm::cos(site.longitude_rad),
            0.0,
        ];
        let north = [
            up[1] * east[2] - up[2] * east[1],
            up[2] * east[0] - up[0] * east[2],
            up[0] * east[1] - up[1] * east[0],
        ];
        let dot = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).map(|(a, b)| a * b).sum::<f64>();
        for lat in [-80.0, -30.0, 0.0, 40.0, 85.0] {
            for lon in [-170.0, -100.0, 0.0, 90.0, 180.0] {
                let target = geo(lat, lon);
                let v = unit(target);
                match map.project(target).unwrap() {
                    Some(p) => {
                        assert!(dot(up, v) >= 0.0);
                        near(p.east_km, dot(east, v) * MAP_RADIUS_KM);
                        near(p.north_km, dot(north, v) * MAP_RADIUS_KM);
                    }
                    None => assert!(dot(up, v) < 0.0),
                }
            }
        }
    }

    #[test]
    fn validation_clipping_and_zoom_limits() {
        assert!(GroundMap::new(geo(91.0, 0.0)).is_err());
        assert!(
            GroundMap::new(geo(0.0, 0.0))
                .unwrap()
                .project(geo(f64::NAN, 0.0))
                .is_err()
        );
        for r in [0.0, -1.0, f64::NAN, f64::INFINITY, 20_001.0] {
            assert!(View::new(r).is_err());
        }
        let mut view = View::new(100.0).unwrap();
        assert!(
            view.locate(MapPoint {
                east_km: 60.0,
                north_km: 80.0
            })
            .is_some()
        );
        assert!(
            view.locate(MapPoint {
                east_km: 60.0,
                north_km: 80.001
            })
            .is_none()
        );
        assert!(
            view.locate(MapPoint {
                east_km: f64::NAN,
                north_km: 0.0
            })
            .is_none()
        );
        view.zoom(i32::MAX);
        assert_eq!(view.radius_km(), MIN_VIEW_RADIUS_KM);
        view.zoom(i32::MIN);
        assert_eq!(view.radius_km(), MAX_VIEW_RADIUS_KM);
        assert!(view.radius_km() > MAP_RADIUS_KM);
    }
}
