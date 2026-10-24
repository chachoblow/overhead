//! Measurement-only extraction of the accepted host trial, NOT production.
//! Packed quadratic ECEF curves, f32 traversal, analytic facing roots and
//! screen-space flattening. Approximation/rounding choices are experimental.
use core::{convert::Infallible, ops::Range};
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle},
};
use overhead_app::projection::{Camera, CameraPoint, View};
use overhead_core::{EcefPosition, GeodeticPosition};

mod decode;
#[cfg(test)]
mod packing;

const A: f64 = 6378.137;
const B: f64 = A * (1.0 - 1.0 / 298.257_223_563);
const FLAT_PX: f32 = 0.25;
const DEPTH: usize = 16;
const NODE_LIMIT: usize = 65_536;
type V3 = [f32; 3];
type Curve = [V3; 3];

fn mix(a: V3, b: V3, t: f32) -> V3 {
    core::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}
fn split(q: Curve, t: f32) -> (Curve, Curve) {
    let a = mix(q[0], q[1], t);
    let b = mix(q[1], q[2], t);
    let m = mix(a, b, t);
    ([q[0], a, m], [m, b, q[2]])
}
#[cfg(test)]
fn curve_at(q: Curve, t: f32) -> V3 {
    mix(mix(q[0], q[1], t), mix(q[1], q[2], t), t)
}
fn subcurve(q: Curve, lo: f32, hi: f32) -> Curve {
    let left = if hi == 1.0 { q } else { split(q, hi).0 };
    if lo == 0.0 {
        left
    } else {
        split(left, lo / hi).1
    }
}
fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

struct Context {
    rows: [V3; 3],
    offset: V3,
    facing: V3,
    lens: [f32; 2],
}
impl Context {
    fn new(site: GeodeticPosition, view: View) -> Self {
        // Constant setup uses the existing f64 camera (including its setup
        // trigonometry). No shared camera or tracking arithmetic is changed.
        let camera = Camera::new(site).unwrap();
        let xyz = |p: CameraPoint| [p.east_km, p.north_km, p.depth_km];
        let origin = xyz(camera.project(EcefPosition::from_km([0.0; 3])).unwrap());
        let mut rows = [[0.0_f64; 3]; 3];
        for axis in 0..3 {
            let mut p = [0.0; 3];
            p[axis] = 8192.0;
            let projected = xyz(camera.project(EcefPosition::from_km(p)).unwrap());
            for row in 0..3 {
                rows[row][axis] = (projected[row] - origin[row]) / 8192.0;
            }
        }
        // Orthogonal camera transform maps the eye to zero. On the ellipsoid,
        // normal-facing sign is C·diag(1/a²,1/a²,1/b²)P - 1.
        // Applying that plane to the APPROXIMATE curve is a model choice, not
        // proof the quadratic lies exactly on the ellipsoid.
        let eye: [f64; 3] =
            core::array::from_fn(|i| -(0..3).map(|j| rows[j][i] * origin[j]).sum::<f64>());
        let scale = view
            .project_unclipped(CameraPoint {
                east_km: 1.0,
                north_km: 1.0,
                depth_km: 1.0,
            })
            .unwrap();
        Self {
            rows: rows.map(|r| r.map(|x| x as f32)),
            offset: origin.map(|x| x as f32),
            facing: [
                (eye[0] / (A * A)) as f32,
                (eye[1] / (A * A)) as f32,
                (eye[2] / (B * B)) as f32,
            ],
            lens: [scale.east as f32, scale.north as f32],
        }
    }
    fn transform(&self, p: V3) -> V3 {
        core::array::from_fn(|i| dot(self.rows[i], p) + self.offset[i])
    }
    fn front(&self, p: V3) -> f32 {
        dot(self.facing, p) - 1.0
    }
    fn project(&self, p: V3) -> [f32; 2] {
        [p[0] / p[2] * self.lens[0], p[1] / p[2] * self.lens[1]]
    }
}

