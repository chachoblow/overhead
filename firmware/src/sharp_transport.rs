//! Host-only wire-budget probe; the separate opt-in `sharp-lines` diagnostic
//! is the only S3 partial-transfer candidate. Emulator checks pixels, not timing.
extern crate std;
use crate::sharp_test::{FRAME_BYTES, HEIGHT, WIDTH};
use std::vec::Vec;

const ROW_BYTES: usize = WIDTH / 8;
const LINE_BYTES: usize = ROW_BYTES + 2; // address + pixels + dummy
const FULL_BYTES: usize = 1 + HEIGHT * LINE_BYTES + 1;

// Candidate multi-line transaction: single data-update command and final dummy,
// with only lines whose *entire* 50-byte pixel row differs from the last
// confirmed display state. This is an offline hypothesis, not a device API.
fn changed_lines(
    vcom: bool,
    previous: &[u8; FRAME_BYTES],
    current: &[u8; FRAME_BYTES],
) -> Vec<Vec<u8>> {
    let mut chunks = Vec::new();
    let mut count = 0;
    for (y, (old, new)) in previous
        .chunks_exact(ROW_BYTES)
        .zip(current.chunks_exact(ROW_BYTES))
        .enumerate()
    {
        if old != new {
            if count == 0 {
                chunks.push(std::vec![0x01 | if vcom { 0x02 } else { 0 }]);
            }
            let mut line = std::vec![0; LINE_BYTES];
            line[0] = (y + 1) as u8;
            for (dest, &byte) in line[1..=ROW_BYTES].iter_mut().zip(new) {
                *dest = !byte;
            }
            chunks.push(line);
            count += 1;
        }
    }
    if count == 0 {
        // Display mode (M0=0, M2=0), only toggles serial M1/VCOM.
        chunks.push(std::vec![if vcom { 0x02 } else { 0 }]);
    }
    chunks.push(std::vec![0]);
    chunks
}

fn full(vcom: bool, current: &[u8; FRAME_BYTES]) -> Vec<Vec<u8>> {
    let mut chunks = Vec::new();
    crate::sharp_test::frame_pixels(vcom, current, |part| {
        chunks.push(part.to_vec());
        Ok::<_, ()>(())
    })
    .unwrap();
    chunks
}

fn wire_bytes(chunks: &[Vec<u8>]) -> usize {
    chunks.iter().map(Vec::len).sum()
}

// Independent panel-memory model: accept complete lines with unique 1-based
// addresses, decode the wire polarity, and retain all unaddressed lines.
pub(crate) fn apply(chunks: &[Vec<u8>], state: &mut [u8; FRAME_BYTES], vcom: bool) -> usize {
    assert!(chunks.len() >= 2);
    assert_eq!(chunks.last().unwrap().as_slice(), [0]);
    let lines = chunks.len() - 2;
    assert_eq!(chunks[0].len(), 1);
    assert_eq!(
        chunks[0][0],
        (if lines == 0 { 0 } else { 1 }) | if vcom { 2 } else { 0 }
    );
    let mut seen = [false; HEIGHT];
    for line in &chunks[1..chunks.len() - 1] {
        assert_eq!(line.len(), LINE_BYTES);
        let row = usize::from(line[0]);
        assert!((1..=HEIGHT).contains(&row));
        assert!(!seen[row - 1], "line written twice");
        seen[row - 1] = true;
        assert_eq!(line[ROW_BYTES + 1], 0);
        for (dest, &byte) in state[(row - 1) * ROW_BYTES..row * ROW_BYTES]
            .iter_mut()
            .zip(&line[1..=ROW_BYTES])
        {
            *dest = !byte;
        }
    }
    lines
}

