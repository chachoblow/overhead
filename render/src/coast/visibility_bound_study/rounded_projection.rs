//! Arithmetic audit for four short-run scenes. This encloses the *algebraic*
//! perspective ratio conditional on exact midpoint camera coordinates and the
//! stated surface derivative envelope. The stored view coefficient is enclosed
//! through a unit projection, without re-evaluating `tan`/`atan`. `libm`
//! sincos/sqrt and the camera's WGS-84 calculation still have no established
//! error bound here: NONE of these bands is a certified visibility result.
use super::*;

#[derive(Clone, Copy, Debug)]
struct Range {
    lo: f64,
    hi: f64,
}

impl Range {
    fn point(v: f64) -> Option<Self> {
        v.is_finite().then_some(Self { lo: v, hi: v })
    }

    fn between(lo: f64, hi: f64) -> Option<Self> {
        (lo.is_finite() && hi.is_finite() && lo <= hi).then_some(Self { lo, hi })
    }

    fn add(self, rhs: Self) -> Option<Self> {
        Self::between((self.lo + rhs.lo).next_down(), (self.hi + rhs.hi).next_up())
    }

    fn neg(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
        }
    }

    fn sub(self, rhs: Self) -> Option<Self> {
        self.add(rhs.neg())
    }

    fn mul(self, rhs: Self) -> Option<Self> {
        let p = [
            self.lo * rhs.lo,
            self.lo * rhs.hi,
            self.hi * rhs.lo,
            self.hi * rhs.hi,
        ];
        if p.iter().any(|v| !v.is_finite()) {
            return None;
        }
        Self::between(
            p.iter().copied().fold(f64::INFINITY, f64::min).next_down(),
            p.iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
                .next_up(),
        )
    }

    fn div(self, rhs: Self) -> Option<Self> {
        if rhs.lo <= 0.0 && rhs.hi >= 0.0 {
            return None;
        }
        let q = [
            self.lo / rhs.lo,
            self.lo / rhs.hi,
            self.hi / rhs.lo,
            self.hi / rhs.hi,
        ];
        if q.iter().any(|v| !v.is_finite()) {
            return None;
        }
        Self::between(
            q.iter().copied().fold(f64::INFINITY, f64::min).next_down(),
            q.iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
                .next_up(),
        )
    }

    fn contains(self, x: f64) -> bool {
        self.lo <= x && x <= self.hi
    }
}

// The unit projection is exactly one rounded division by View's stored
// half-span (north has no aspect factor). The exact reciprocal is bracketed
// by adjacent representable values under IEEE-754 round-to-nearest.
fn reciprocal_span(view: View) -> Option<Range> {
    let unit = view.project_unclipped(CameraPoint {
        east_km: 0.0,
        north_km: 1.0,
        depth_km: 1.0,
    })?;
    Range::between(unit.north.next_down(), unit.north.next_up())
}

#[derive(Debug)]
enum Audit {
    // Still UNKNOWN with respect to shared camera arithmetic and input
    // conversion. Never use this for a hidden/outside/pixel classification.
    Conditional { east: Range, north: Range },
    Unbounded,
}

fn audit(study: &Study<'_>, lo: f64, hi: f64) -> Audit {
    let Some((p, _)) = sample(study.camera, interpolate(study.a, study.b, (lo + hi) * 0.5)) else {
        return Audit::Unbounded;
    };
    // The derivative constants are those of the existing study. Round every
    // *algebraic* step outward, including parameter width, radians, distance
    // and the ratio denominator. Endpoint decoding/interpolation, PI, libm,
    // and these constants' derivation remain separate uncertified inputs.
    let mut dl = study.b.0 - study.a.0;
    if dl.abs() > 180.0 {
        dl -= 360.0 * dl.signum();
    }
    let Some(half_width) = Range::point(hi)
        .and_then(|hi| hi.sub(Range::point(lo)?))
        .and_then(|w| w.mul(Range::point(0.5)?))
    else {
        return Audit::Unbounded;
    };
    let Some(travel) = Range::point(dl.abs())
        .and_then(|dl| dl.add(Range::point((study.b.1 - study.a.1).abs())?))
        .and_then(|angle| angle.mul(Range::point(core::f64::consts::PI / 180.0)?))
        .and_then(|angle| angle.mul(half_width))
        .and_then(|angle| angle.mul(Range::point(7000.0)?))
        .and_then(|km| km.add(Range::point(1e-6)?))
    else {
        return Audit::Unbounded;
    };
    let Some(radius) = Range::between(-travel.hi, travel.hi) else {
        return Audit::Unbounded;
    };
    let Some(depth) = Range::point(p.depth_km).and_then(|p| p.add(radius)) else {
        return Audit::Unbounded;
    };
    if depth.lo < overhead_app::projection::NEAR_PLANE_KM {
        return Audit::Unbounded;
    }
    // Projecting a unit north/depth point computes 1 / the EXACT stored
    // binary64 half-span with one rounded division. Widen by one ulp each
    // way to enclose that reciprocal; we do not need to certify libm::tan's
    // accuracy versus the ideal angular FOV to enclose the renderer's lens.
    // The aspect input below is the same deterministic Layout ratio passed to
    // View::rectangular in the test; it is treated as an exact stored binary64.
    let Some(inv_span) = reciprocal_span(study.view) else {
        return Audit::Unbounded;
    };
    let Some((east, north)) = Range::point(p.east_km)
        .and_then(|v| v.add(radius))
        .and_then(|v| v.div(depth))
        .and_then(|v| v.mul(inv_span))
        .and_then(|v| v.div(Range::point(study.layout.aspect_ratio())?))
        .zip(
            Range::point(p.north_km)
                .and_then(|v| v.add(radius))
                .and_then(|v| v.div(depth))
                .and_then(|v| v.mul(inv_span)),
        )
    else {
        return Audit::Unbounded;
    };
    Audit::Conditional { east, north }
}