#[derive(Default, Debug, PartialEq)]
pub struct Counts {
    pub pieces: usize,
    pub hidden: usize,
    pub root_curves: usize,
    pub transforms: usize,
    pub nodes: usize,
    pub outside: usize,
    pub splits: usize,
    pub strokes: usize,
    pub peak_pending: usize,
}

/// Inventory only; excludes other live locals/callers and is not a stack bound.
pub const fn pending_bytes() -> usize {
    core::mem::size_of::<[Pending; DEPTH]>()
}

// Interior roots of a quadratic in Bernstein form. Stable q formulation;
// all-f32 rounding remains empirical, not outward-rounded certification.
fn cuts(f: [f32; 3]) -> ([f32; 4], usize) {
    let a = f[0] - 2.0 * f[1] + f[2];
    let b = 2.0 * (f[1] - f[0]);
    let c = f[0];
    let mut result = [0.0, 1.0, 1.0, 1.0];
    let mut n = 2;
    let mut add = |t: f32| {
        if t.is_finite() && t > 0.0 && t < 1.0 && !result[..n].contains(&t) {
            result[n] = t;
            n += 1;
        }
    };
    if a == 0.0 {
        if b != 0.0 {
            add(-c / b);
        }
    } else {
        let discriminant = b * b - 4.0 * a * c;
        if discriminant >= 0.0 {
            let q = -0.5 * (b + libm::sqrtf(discriminant).copysign(b));
            if q == 0.0 {
                add(-b / (2.0 * a));
            } else {
                add(q / a);
                add(c / q);
            }
        }
    }
    result[..n].sort_unstable_by(f32::total_cmp);
    (result, n)
}

fn clip_fast(a: [f32; 2], b: [f32; 2]) -> Option<([f32; 2], [f32; 2])> {
    let delta = [b[0] - a[0], b[1] - a[1]];
    let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
    for (p, q) in [
        (-delta[0], a[0] + 1.0),
        (delta[0], 1.0 - a[0]),
        (-delta[1], a[1] + 1.0),
        (delta[1], 1.0 - a[1]),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else if p < 0.0 {
            lo = lo.max(q / p);
        } else {
            hi = hi.min(q / p);
        }
        if lo > hi {
            return None;
        }
    }
    Some((
        core::array::from_fn(|i| a[i] + lo * delta[i]),
        core::array::from_fn(|i| a[i] + hi * delta[i]),
    ))
}

fn flat(p: [[f32; 2]; 3]) -> bool {
    let scale = [173.0, 117.0];
    let delta: [f32; 2] = core::array::from_fn(|i| (p[2][i] - p[0][i]) * scale[i]);
    let control: [f32; 2] = core::array::from_fn(|i| (p[1][i] - p[0][i]) * scale[i]);
    let length2 = delta[0] * delta[0] + delta[1] * delta[1];
    let t = if length2 == 0.0 {
        0.0
    } else {
        ((control[0] * delta[0] + control[1] * delta[1]) / length2).clamp(0.0, 1.0)
    };
    let d = [control[0] - t * delta[0], control[1] - t * delta[1]];
    d[0] * d[0] + d[1] * d[1] <= FLAT_PX * FLAT_PX
}

#[derive(Clone, Copy)]
struct Pending {
    q: Curve,
    depth: usize,
}

