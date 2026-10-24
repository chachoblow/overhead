//! Allocation-free reader for the single immutable measurement fixture.
//! Structural checks are not a production corrupt-asset/CRC policy.
use super::{Curve, V3};
use core::ops::Range;
pub(super) const PARTS: usize = 134;
const HEADER: usize = (PARTS + 1) * 4;
fn u32_at(bytes: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?) as usize)
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
            && r.end >= r.start.saturating_add(24)
            && (r.end - r.start - 9).is_multiple_of(15)
    })
}
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