#[test]
fn partial_lines_preserve_pixels_and_vcom_on_empty_and_changed_rows() {
    let mut old = [0_u8; FRAME_BYTES];
    let mut state = old;
    let idle = changed_lines(true, &old, &old);
    assert_eq!(wire_bytes(&idle), 2);
    assert_eq!(apply(&idle, &mut state, true), 0);
    assert_eq!(state, old);

    let mut next = old;
    next[ROW_BYTES * 239] = 0x81;
    next[0] = 0x02;
    let changed = changed_lines(false, &old, &next);
    assert_eq!(wire_bytes(&changed), 2 + 2 * LINE_BYTES);
    assert_eq!(changed[1][0], 1);
    assert_eq!(changed[2][0], 240);
    assert_eq!(apply(&changed, &mut state, false), 2);
    assert_eq!(state, next);
    old = next;
    next[0] = 0; // erasure must be sent too
    assert_eq!(
        apply(&changed_lines(true, &old, &next), &mut state, true),
        1
    );
    assert_eq!(state, next);

    // If panel state is unknown (reset or failed transfer), comparison against
    // the old buffer is insufficient: force a full resync, then resume diffs.
    let mut unknown = [0xff; FRAME_BYTES];
    assert_ne!(unknown, next);
    assert_eq!(apply(&full(false, &next), &mut unknown, false), HEIGHT);
    assert_eq!(unknown, next);
    assert_eq!(wire_bytes(&full(false, &next)), FULL_BYTES);
}

#[test]
fn fixture_replay_transport_budget_and_panel_parity() {
    use overhead_core::sgp4::chrono::TimeDelta;
    use overhead_render::{CoastLayer, Layout, PeriodCache};
    let mut app = crate::coast_measurement::app("iss-ground-20", 0.0, 0.0, 5.0);
    let epoch = app.time().unwrap();
    let mut coast = CoastLayer::prepare(app.observer());
    let mut cache = PeriodCache::new();
    let mut screen = crate::coast_measurement::Screen::new();
    let mut previous = [0_u8; FRAME_BYTES]; // white after the Sharp clear command
    let mut partial_panel = previous;
    let mut full_panel = previous;
    let mut rows = [0_usize; 8]; // initial full frame, then seven transitions (including wrap)
    for tick in 0..21_u32 {
        let second = crate::sharp_test::replay_second(tick);
        app.update(epoch + TimeDelta::seconds(second)).unwrap();
        overhead_render::draw_cached(
            &mut screen,
            &app,
            Layout::wide(),
            false,
            &mut coast,
            &mut cache,
        )
        .unwrap();
        let vcom = tick % 2 != 0;
        let all = full(vcom, &screen.pixels);
        assert_eq!(apply(&all, &mut full_panel, vcom), HEIGHT);
        assert_eq!(wire_bytes(&all), FULL_BYTES);
        let changed = if tick == 0 {
            // Initial panel state must be established unconditionally.
            all
        } else {
            changed_lines(vcom, &previous, &screen.pixels)
        };
        let count = apply(&changed, &mut partial_panel, vcom);
        assert_eq!(partial_panel, screen.pixels, "partial tick {tick}");
        assert_eq!(partial_panel, full_panel, "full tick {tick}");
        if tick < 8 {
            rows[tick as usize] = count;
        } else {
            assert_eq!(
                count,
                rows[1 + (tick as usize - 1) % 7],
                "repeat tick {tick}"
            );
        }
        let bytes = wire_bytes(&changed);
        // Bit-time floor only, excluding CPU/SPI calls, chip-select and render.
        // At 2 MHz, 1 byte costs 4 microseconds.
        std::println!(
            "TRANSPORT tick={tick} fixture_s={second} changed_rows={count} bytes={bytes} wire_floor_us={}",
            bytes * 4
        );
        previous.copy_from_slice(&screen.pixels);
    }
    assert!(coast.error().is_none());
    assert_eq!(rows, [240, 34, 30, 25, 23, 18, 22, 33]);
    assert_eq!(
        rows[1..]
            .iter()
            .map(|&n| 2 + n * LINE_BYTES)
            .collect::<Vec<_>>(),
        [1770, 1562, 1302, 1198, 938, 1146, 1718]
    );
    std::println!(
        "TRANSPORT full_bytes={FULL_BYTES} full_wire_floor_us={}",
        FULL_BYTES * 4
    );
}