fn flatten<D: DrawTarget<Color = BinaryColor, Error = Infallible>>(
    screen: &mut D,
    context: &Context,
    curve: Curve,
    counts: &mut Counts,
    depth_limit: usize,
) -> Result<(), &'static str> {
    let mut stack = [Pending {
        q: [[0.0; 3]; 3],
        depth: 0,
    }; DEPTH];
    let mut len = 0;
    let mut current = Pending { q: curve, depth: 0 };
    loop {
        counts.nodes += 1;
        if counts.nodes > NODE_LIMIT {
            return Err("node limit");
        }
        if current
            .q
            .iter()
            .any(|p| p[2] < 1.0 || p.iter().any(|v| !v.is_finite()))
        {
            return Err("invalid camera curve");
        }
        let p = current.q.map(|p| context.project(p));
        if p.iter().flatten().any(|x| !x.is_finite()) {
            return Err("invalid projection");
        }
        // Positive-depth rational Bézier projection stays in its control hull.
        // Whole-hull rejection cannot discard a crossing with outside endpoints.
        let outside =
            (0..2).any(|axis| p.iter().all(|p| p[axis] < -1.0) || p.iter().all(|p| p[axis] > 1.0));
        if outside {
            counts.outside += 1;
        } else if flat(p) {
            if let Some((a, b)) = clip_fast(p[0], p[2]) {
                let pixel = |p: [f32; 2]| {
                    let round = |x: f32| (x + if x < 0.0 { -0.5 } else { 0.5 }) as i32;
                    Point::new(
                        176 + round(p[0].clamp(-1.0, 1.0) * 173.0),
                        120 - round(p[1].clamp(-1.0, 1.0) * 117.0),
                    )
                };
                counts.strokes += 1;
                Line::new(pixel(a), pixel(b))
                    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                    .draw(screen)
                    .unwrap();
            }
        } else {
            if current.depth >= depth_limit || len == DEPTH {
                return Err("refinement limit");
            }
            let (left, right) = split(current.q, 0.5);
            stack[len] = Pending {
                q: right,
                depth: current.depth + 1,
            };
            len += 1;
            counts.peak_pending = counts.peak_pending.max(len);
            counts.splits += 1;
            current = Pending {
                q: left,
                depth: current.depth + 1,
            };
            continue;
        }
        if len == 0 {
            break;
        }
        len -= 1;
        current = stack[len];
    }
    Ok(())
}

fn render_piece<D: DrawTarget<Color = BinaryColor, Error = Infallible>>(
    screen: &mut D,
    context: &Context,
    q: Curve,
    counts: &mut Counts,
) -> Result<(), &'static str> {
    counts.pieces += 1;
    let f = q.map(|p| context.front(p));
    if f.iter().any(|x| !x.is_finite()) {
        return Err("invalid facing");
    }
    if f.iter().all(|x| *x <= 0.0) {
        counts.hidden += 1;
        return Ok(());
    }
    counts.transforms += 3;
    let camera = q.map(|p| context.transform(p));
    if f.iter().all(|x| *x > 0.0) {
        return flatten(screen, context, camera, counts, DEPTH);
    }
    counts.root_curves += 1;
    let (cuts, n) = cuts(f);
    for pair in cuts[..n].windows(2) {
        let t = 0.5 * (pair[0] + pair[1]);
        // Explicit square: the no_std f32 API has no powi. Same operation order.
        let s = 1.0 - t;
        let facing = s * s * f[0] + 2.0 * t * (1.0 - t) * f[1] + t * t * f[2];
        if facing > 0.0 {
            flatten(
                screen,
                context,
                subcurve(camera, pair[0], pair[1]),
                counts,
                DEPTH,
            )?;
        }
    }
    Ok(())
}

fn render_range<D: DrawTarget<Color = BinaryColor, Error = Infallible>>(
    screen: &mut D,
    context: &Context,
    bytes: &[u8],
    range: Range<usize>,
    counts: &mut Counts,
) -> Result<(), &'static str> {
    for at in (range.start..range.end - 9).step_by(15) {
        render_piece(screen, context, decode::decode_fast(bytes, at), counts)?;
    }
    Ok(())
}

