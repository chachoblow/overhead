//! Host preparation only. The timed trial consumes the actual packed bytes.
//! Sampled fitting is not an all-parameter approximation certificate.
extern crate std;
use super::*;
use crate::coast::*;
use std::{ops::Range, vec, vec::Vec};

pub(super) const HEADER: usize = (PARTS + 1) * 4;
const FIT_KM: f64 = 0.020;
const POSITION_SCALE: f64 = 1024.0; // signed i24: ±8192 km, ~1 m quantum
const CORRECTION_SCALE: f64 = 128.0; // signed i16: ±256 km, ~8 m quantum
pub(super) type D3 = [f64; 3];

#[derive(Clone)]
pub(super) struct SourceSpan {
    pub index: usize,
    pub a: Geo,
    pub b: Geo,
    pub bytes: Range<usize>,
}

pub(super) struct Piece {
    pub a: Geo,
    pub b: Geo,
    pub offset: usize,
}

pub(super) struct Packed {
    pub bytes: Vec<u8>,
    // Host audit metadata only; neither is needed by the rendering path.
    pub spans: Vec<SourceSpan>,
    pub pieces: Vec<Piece>,
}

pub(super) fn world(p: Geo) -> D3 {
    GeodeticPosition {
        longitude_rad: p.0.to_radians(),
        latitude_rad: p.1.to_radians(),
        altitude_km: 0.0,
    }
    .to_ecef()
    .unwrap()
    .km()
}

pub(super) fn eval(q: [D3; 3], t: f64) -> D3 {
    core::array::from_fn(|i| {
        (1.0 - t) * (1.0 - t) * q[0][i] + 2.0 * t * (1.0 - t) * q[1][i] + t * t * q[2][i]
    })
}

fn control(a: D3, m: D3, b: D3) -> D3 {
    core::array::from_fn(|i| 2.0 * m[i] - 0.5 * (a[i] + b[i]))
}

fn distance(a: D3, b: D3) -> f64 {
    libm::sqrt((0..3).map(|i| (a[i] - b[i]).powi(2)).sum())
}

fn put_position(out: &mut Vec<u8>, p: D3) {
    for v in p {
        let q = (v * POSITION_SCALE).round();
        assert!((-8_388_608.0..=8_388_607.0).contains(&q));
        out.extend_from_slice(&(q as i32).to_le_bytes()[..3]);
    }
}

fn position(bytes: &[u8], at: usize) -> D3 {
    core::array::from_fn(|i| {
        let j = at + 3 * i;
        let sign = if bytes[j + 2] & 0x80 == 0 { 0 } else { 255 };
        f64::from(i32::from_le_bytes([
            bytes[j],
            bytes[j + 1],
            bytes[j + 2],
            sign,
        ])) / POSITION_SCALE
    })
}

pub(super) fn decode(bytes: &[u8], at: usize) -> [D3; 3] {
    let a = position(bytes, at);
    let b = position(bytes, at + 15);
    let c = core::array::from_fn(|i| {
        let j = at + 9 + i * 2;
        let correction = f64::from(i16::from_le_bytes([bytes[j], bytes[j + 1]]));
        0.5 * (a[i] + b[i]) + correction / CORRECTION_SCALE
    });
    [a, c, b]
}

// Separate f32 decoder used by the timed path: no f64 arithmetic per vertex.
pub(super) fn decode_fast(bytes: &[u8], at: usize) -> Curve {
    let pos = |at: usize| {
        core::array::from_fn(|i| {
            let j = at + i * 3;
            let sign = if bytes[j + 2] & 0x80 == 0 { 0 } else { 255 };
            i32::from_le_bytes([bytes[j], bytes[j + 1], bytes[j + 2], sign]) as f32 * (1.0 / 1024.0)
        })
    };
    let a: V3 = pos(at);
    let b: V3 = pos(at + 15);
    let c = core::array::from_fn(|i| {
        let j = at + 9 + 2 * i;
        0.5 * (a[i] + b[i])
            + f32::from(i16::from_le_bytes([bytes[j], bytes[j + 1]])) * (1.0 / 128.0)
    });
    [a, c, b]
}

fn append(packed: &mut Packed, a: Geo, b: Geo, depth: usize) {
    let pa = world(a);
    let pb = world(b);
    let mid = interpolate(a, b, 0.5);
    let pc = control(pa, world(mid), pb);
    let error = (1..16)
        .map(|k| {
            let t = f64::from(k) / 16.0;
            distance(world(interpolate(a, b, t)), eval([pa, pc, pb], t))
        })
        .fold(0.0, f64::max);
    if error > FIT_KM {
        assert!(depth < 16, "host fit exhausted; never silently truncate");
        append(packed, a, mid, depth + 1);
        append(packed, mid, b, depth + 1);
        return;
    }
    let offset = packed.bytes.len() - 9;
    let decoded_a = position(&packed.bytes, offset);
    let mut endpoint = Vec::new();
    put_position(&mut endpoint, pb);
    let decoded_b = position(&endpoint, 0);
    // Fit midpoint using decoded endpoints, then quantize the correction.
    for i in 0..3 {
        let correction = 2.0 * (world(mid)[i] - 0.5 * (decoded_a[i] + decoded_b[i]));
        let q = (correction * CORRECTION_SCALE).round();
        assert!((-32768.0..=32767.0).contains(&q), "correction overflow");
        packed.bytes.extend_from_slice(&(q as i16).to_le_bytes());
    }
    packed.bytes.extend_from_slice(&endpoint);
    packed.pieces.push(Piece { a, b, offset });
}