#[test]
fn outward_algebra_refuses_invalid_bounds_and_brackets_cancellation() {
    let a = Range::point(1.0).unwrap();
    let b = Range::point(3.0).unwrap();
    assert!(a.sub(b).unwrap().contains(-2.0));
    assert!(a.div(Range::between(-1.0, 1.0).unwrap()).is_none());
    assert!(Range::point(f64::INFINITY).is_none());
    assert!(Range::point(f64::MAX).unwrap().add(a).is_none());
    assert!(Range::point(0.0).unwrap().mul(b).unwrap().contains(0.0));
}

#[test]
fn stored_lens_bound_follows_zoom_without_an_atan_round_trip() {
    let layout = Layout::wide();
    for fov in [0.1, 5.0, 100.0] {
        let mut view = View::rectangular(fov, layout.aspect_ratio()).unwrap();
        for zoom in [0, 3, -7] {
            view.zoom(zoom);
            let inverse = reciprocal_span(view).unwrap();
            assert!(inverse.lo > 0.0);
            for (north, depth) in [(1.0, 1.0), (-8.0, 70_000.0), (1e-6, 72_000.0)] {
                let p = CameraPoint {
                    east_km: 0.0,
                    north_km: north,
                    depth_km: depth,
                };
                let actual = view.project_unclipped(p).unwrap().north;
                let bracket = Range::point(north)
                    .unwrap()
                    .div(Range::point(depth).unwrap())
                    .unwrap()
                    .mul(inverse)
                    .unwrap();
                assert!(bracket.contains(actual), "fov={fov} zoom={zoom}");
            }
        }
    }
}

#[test]
fn short_run_conditional_projection_audit() {
    let layout = Layout::wide();
    let view = View::rectangular(0.1, layout.aspect_ratio()).unwrap();
    for (index, t, outward) in [
        (552, 0.95, 0.004),
        (575, 0.95, 0.004),
        (621, 0.05, 0.004),
        (667, 0.95, 0.001),
    ] {
        let a = point(ASSET, index).unwrap();
        let b = point(ASSET, index + 1).unwrap();
        let origin = interpolate(a, b, t);
        let at = |offset: f64| {
            site(
                origin.1,
                (origin.0 + offset + 180.0).rem_euclid(360.0) - 180.0,
            )
        };
        let east = |offset: f64| {
            let camera = Camera::new(at(offset)).unwrap();
            view.project_unclipped(sample(&camera, origin).unwrap().0)
                .unwrap()
                .east
                .abs()
        };
        let (mut low, mut high) = (0.0, 3.0);
        assert!(east(low) < 1.0 && east(high) > 1.0);
        for _ in 0..32 {
            let m = (low + high) * 0.5;
            if east(m) < 1.0 {
                low = m;
            } else {
                high = m;
            }
        }
        let observer = at((low + high) * 0.5 + outward);
        let camera = Camera::new(observer).unwrap();
        let study = Study {
            camera: &camera,
            view,
            layout,
            a,
            b,
        };
        let (mut conditional, mut unbounded) = (0, 0);
        for i in 0..64 {
            let (lo, hi) = (i as f64 / 64.0, (i + 1) as f64 / 64.0);
            match audit(&study, lo, hi) {
                Audit::Conditional { east, north } => {
                    conditional += 1;
                    // Falsification samples, not a proof about libm or all t.
                    for j in 0..=8 {
                        let t = lo + (hi - lo) * j as f64 / 8.0;
                        let (p, _) = sample(&camera, interpolate(a, b, t)).unwrap();
                        let projected = view.project_unclipped(p).unwrap();
                        assert!(east.contains(projected.east), "segment={index}, t={t}");
                        assert!(north.contains(projected.north), "segment={index}, t={t}");
                    }
                }
                Audit::Unbounded => unbounded += 1,
            }
        }
        std::println!(
            "segment={index}: conditional_algebra_and_stored_lens={conditional}, unbounded_algebra={unbounded}, uncertified_camera_intervals=64"
        );
        assert_eq!(conditional + unbounded, 64);
        assert_eq!(unbounded, 0, "algebraic enclosure lost segment={index}");
        // No `Bound::Pixel` or invisibility conclusion follows from this audit:
        // every interval still has an unbounded camera input error.
    }
}
