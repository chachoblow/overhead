//! Host-only observer-space feasibility experiment. Not a device representation,
//! cache API, UI implementation, or allocation policy. Keeps complete visible
//! camera curves, not a bitmap; drawing is lens-dependent but site-independent.
extern crate std;

use super::*;
use crate::{Layout, coast::cost_preflight::Screen, coast::work_review};
use core::ops::Range;
use std::{hint::black_box, time::Instant, vec::Vec};

mod memory;

struct Prepared {
    site: GeodeticPosition,
    context: Context,
    visible: Vec<Curve>,
    source_pieces: usize,
    peak_capacity: usize,
}

// Quantized-camera storage experiment. At this scale, the 75,000 km camera
// depth is stored as an offset. Every conversion is checked before publishing.
// This is a host Vec, NOT a device allocation or an approved error tolerance.
const CAMERA_BIAS: f32 = overhead_app::projection::CAMERA_HEIGHT_KM as f32;
const STEP_KM: f32 = 0.25;
type ShortCurve = [[i16; 3]; 3];

struct Compact {
    site: GeodeticPosition,
    context: Context,
    visible: Vec<ShortCurve>,
}

impl Compact {
    fn new(source: &Prepared) -> Result<Self, &'static str> {
        let mut visible = Vec::with_capacity(source.visible.len());
        for q in &source.visible {
            let mut short = [[0_i16; 3]; 3];
            for (p, packed) in q.iter().zip(short.iter_mut()) {
                for i in 0..3 {
                    let value = (p[i] - if i == 2 { CAMERA_BIAS } else { 0.0 }) / STEP_KM;
                    if !value.is_finite() || value < i16::MIN as f32 || value > i16::MAX as f32 {
                        return Err("camera coordinate outside trial encoding");
                    }
                    packed[i] = value.round() as i16;
                }
            }
            visible.push(short);
        }
        Ok(Self {
            site: source.site,
            context: source.context,
            visible,
        })
    }

    fn draw<D: DrawTarget<Color = BinaryColor, Error = Infallible>>(
        &self,
        target: &mut D,
        site: GeodeticPosition,
        view: View,
    ) -> Result<Counts, &'static str> {
        if self.site != site {
            return Err("observer changed");
        }
        let context = self.context.with_view(view);
        let mut counts = Counts::default();
        for q in &self.visible {
            let curve = q.map(|p| {
                core::array::from_fn(|i| {
                    f32::from(p[i]) * STEP_KM + if i == 2 { CAMERA_BIAS } else { 0.0 }
                })
            });
            flatten(target, &context, curve, &mut counts, DEPTH)?;
        }
        Ok(counts)
    }

    fn retained_bytes(&self) -> usize {
        self.visible.capacity() * core::mem::size_of::<ShortCurve>() + core::mem::size_of::<Self>()
    }
}

impl Prepared {
    fn new(site: GeodeticPosition, bytes: &[u8]) -> Result<Self, &'static str> {
        Self::new_ranges(
            site,
            bytes,
            (0..decode::PARTS).map(|part| decode::part_range(bytes, part)),
        )
    }

    fn from_range(
        site: GeodeticPosition,
        bytes: &[u8],
        range: Range<usize>,
    ) -> Result<Self, &'static str> {
        Self::new_ranges(site, bytes, core::iter::once(range))
    }

    fn new_ranges(
        site: GeodeticPosition,
        bytes: &[u8],
        ranges: impl Iterator<Item = Range<usize>>,
    ) -> Result<Self, &'static str> {
        if !decode::checked(bytes) {
            return Err("invalid packed structure");
        }
        Camera::new(site).map_err(|_| "invalid observer")?;
        // The initial lens is arbitrary: neither visibility roots nor camera
        // transform uses it. with_view supplies the actual FOV at draw time.
        let context = Context::new(site, View::new(20.0).unwrap());
        let mut prepared = Self {
            site,
            context,
            visible: Vec::new(),
            source_pieces: 0,
            peak_capacity: 0,
        };
        for range in ranges {
            if range.start > range.end
                || range.end > bytes.len()
                || range.end - range.start < 24
                || !(range.end - range.start - 9).is_multiple_of(15)
            {
                return Err("invalid piece range");
            }
            for at in (range.start..range.end - 9).step_by(15) {
                prepared.source_pieces += 1;
                let q = decode::decode_fast(bytes, at);
                let f = q.map(|p| context.front(p));
                if f.iter().any(|x| !x.is_finite()) {
                    return Err("invalid facing");
                }
                if f.iter().all(|x| *x <= 0.0) {
                    continue;
                }
                let camera = q.map(|p| context.transform(p));
                if f.iter().all(|x| *x > 0.0) {
                    prepared.visible.push(camera);
                } else {
                    let (roots, n) = cuts(f);
                    for pair in roots[..n].windows(2) {
                        let t = 0.5 * (pair[0] + pair[1]);
                        let s = 1.0 - t;
                        let facing = s * s * f[0] + 2.0 * t * (1.0 - t) * f[1] + t * t * f[2];
                        if facing > 0.0 {
                            prepared.visible.push(subcurve(camera, pair[0], pair[1]));
                        }
                    }
                }
                prepared.peak_capacity = prepared.peak_capacity.max(prepared.visible.capacity());
            }
        }
        Ok(prepared)
    }

    fn draw<D: DrawTarget<Color = BinaryColor, Error = Infallible>>(
        &self,
        target: &mut D,
        site: GeodeticPosition,
        view: View,
    ) -> Result<Counts, &'static str> {
        if self.site != site {
            return Err("observer changed");
        }
        let context = self.context.with_view(view);
        let mut counts = Counts::default();
        for &curve in &self.visible {
            flatten(target, &context, curve, &mut counts, DEPTH)?;
        }
        Ok(counts)
    }

    fn retained_bytes(&self) -> usize {
        self.visible.capacity() * core::mem::size_of::<Curve>() + core::mem::size_of::<Self>()
    }
}