pub(super) fn prepare_segment(a: Geo, b: Geo) -> Vec<u8> {
    let mut packed = Packed {
        bytes: Vec::new(),
        spans: Vec::new(),
        pieces: Vec::new(),
    };
    put_position(&mut packed.bytes, world(a));
    append(&mut packed, a, b, 0);
    packed.bytes
}

pub(super) fn prepare() -> Packed {
    assert!(valid(ASSET));
    let mut result = Packed {
        bytes: vec![0; HEADER],
        spans: Vec::new(),
        pieces: Vec::new(),
    };
    for part in 0..PARTS {
        let offset = result.bytes.len() as u32;
        result.bytes[4 * part..4 * part + 4].copy_from_slice(&offset.to_le_bytes());
        let start = u32_at(ASSET, COORD_BYTES + 4 * part).unwrap();
        let end = u32_at(ASSET, COORD_BYTES + 4 * (part + 1)).unwrap();
        put_position(&mut result.bytes, world(point(ASSET, start).unwrap()));
        for index in start..end - 1 {
            let a = point(ASSET, index).unwrap();
            let b = point(ASSET, index + 1).unwrap();
            let first = result.bytes.len() - 9;
            append(&mut result, a, b, 0);
            result.spans.push(SourceSpan {
                index,
                a,
                b,
                bytes: first..result.bytes.len(),
            });
        }
    }
    let end = result.bytes.len() as u32;
    result.bytes[PARTS * 4..HEADER].copy_from_slice(&end.to_le_bytes());
    assert!(checked(&result.bytes));
    result
}

pub(super) fn part_range(bytes: &[u8], part: usize) -> Range<usize> {
    u32_at(bytes, part * 4).unwrap()..u32_at(bytes, (part + 1) * 4).unwrap()
}

pub(super) fn checked(bytes: &[u8]) -> bool {
    if bytes.len() < HEADER
        || u32_at(bytes, 0) != Some(HEADER)
        || u32_at(bytes, PARTS * 4) != Some(bytes.len())
    {
        return false;
    }
    (0..PARTS).all(|part| {
        let r = part_range(bytes, part);
        r.start >= HEADER
            && r.end <= bytes.len()
            && r.end >= r.start + 24
            && (r.end - r.start - 9).is_multiple_of(15)
    })
}

pub(super) fn representation_error(packed: &Packed) -> (f64, f64) {
    let mut quadratic = 0.0_f64;
    let mut chord = 0.0_f64;
    for piece in &packed.pieces {
        let q = decode(&packed.bytes, piece.offset);
        for k in 0..=64 {
            let t = f64::from(k) / 64.0; // finer than the fitting samples
            quadratic = quadratic.max(distance(
                world(interpolate(piece.a, piece.b, t)),
                eval(q, t),
            ));
        }
    }
    // The tempting chord-only replacement on ORIGINAL segments, not the
    // already refined quadratic pieces. This does not promise a surface curve.
    for span in &packed.spans {
        let a = world(span.a);
        let b = world(span.b);
        for k in 0..=64 {
            let t = f64::from(k) / 64.0;
            let p = core::array::from_fn(|i| a[i] + t * (b[i] - a[i]));
            chord = chord.max(distance(world(interpolate(span.a, span.b, t)), p));
        }
    }
    (quadratic, chord)
}

pub(super) fn projected_error(packed: &Packed, site: GeodeticPosition, view: View) -> f64 {
    let camera = Camera::new(site).unwrap();
    let context = Context::new(site, view);
    let mut maximum = 0.0_f64;
    for piece in &packed.pieces {
        let q = decode_fast(&packed.bytes, piece.offset);
        for k in 0..=64 {
            let t = f64::from(k) / 64.0;
            let geo = interpolate(piece.a, piece.b, t);
            let (p, facing) = sample(&camera, geo).unwrap();
            let p = view.project_unclipped(p).unwrap();
            if facing <= 0.0 || p.east.abs() > 1.0 || p.north.abs() > 1.0 {
                continue;
            }
            let x = curve_at(q, t as f32);
            let projected = context.project(context.transform(x));
            let dx = (p.east - f64::from(projected[0])) * 173.0;
            let dy = (p.north - f64::from(projected[1])) * 117.0;
            maximum = maximum.max(libm::hypot(dx, dy));
        }
    }
    maximum
}

#[test]
fn representation_packing_is_checked_and_reproducible() {
    let packed = prepare();
    assert_eq!(packed.bytes, prepare().bytes);
    assert_eq!(packed.spans.len(), 1734);
    assert!(checked(&packed.bytes));
    assert!(super::decode::checked(&packed.bytes));
    assert!(!super::decode::checked(
        &packed.bytes[..packed.bytes.len() - 1]
    ));
    assert!(!checked(&packed.bytes[..packed.bytes.len() - 1]));
    let mut broken = packed.bytes.clone();
    broken[4..8].copy_from_slice(&0_u32.to_le_bytes());
    assert!(!checked(&broken));
    assert!(!super::decode::checked(&broken));
    broken[..4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(!super::decode::checked(&broken));
    for piece in &packed.pieces {
        let q = decode(&packed.bytes, piece.offset);
        let fast = decode_fast(&packed.bytes, piece.offset);
        assert_eq!(
            fast,
            super::decode::decode_fast(&packed.bytes, piece.offset)
        );
        for i in 0..3 {
            for j in 0..3 {
                assert!((q[i][j] - f64::from(fast[i][j])).abs() <= 0.001);
            }
        }
    }
    let (error, chord) = representation_error(&packed);
    assert!(error < 0.025, "sampled representation regression: {error}");
    assert!(chord > error);
}
