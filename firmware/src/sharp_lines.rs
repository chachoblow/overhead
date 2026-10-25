//! Opt-in Sharp selective-line diagnostic, separate from accepted replay/cache.
//! One SCS-high transaction per step; caller owns SPI, CS, timing and VCOM phase.
//! Only a completed transfer establishes a new known panel image. Errors make
//! the next attempted frame a full resync. No live device scheduling is implied.
use crate::sharp_test::{self, FRAME_BYTES, HEIGHT, WIDTH};

const ROW_BYTES: usize = WIDTH / 8;
const LINE_BYTES: usize = ROW_BYTES + 2;
pub const PERIODIC_FULL_TICKS: u32 = 16; // 8 seconds at this image's 500 ms tick

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Frame { second: i64, force_full: bool },
    Idle,
}

/// Seven accelerated fixture samples followed by one idle-VCOM tick. Every
/// second replay loop begins with a full refresh, not an assumed stable panel.
pub const fn step(tick: u32) -> Step {
    if tick % 8 == 7 {
        Step::Idle
    } else {
        Step::Frame {
            second: ((tick - tick / 8) % 7 * 50) as i64,
            force_full: tick.is_multiple_of(PERIODIC_FULL_TICKS),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Full,
    Lines,
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transfer {
    pub kind: Kind,
    pub rows: usize,
    pub bytes: usize,
}

pub struct LineSender {
    previous: [u8; FRAME_BYTES],
    valid: bool,
}

impl LineSender {
    pub const fn new() -> Self {
        Self {
            previous: [0; FRAME_BYTES],
            valid: false,
        }
    }

    /// Send one frame; a failed write invalidates the retained display state.
    /// Keep active-high CS asserted for this entire function. Even if the
    /// framebuffer is unchanged, display mode toggles M1/VCOM without ink.
    pub fn send<E>(
        &mut self,
        vcom: bool,
        pixels: &[u8; FRAME_BYTES],
        force_full: bool,
        mut write: impl FnMut(&[u8]) -> Result<(), E>,
    ) -> Result<Transfer, E> {
        let full = force_full || !self.valid;
        self.valid = false; // any incomplete transaction leaves panel state unknown
        let result = if full {
            sharp_test::frame_pixels(vcom, pixels, &mut write)?;
            Transfer {
                kind: Kind::Full,
                rows: HEIGHT,
                bytes: 1 + HEIGHT * LINE_BYTES + 1,
            }
        } else {
            let mut rows = 0;
            for (y, (old, new)) in self
                .previous
                .chunks_exact(ROW_BYTES)
                .zip(pixels.chunks_exact(ROW_BYTES))
                .enumerate()
            {
                if old != new {
                    if rows == 0 {
                        write(&[0x01 | if vcom { 0x02 } else { 0 }])?;
                    }
                    let mut line = [0_u8; LINE_BYTES];
                    line[0] = (y + 1) as u8;
                    for (dest, &byte) in line[1..=ROW_BYTES].iter_mut().zip(new) {
                        *dest = !byte;
                    }
                    write(&line)?;
                    rows += 1;
                }
            }
            if rows == 0 {
                write(&[if vcom { 0x02 } else { 0 }])?;
            }
            write(&[0])?;
            Transfer {
                kind: if rows == 0 { Kind::Idle } else { Kind::Lines },
                rows,
                bytes: 2 + rows * LINE_BYTES,
            }
        };
        self.previous.copy_from_slice(pixels);
        self.valid = true;
        Ok(result)
    }

    /// No render on the image's dedicated idle step. Only use this when the
    /// previous frame was confirmed; an error returns without toggling VCOM.
    pub fn idle<E>(
        &mut self,
        vcom: bool,
        mut write: impl FnMut(&[u8]) -> Result<(), E>,
    ) -> Result<Transfer, E> {
        if !self.valid {
            // Cannot reconstruct pixels here. Caller must render/send full first.
            // This is a diagnostic contract violation, not a recoverable idle.
            panic!("idle before confirmed full frame");
        }
        self.valid = false;
        write(&[if vcom { 0x02 } else { 0 }])?;
        write(&[0])?;
        self.valid = true;
        Ok(Transfer {
            kind: Kind::Idle,
            rows: 0,
            bytes: 2,
        })
    }
}

impl Default for LineSender {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use overhead_core::sgp4::chrono::TimeDelta;
    use overhead_render::{CoastLayer, Layout, PeriodCache};
    use std::vec::Vec;

    fn collect(
        sender: &mut LineSender,
        phase: bool,
        pixels: &[u8; FRAME_BYTES],
        force: bool,
    ) -> (Transfer, Vec<Vec<u8>>) {
        let mut chunks = Vec::new();
        let transfer = sender
            .send(phase, pixels, force, |part| {
                chunks.push(part.to_vec());
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(transfer.bytes, chunks.iter().map(Vec::len).sum::<usize>());
        (transfer, chunks)
    }

    #[test]
    fn scheduled_replay_matches_full_panel_including_wrap_idle_and_periodic_resync() {
        let mut app = crate::coast_measurement::app("iss-ground-20", 0.0, 0.0, 5.0);
        let epoch = app.time().unwrap();
        let mut coast = CoastLayer::prepare(app.observer());
        let mut period = PeriodCache::new();
        let mut screen = crate::coast_measurement::Screen::new();
        let mut sender = LineSender::new();
        let mut partial_panel = [0_u8; FRAME_BYTES];
        let mut full_panel = partial_panel;
        for tick in 0..40_u32 {
            let phase = tick % 2 != 0;
            match step(tick) {
                Step::Idle => {
                    let mut chunks = Vec::new();
                    let transfer = sender
                        .idle(phase, |part| {
                            chunks.push(part.to_vec());
                            Ok::<_, ()>(())
                        })
                        .unwrap();
                    assert_eq!(transfer.kind, Kind::Idle);
                    assert_eq!(
                        crate::sharp_transport::apply(&chunks, &mut partial_panel, phase),
                        0
                    );
                }
                Step::Frame { second, force_full } => {
                    app.update(epoch + TimeDelta::seconds(second)).unwrap();
                    overhead_render::draw_cached(
                        &mut screen,
                        &app,
                        Layout::wide(),
                        false,
                        &mut coast,
                        &mut period,
                    )
                    .unwrap();
                    let (transfer, chunks) =
                        collect(&mut sender, phase, &screen.pixels, force_full);
                    let count = crate::sharp_transport::apply(&chunks, &mut partial_panel, phase);
                    assert_eq!(count, transfer.rows);
                    if tick % PERIODIC_FULL_TICKS == 0 {
                        assert_eq!(transfer.kind, Kind::Full);
                    } else {
                        assert_eq!(transfer.kind, Kind::Lines);
                        assert!((18..=34).contains(&count), "tick {tick}");
                    }
                    let mut full_chunks = Vec::new();
                    sharp_test::frame_pixels(phase, &screen.pixels, |part| {
                        full_chunks.push(part.to_vec());
                        Ok::<_, ()>(())
                    })
                    .unwrap();
                    crate::sharp_transport::apply(&full_chunks, &mut full_panel, phase);
                }
            }
            assert_eq!(partial_panel, full_panel, "tick {tick}");
        }
        assert!(coast.error().is_none());
    }

    #[test]
    fn empty_frame_erasures_and_failed_transaction_force_resync() {
        let mut sender = LineSender::new();
        let mut pixels = [0_u8; FRAME_BYTES];
        let (first, _) = collect(&mut sender, false, &pixels, false);
        assert_eq!(first.kind, Kind::Full);
        let (unchanged, _) = collect(&mut sender, true, &pixels, false);
        assert_eq!(unchanged.kind, Kind::Idle);
        pixels[0] = 0xff;
        let (changed, _) = collect(&mut sender, false, &pixels, false);
        assert_eq!((changed.kind, changed.rows), (Kind::Lines, 1));
        pixels[0] = 0;
        let (erased, _) = collect(&mut sender, true, &pixels, false);
        assert_eq!((erased.kind, erased.rows), (Kind::Lines, 1));
        pixels[ROW_BYTES] = 1;
        let mut writes = 0;
        assert_eq!(
            sender.send(false, &pixels, false, |_| {
                writes += 1;
                if writes == 2 { Err(()) } else { Ok(()) }
            }),
            Err(())
        );
        assert_eq!(writes, 2);
        let (recovered, chunks) = collect(&mut sender, true, &pixels, false);
        assert_eq!(recovered.kind, Kind::Full);
        let mut panel = [0xff; FRAME_BYTES];
        crate::sharp_transport::apply(&chunks, &mut panel, true);
        assert_eq!(panel, pixels);
        assert_eq!(
            sender.idle(false, |part| if part.len() == 1 && part[0] == 0 {
                Err(())
            } else {
                Ok(())
            }),
            Err(())
        );
        let (again, _) = collect(&mut sender, true, &pixels, false);
        assert_eq!(again.kind, Kind::Full);
    }
}