#[test]
fn prepared_geometry_reuses_zoom_and_rejects_bad_asset() {
    let bytes = packing::prepare().bytes;
    let site = work_review::cases()[0].1;
    assert!(Prepared::new(site, &bytes[..bytes.len() - 1]).is_err());
    assert!(
        Prepared::new(
            GeodeticPosition {
                latitude_rad: f64::NAN,
                ..site
            },
            &bytes
        )
        .is_err()
    );
    let prepared = Prepared::new(site, &bytes).unwrap();
    assert_eq!(prepared.site, site);
    assert_eq!(prepared.source_pieces, 2031);
    for fov in [0.1, 5.0, 20.0, 100.0] {
        let view = View::rectangular(fov, Layout::wide().aspect_ratio()).unwrap();
        let mut fresh = Screen::new();
        let mut reused = Screen::new();
        candidate_draw(&mut fresh, site, view, &bytes).unwrap();
        prepared.draw(&mut reused, site, view).unwrap();
        assert_eq!(fresh.pixels, reused.pixels, "zoom={fov}");
    }
    // The quantized trial is allowed to differ but must not silently drop
    // distant coastline in these controls; no tolerance is approved for use.
    let compact = Compact::new(&prepared).unwrap();
    for fov in [0.1, 5.0, 20.0, 100.0] {
        let view = View::rectangular(fov, Layout::wide().aspect_ratio()).unwrap();
        let mut baseline = Screen::new();
        let mut reduced = Screen::new();
        prepared.draw(&mut baseline, site, view).unwrap();
        compact.draw(&mut reduced, site, view).unwrap();
        assert_eq!(mask_distance(&baseline, &reduced).1, 0);
    }
    let mut outside = Prepared::new(site, &bytes).unwrap();
    outside.visible[0][0][0] = 1_000_000.0;
    assert!(Compact::new(&outside).is_err());
    // An explicit changed-site preparation is required, not a keyed bitmap
    // accidentally reused at a new site. UI invalidation is not implemented.
    let changed = work_review::cases()[5].1;
    let next = Prepared::new(changed, &bytes).unwrap();
    assert_ne!(next.site, prepared.site);
    let view = View::rectangular(20.0, Layout::wide().aspect_ratio()).unwrap();
    let mut fresh = Screen::new();
    let mut reused = Screen::new();
    candidate_draw(&mut fresh, changed, view, &bytes).unwrap();
    assert_eq!(
        prepared.draw(&mut reused, changed, view),
        Err("observer changed")
    );
    assert!(reused.pixels.iter().flatten().all(|&p| p == 0));
    assert_eq!(
        compact.draw(&mut reused, changed, view),
        Err("observer changed")
    );
    assert!(reused.pixels.iter().flatten().all(|&p| p == 0));
    next.draw(&mut reused, changed, view).unwrap();
    assert_eq!(fresh.pixels, reused.pixels);

    // Host-side staging example only: a late failure cannot publish a partial
    // layer if the caller draws into an unpublished buffer. No device rollback
    // or loading-screen transition is implemented by this experiment.
    let mut broken = Prepared::new(changed, &bytes).unwrap();
    broken.visible.push([[f32::NAN; 3]; 3]);
    let mut staging = Screen::new();
    assert_eq!(
        broken.draw(&mut staging, changed, view),
        Err("invalid camera curve")
    );
    assert_eq!(fresh.pixels, reused.pixels); // prior published result unchanged
}