pub fn candidate_draw<D: DrawTarget<Color = BinaryColor, Error = Infallible>>(
    screen: &mut D,
    site: GeodeticPosition,
    view: View,
    bytes: &[u8],
) -> Result<Counts, &'static str> {
    // Includes structure validation and camera setup. All fitting/precomputation
    // is offline. No prepared per-observer state is hidden outside this timer.
    if !decode::checked(bytes) {
        return Err("invalid packed structure");
    }
    let context = Context::new(site, view);
    let mut counts = Counts::default();
    for part in 0..decode::PARTS {
        render_range(
            screen,
            &context,
            bytes,
            decode::part_range(bytes, part),
            &mut counts,
        )?;
    }
    Ok(counts)
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use crate::{Layout, coast::*};
    use cost_preflight::Screen;
    use std::{hint::black_box, time::Instant, vec::Vec};

    fn baseline_segment(screen: &mut Screen, camera: &Camera, view: View, a: Geo, b: Geo) {
        let n = steps(a, b);
        for k in 0..n {
            assert!(
                interval(
                    screen,
                    camera,
                    view,
                    Layout::wide(),
                    interpolate(a, b, k as f64 / n as f64),
                    interpolate(a, b, (k + 1) as f64 / n as f64)
                )
                .unwrap()
            );
        }
    }

    fn lit(screen: &Screen) -> Vec<(i32, i32)> {
        screen
            .pixels
            .iter()
            .enumerate()
            .flat_map(|(y, row)| {
                row.iter()
                    .enumerate()
                    .filter_map(move |(x, &v)| (v != 0).then_some((x as i32, y as i32)))
            })
            .collect()
    }

    #[derive(Default)]
    struct Difference {
        changed: usize,
        beyond1: usize,
        beyond3: usize,
        maximum: usize,
    }

    // Bidirectional, per-source-segment comparison. Adjacent geographic runs cannot
    // hide loss. Empty opposite masks count as distance 400, never as a match.
    fn difference(a: &Screen, b: &Screen) -> Difference {
        let pa = lit(a);
        let pb = lit(b);
        let mut d = Difference::default();
        for (points, other, other_points) in [(&pa, b, &pb), (&pb, a, &pa)] {
            for &(x, y) in points {
                if other.pixels[y as usize][x as usize] != 0 {
                    continue;
                }
                d.changed += 1;
                let near = (-1..=1).any(|dy| {
                    (-1..=1).any(|dx| {
                        let (x, y) = (x + dx, y + dy);
                        (0..400).contains(&x)
                            && (0..240).contains(&y)
                            && other.pixels[y as usize][x as usize] != 0
                    })
                });
                let distance = if near {
                    1
                } else {
                    other_points
                        .iter()
                        .map(|&(ox, oy)| (x - ox).abs().max((y - oy).abs()) as usize)
                        .min()
                        .unwrap_or(400)
                };
                d.maximum = d.maximum.max(distance);
                d.beyond1 += usize::from(distance > 1);
                d.beyond3 += usize::from(distance > 3);
            }
        }
        d
    }

    fn snapshot(name: &str, screen: &Screen) {
        // Optional disposable/retained review output; no device file I/O path.
        if let Ok(dir) = std::env::var("OVERHEAD_COAST_REP_OUTPUT") {
            std::fs::create_dir_all(&dir).unwrap();
            let mut bytes = b"P5\n400 240\n255\n".to_vec();
            bytes.extend(
                screen
                    .pixels
                    .iter()
                    .flatten()
                    .map(|v| if *v == 0 { 255 } else { 0 }),
            );
            std::fs::write(
                std::path::Path::new(&dir).join(std::format!("{name}.pgm")),
                bytes,
            )
            .unwrap();
        }
    }

    #[test]
    fn representation_roots_and_subdivision_do_not_silently_drop_unknown_work() {
        let (roots, n) = cuts([-1.0, 3.0, -1.0]);
        assert_eq!(n, 4); // two hidden endpoints enclose a visible interior
        assert!(roots[1] > 0.0 && roots[2] < 1.0);
        let (tangent, n) = cuts([-1.0, 1.0, -1.0]);
        assert_eq!(&tangent[..n], &[0.0, 0.5, 1.0]);
        assert_eq!(cuts([0.0, 0.0, 0.0]).1, 2);
        let context = Context::new(cost_preflight::observer(0.0, 0.0), View::new(20.0).unwrap());
        let curve = [
            [-100.0, 0.0, 75000.0],
            [0.0, 1000.0, 75000.0],
            [100.0, 0.0, 75000.0],
        ];
        assert_eq!(
            flatten(
                &mut Screen::new(),
                &context,
                curve,
                &mut Counts::default(),
                0
            ),
            Err("refinement limit")
        );
        assert_eq!(
            flatten(
                &mut Screen::new(),
                &context,
                curve,
                &mut Counts {
                    nodes: NODE_LIMIT,
                    ..Counts::default()
                },
                DEPTH
            ),
            Err("node limit")
        );
        // Viewport crossing survives even though both projected ends are outside.
        let mut screen = Screen::new();
        flatten(
            &mut screen,
            &context,
            [
                [-50000.0, 0.0, 75000.0],
                [0.0, 0.0, 75000.0],
                [50000.0, 0.0, 75000.0],
            ],
            &mut Counts::default(),
            DEPTH,
        )
        .unwrap();
        assert!(screen.on_writes > 300);
    }

    #[test]
    fn representation_crossings_dateline_poles_height_and_part_breaks() {
        for (lat, lon, height, fov, a, b) in [
            (0.0, 0.0, 0.0, 0.1, (-2.0, 0.0), (2.0, 0.0)),
            (0.0, 180.0, 0.0, 0.1, (179.0, 0.0), (-179.0, 0.0)),
            (90.0, 0.0, 0.0, 0.1, (0.0, 89.0), (180.0, 89.0)),
            (0.0, 0.0, 0.0, 20.0, (80.0, 0.0), (90.0, 0.0)),
            (0.0, 0.0, 0.0, 20.0, (-90.0, 0.0), (90.0, 0.0)),
            (
                37.7749,
                -122.4194,
                2.187,
                0.1,
                (-123.0, 37.7),
                (-122.0, 37.7),
            ),
        ] {
            let site = GeodeticPosition {
                altitude_km: height,
                ..cost_preflight::observer(lat, lon)
            };
            let view = View::rectangular(fov, Layout::wide().aspect_ratio()).unwrap();
            let context = Context::new(site, view);
            let bytes = packing::prepare_segment(a, b);
            let mut actual = Screen::new();
            let mut baseline = Screen::new();
            render_range(
                &mut actual,
                &context,
                &bytes,
                0..bytes.len(),
                &mut Counts::default(),
            )
            .unwrap();
            baseline_segment(&mut baseline, &Camera::new(site).unwrap(), view, a, b);
            let d = difference(&baseline, &actual);
            assert_eq!(
                d.beyond1, 0,
                "synthetic case {lat}/{lon}/{fov}: {}",
                d.maximum
            );
            assert!(actual.on_writes > 0, "vacuous synthetic case");
        }
        let site = cost_preflight::observer(37.7749, -122.4194);
        let view = View::rectangular(0.1, Layout::wide().aspect_ratio()).unwrap();
        let context = Context::new(site, view);
        let mut separate = Screen::new();
        for (a, b) in [
            ((-123.0, 37.7749), (-122.6, 37.7749)),
            ((-122.2, 37.7749), (-121.9, 37.7749)),
        ] {
            let bytes = packing::prepare_segment(a, b);
            render_range(
                &mut separate,
                &context,
                &bytes,
                0..bytes.len(),
                &mut Counts::default(),
            )
            .unwrap();
        }
        assert_eq!(
            separate.pixels[120][176], 0,
            "part break must not draw a bridge"
        );
        assert!(separate.on_writes > 0);
    }

    #[test]
    fn representation_camera_plane_matches_surface_facing() {
        for site in [
            cost_preflight::observer(0.0, 0.0),
            cost_preflight::observer(89.9, 0.0),
            GeodeticPosition {
                altitude_km: 2.187,
                ..cost_preflight::observer(39.007, -104.883)
            },
        ] {
            let camera = Camera::new(site).unwrap();
            let context = Context::new(site, View::new(20.0).unwrap());
            for i in 0..POINTS {
                let geo = point(ASSET, i).unwrap();
                let world = packing::world(geo).map(|v| v as f32);
                let (p, f) = sample(&camera, geo).unwrap();
                let fast = context.transform(world);
                assert!((f64::from(fast[0]) - p.east_km).abs() < 0.003);
                assert!((f64::from(fast[1]) - p.north_km).abs() < 0.003);
                assert!((f64::from(fast[2]) - p.depth_km).abs() < 0.015);
                if f.abs() > 0.1 {
                    assert_eq!(context.front(world) > 0.0, f > 0.0);
                }
            }
        }
    }

    fn tight_sweep(packed: &packing::Packed) {
        let view = View::rectangular(0.1, Layout::wide().aspect_ratio()).unwrap();
        let mut reference = Screen::new();
        let mut candidate = Screen::new();
        let mut changed = 0;
        let mut beyond1 = 0;
        let mut maximum = 0;
        let mut peak_pending = 0;
        let mut scenes = 0;
        let mut visible = 0;
        for span in &packed.spans {
            let middle = interpolate(span.a, span.b, 0.5);
            // Every original segment at its own midpoint: 1734 isolated tight
            // scenes, plus stratified nonzero-height controls. Not whole frames or
            // all observers; this specifically stresses local curvature/precision.
            for height in [0.0, 2.187] {
                if height != 0.0 && span.index % 23 != 0 {
                    continue;
                }
                let site = GeodeticPosition {
                    altitude_km: height,
                    ..cost_preflight::observer(middle.1, middle.0)
                };
                reference.pixels.fill([0; 400]);
                candidate.pixels.fill([0; 400]);
                baseline_segment(
                    &mut reference,
                    &Camera::new(site).unwrap(),
                    view,
                    span.a,
                    span.b,
                );
                let mut counts = Counts::default();
                render_range(
                    &mut candidate,
                    &Context::new(site, view),
                    &packed.bytes,
                    span.bytes.clone(),
                    &mut counts,
                )
                .unwrap();
                let d = difference(&reference, &candidate);
                if d.maximum > 1 {
                    std::println!(
                        "tight_difference,segment={},height={height},beyond1={},distance={}",
                        span.index,
                        d.beyond1,
                        d.maximum
                    );
                }
                changed += d.changed;
                beyond1 += d.beyond1;
                maximum = maximum.max(d.maximum);
                peak_pending = peak_pending.max(counts.peak_pending);
                scenes += 1;
                visible += usize::from(reference.pixels.iter().flatten().any(|v| *v != 0));
            }
        }
        std::println!(
            "tight_sweep,scenes={scenes},visible={visible},changed={changed},beyond1={beyond1},maximum={maximum},pending={peak_pending}"
        );
    }

    #[test]
    #[ignore = "explicit host representation experiment; not S3 or visual acceptance"]
    fn coast_representation_spike() {
        let packed = packing::prepare();
        let (error, chord) = packing::representation_error(&packed);
        std::println!(
            "representation,bytes={},pieces={},source_segments={},sampled_error_km={error:.6},chord_error_km={chord:.6},pending_bytes={}",
            packed.bytes.len(),
            packed.pieces.len(),
            packed.spans.len(),
            pending_bytes()
        );
        if let Ok(dir) = std::env::var("OVERHEAD_COAST_REP_OUTPUT") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                std::path::Path::new(&dir).join("quadratic-trial.bin"),
                &packed.bytes,
            )
            .unwrap();
        }
        tight_sweep(&packed);
        std::println!(
            "case,pieces,hidden,root_curves,transforms,nodes,outside,splits,strokes,pending,on_writes,isolated_changed,isolated_beyond1,isolated_beyond3,isolated_max,whole_changed,whole_max,projected_error_px,baseline_us,candidate_us"
        );
        for (name, site, fov) in work_review::cases() {
            let view = View::rectangular(fov, Layout::wide().aspect_ratio()).unwrap();
            let camera = Camera::new(site).unwrap();
            let context = Context::new(site, view);
            let mut total = Difference::default();
            let mut reference = Screen::new();
            let mut candidate = Screen::new();
            let mut worst = 0;
            for span in &packed.spans {
                reference.pixels.fill([0; 400]);
                candidate.pixels.fill([0; 400]);
                baseline_segment(&mut reference, &camera, view, span.a, span.b);
                render_range(
                    &mut candidate,
                    &context,
                    &packed.bytes,
                    span.bytes.clone(),
                    &mut Counts::default(),
                )
                .unwrap();
                let d = difference(&reference, &candidate);
                if d.maximum > total.maximum || (d.maximum == total.maximum && d.changed > worst) {
                    worst = d.changed;
                    snapshot(&std::format!("{name}-isolated-reference"), &reference);
                    snapshot(&std::format!("{name}-isolated-candidate"), &candidate);
                    std::println!(
                        "worst,{name},segment={},distance={},changed={}",
                        span.index,
                        d.maximum,
                        d.changed
                    );
                }
                total.changed += d.changed;
                total.beyond1 += d.beyond1;
                total.beyond3 += d.beyond3;
                total.maximum = total.maximum.max(d.maximum);
            }
            let mut baseline = Screen::new();
            assert!(draw(&mut baseline, site, view, Layout::wide(), ASSET).unwrap());
            let mut candidate = Screen::new();
            let expected = candidate_draw(&mut candidate, site, view, &packed.bytes).unwrap();
            snapshot(&std::format!("{name}-reference"), &baseline);
            snapshot(&std::format!("{name}-candidate"), &candidate);
            let whole = difference(&baseline, &candidate);
            let projected_error = packing::projected_error(&packed, site, view);
            let mut baseline_us = [0.0_f64; 5];
            let mut candidate_us = [0.0_f64; 5];
            for i in 0..5 {
                let mut run = |fast: bool| {
                    let mut screen = Screen::new(); // blanking outside timer
                    let start = Instant::now();
                    if fast {
                        let counts = candidate_draw(
                            black_box(&mut screen),
                            black_box(site),
                            black_box(view),
                            black_box(&packed.bytes),
                        )
                        .unwrap();
                        candidate_us[i] = start.elapsed().as_secs_f64() * 1e6;
                        assert_eq!(counts, expected);
                        assert_eq!(screen.pixels, candidate.pixels);
                    } else {
                        assert!(
                            draw(
                                black_box(&mut screen),
                                black_box(site),
                                black_box(view),
                                Layout::wide(),
                                black_box(ASSET)
                            )
                            .unwrap()
                        );
                        baseline_us[i] = start.elapsed().as_secs_f64() * 1e6;
                        assert_eq!(screen.pixels, baseline.pixels);
                    }
                    black_box(screen);
                };
                run(i % 2 == 0);
                run(i % 2 != 0);
            }
            baseline_us.sort_by(f64::total_cmp);
            candidate_us.sort_by(f64::total_cmp);
            std::println!(
                "{name},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{:.1},{:.1}",
                expected.pieces,
                expected.hidden,
                expected.root_curves,
                expected.transforms,
                expected.nodes,
                expected.outside,
                expected.splits,
                expected.strokes,
                expected.peak_pending,
                candidate.on_writes,
                total.changed,
                total.beyond1,
                total.beyond3,
                total.maximum,
                whole.changed,
                whole.maximum,
                projected_error,
                baseline_us[2],
                candidate_us[2]
            );
        }
    }
}