// Bidirectional raster distance: changed pixels alone can conceal isolated
// gaps. Complete 400x240 masks are still not geographic proof.
fn mask_distance(a: &Screen, b: &Screen) -> (usize, usize, usize) {
    let mut changed = 0;
    let mut beyond_one = 0;
    let mut maximum = 0;
    for (first, second) in [(a, b), (b, a)] {
        let other: Vec<_> = second
            .pixels
            .iter()
            .enumerate()
            .flat_map(|(y, row)| {
                row.iter()
                    .enumerate()
                    .filter_map(move |(x, &v)| (v != 0).then_some((x as i32, y as i32)))
            })
            .collect();
        for (y, row) in first.pixels.iter().enumerate() {
            for (x, &v) in row.iter().enumerate() {
                if v == 0 || second.pixels[y][x] != 0 {
                    continue;
                }
                changed += 1;
                let (x, y) = (x as i32, y as i32);
                let near = (-1..=1).any(|dy| {
                    (-1..=1).any(|dx| {
                        let (xx, yy) = (x + dx, y + dy);
                        (0..400).contains(&xx)
                            && (0..240).contains(&yy)
                            && second.pixels[yy as usize][xx as usize] != 0
                    })
                });
                if !near {
                    beyond_one += 1;
                    maximum = maximum.max(
                        other
                            .iter()
                            .map(|&(xx, yy)| (x - xx).abs().max((y - yy).abs()) as usize)
                            .min()
                            .unwrap_or(400),
                    );
                } else {
                    maximum = maximum.max(1);
                }
            }
        }
    }
    (changed, beyond_one, maximum)
}

#[test]
#[ignore = "explicit host-only compact-geometry comparison; not a device accuracy or memory gate"]
fn coast_compact_trial() {
    let bytes = packing::prepare().bytes;
    std::println!(
        "case,prepare_and_pack_us,compact_bytes,visible_curves,zoom,compact_draw_us,changed_pixels,beyond_1px,max_distance_px"
    );
    for (name, site, fov) in work_review::cases() {
        let start = Instant::now();
        let original = Prepared::new(site, &bytes).unwrap();
        let compact = Compact::new(&original).unwrap();
        let prepare_us = start.elapsed().as_secs_f64() * 1e6;
        for zoom in [fov, 0.1, 5.0, 20.0, 100.0] {
            let view = View::rectangular(zoom, Layout::wide().aspect_ratio()).unwrap();
            let mut actual = Screen::new();
            let start = Instant::now();
            compact
                .draw(black_box(&mut actual), black_box(site), black_box(view))
                .unwrap();
            let draw_us = start.elapsed().as_secs_f64() * 1e6;
            let mut expected = Screen::new();
            original.draw(&mut expected, site, view).unwrap();
            let (changed, beyond_one, maximum) = mask_distance(&actual, &expected);
            std::println!(
                "{name},{prepare_us:.1},{},{},{zoom},{draw_us:.1},{changed},{beyond_one},{maximum}",
                compact.retained_bytes(),
                compact.visible.len()
            );
        }
    }
}

#[test]
#[ignore = "explicit host requested-heap probe; not an S3 memory or stack measurement"]
fn coast_preparation_memory_trial() {
    let bytes = packing::prepare().bytes; // immutable fixture, outside meter
    let view = View::rectangular(20.0, Layout::wide().aspect_ratio()).unwrap();
    std::println!(
        "inventory,pending_array_bytes={},curve_bytes={},context_bytes={},counts_bytes={},prepared_struct_bytes={},compact_struct_bytes={}",
        pending_bytes(),
        core::mem::size_of::<Curve>(),
        core::mem::size_of::<Context>(),
        core::mem::size_of::<Counts>(),
        core::mem::size_of::<Prepared>(),
        core::mem::size_of::<Compact>()
    );
    std::println!(
        "case,original_live_bytes,both_live_bytes,compact_live_bytes,peak_logical_bytes,realloc_envelope_bytes,allocation_calls,draw_alloc_calls,after_drop_bytes"
    );
    for (name, site, _) in work_review::cases() {
        let mut screen = Screen::new();
        memory::begin();
        let original = Prepared::new(site, &bytes).unwrap();
        let first = memory::snapshot();
        let compact = Compact::new(&original).unwrap();
        let both = memory::snapshot();
        drop(original);
        let compact_only = memory::snapshot();
        compact
            .draw(black_box(&mut screen), black_box(site), black_box(view))
            .unwrap();
        let after_draw = memory::snapshot();
        drop(compact);
        let final_stats = memory::end();
        assert_eq!(first.live, first.peak);
        assert_eq!(compact_only.live, both.live - first.live);
        assert_eq!(after_draw.calls, compact_only.calls);
        assert_eq!(final_stats.live, 0);
        std::println!(
            "{name},{},{},{},{},{},{},{},{}",
            first.live,
            both.live,
            compact_only.live,
            final_stats.peak,
            final_stats.transient_envelope,
            final_stats.calls,
            after_draw.calls - compact_only.calls,
            final_stats.live
        );
    }
}

#[test]
#[ignore = "explicit isolated-source host audit; not an all-observer completeness proof"]
fn coast_compact_isolated_trial() {
    let packed = packing::prepare();
    let mut scenes = 0;
    let mut visible = 0;
    let mut changed = 0;
    let mut beyond_one = 0;
    let mut maximum = 0;
    // Every source interval at its own midpoint at tight zoom, then stratified
    // fixed-site polar/dateline/limb/viewport checks at tight and overview FOVs.
    let controls = work_review::cases();
    for span in &packed.spans {
        // Source intervals are geographic; use the same interpolation as the
        // existing tight-sweep experiment.
        let mid = super::super::interpolate(span.a, span.b, 0.5);
        let midpoint = crate::coast::cost_preflight::observer(mid.1, mid.0);
        for (site, fov) in core::iter::once((midpoint, 0.1)).chain(
            controls
                .iter()
                .enumerate()
                .filter(|(i, _)| span.index % 23 == 0 && [5, 6, 7, 9].contains(i))
                .flat_map(|(_, (_, site, _))| [(*site, 0.1), (*site, 20.0)]),
        ) {
            let view = View::rectangular(fov, Layout::wide().aspect_ratio()).unwrap();
            let original = Prepared::from_range(site, &packed.bytes, span.bytes.clone()).unwrap();
            let compact = Compact::new(&original).unwrap();
            let mut reference = Screen::new();
            let mut baseline = Screen::new();
            let mut actual = Screen::new();
            render_range(
                &mut reference,
                &Context::new(site, view),
                &packed.bytes,
                span.bytes.clone(),
                &mut Counts::default(),
            )
            .unwrap();
            original.draw(&mut baseline, site, view).unwrap();
            assert_eq!(
                reference.pixels, baseline.pixels,
                "source segment {}",
                span.index
            );
            compact.draw(&mut actual, site, view).unwrap();
            let (c, b, m) = mask_distance(&reference, &actual);
            changed += c;
            beyond_one += b;
            maximum = maximum.max(m);
            visible += usize::from(reference.on_writes > 0);
            scenes += 1;
        }
    }
    std::println!(
        "isolated,scenes={scenes},visible={visible},changed={changed},beyond_1px={beyond_one},max_distance_px={maximum}"
    );
}

#[test]
#[ignore = "explicit host-only preparation/zoom timing; not device or full-frame evidence"]
fn coast_preparation_trial() {
    let bytes = packing::prepare().bytes;
    std::println!(
        "case,prepare_us,visible_curves,source_pieces,retained_bytes,peak_vec_capacity,draw_us,candidate_us,changed_pixels"
    );
    for (name, site, fov) in work_review::cases() {
        let start = Instant::now();
        let prepared = Prepared::new(black_box(site), black_box(&bytes)).unwrap();
        let setup_us = start.elapsed().as_secs_f64() * 1e6;
        // Additional FOVs for the SAME prepared site; setup is outside draw.
        for zoom in [fov, 0.1, 5.0, 20.0, 100.0] {
            let view = View::rectangular(zoom, Layout::wide().aspect_ratio()).unwrap();
            let mut trial = Screen::new();
            let start = Instant::now();
            let counts = prepared
                .draw(black_box(&mut trial), black_box(site), black_box(view))
                .unwrap();
            let draw_us = start.elapsed().as_secs_f64() * 1e6;
            let mut reference = Screen::new();
            let start = Instant::now();
            candidate_draw(
                black_box(&mut reference),
                black_box(site),
                black_box(view),
                black_box(&bytes),
            )
            .unwrap();
            let candidate_us = start.elapsed().as_secs_f64() * 1e6;
            let changed = trial
                .pixels
                .iter()
                .flatten()
                .zip(reference.pixels.iter().flatten())
                .filter(|(a, b)| a != b)
                .count();
            assert_eq!(changed, 0, "{name} zoom={zoom}");
            std::println!(
                "{name}/{zoom},{setup_us:.1},{},{},{},{},{draw_us:.1},{candidate_us:.1},{changed}",
                prepared.visible.len(),
                prepared.source_pieces,
                prepared.retained_bytes(),
                prepared.peak_capacity
            );
            black_box(counts);
        }
    }
}
