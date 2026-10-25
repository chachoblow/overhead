//! Allocation-free wire format for the isolated #4694 (LS027B7DH01) test.
//! Caller holds the display's *active-high* CS for the entire command and
//! transmits every byte LSB-first in SPI mode 0. No frame buffer is required.
//! Reference: Sharp LS027B7DH01 datasheet §§6-5, 6-6 and Adafruit_SharpMem.

pub const WIDTH: usize = 400;
pub const HEIGHT: usize = 240;
const BYTES_PER_LINE: usize = WIDTH / 8;
pub const FRAME_BYTES: usize = HEIGHT * BYTES_PER_LINE;
/// Deliberately accelerated ISS excerpt: 0..=300 fixture seconds in 50 s
/// increments, one frame per 500 ms display tick. This is not a wall clock.
#[cfg(feature = "sharp-replay")]
pub const REPLAY_FRAMES: u32 = 7;
#[cfg(feature = "sharp-replay")]
pub fn replay_second(tick: u32) -> i64 {
    i64::from(tick % REPLAY_FRAMES) * 50
}

/// `vcom` must alternate at a steady rate even when the image is unchanged.
pub fn clear<E>(vcom: bool, mut write: impl FnMut(&[u8]) -> Result<(), E>) -> Result<(), E> {
    write(&[0x04 | if vcom { 0x02 } else { 0 }, 0x00])
}

/// Two distinct full-screen patterns, with a border and large black/white bands.
/// White pixels are 1; line addresses are 1-based and transmitted LSB-first.
pub fn frame<E>(
    vcom: bool,
    phase: bool,
    mut write: impl FnMut(&[u8]) -> Result<(), E>,
) -> Result<(), E> {
    write(&[0x01 | if vcom { 0x02 } else { 0 }])?;
    for y in 0..HEIGHT {
        let mut line = [0xff; BYTES_PER_LINE + 2];
        line[0] = (y + 1) as u8;
        line[BYTES_PER_LINE + 1] = 0;
        for (i, byte) in line[1..=BYTES_PER_LINE].iter_mut().enumerate() {
            // Large quadrants change every two seconds; a solid border reveals
            // orientation, clipping and missing edge rows.
            let border = y < 4 || y >= HEIGHT - 4 || i == 0 || i == BYTES_PER_LINE - 1;
            let dark = border || ((y >= HEIGHT / 2) ^ (i >= BYTES_PER_LINE / 2) ^ phase);
            *byte = if dark { 0 } else { 0xff };
        }
        write(&line)?;
    }
    write(&[0x00]) // final dummy byte, required after the last line
}

/// Transfer an embedded-graphics framebuffer (1 = black/On) to the Sharp
/// panel (1 = white), with no on-stack full-frame copy or transposition.
pub fn frame_pixels<E>(
    vcom: bool,
    pixels: &[u8; FRAME_BYTES],
    mut write: impl FnMut(&[u8]) -> Result<(), E>,
) -> Result<(), E> {
    write(&[0x01 | if vcom { 0x02 } else { 0 }])?;
    for (y, source) in pixels.chunks_exact(BYTES_PER_LINE).enumerate() {
        let mut line = [0_u8; BYTES_PER_LINE + 2];
        line[0] = (y + 1) as u8;
        for (dest, &byte) in line[1..=BYTES_PER_LINE].iter_mut().zip(source) {
            *dest = !byte;
        }
        write(&line)?;
    }
    write(&[0x00])
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;

    #[test]
    fn full_frame_has_all_lines_and_both_phases() {
        for phase in [false, true] {
            let mut chunks = std::vec::Vec::new();
            frame(false, phase, |bytes| {
                chunks.push(bytes.to_vec());
                Ok::<_, ()>(())
            })
            .unwrap();
            assert_eq!(chunks.len(), HEIGHT + 2);
            assert_eq!(chunks[0], [0x01]);
            assert_eq!(chunks[1][0], 1);
            assert_eq!(chunks[HEIGHT][0], 240);
            assert_eq!(chunks[HEIGHT + 1], [0]);
            assert!(
                chunks[1..=HEIGHT]
                    .iter()
                    .all(|line| line.len() == BYTES_PER_LINE + 2 && line[51] == 0)
            );
            assert_eq!(chunks[1][1], 0); // black border
            assert_eq!(chunks[1][49], 0); // other border
            assert_eq!(chunks[5][2], if phase { 0 } else { 0xff });
            assert_eq!(chunks[5][26], if phase { 0xff } else { 0 });
            assert_eq!(chunks[HEIGHT - 4][2], if phase { 0xff } else { 0 });
        }
    }

    #[test]
    fn framebuffer_transfer_preserves_row_order_and_inverts_ink() {
        let mut pixels = [0_u8; FRAME_BYTES];
        pixels[0] = 0b1000_0001; // first and eighth screen pixels black
        pixels[FRAME_BYTES - 1] = 0x55;
        let mut chunks = std::vec::Vec::new();
        frame_pixels(true, &pixels, |part| {
            chunks.push(part.to_vec());
            Ok::<_, ()>(())
        })
        .unwrap();
        assert_eq!(chunks.len(), HEIGHT + 2);
        assert_eq!(chunks[0], [0x03]);
        assert_eq!(chunks[1][0], 1);
        assert_eq!(chunks[1][1], !0b1000_0001);
        assert_eq!(chunks[1][2], 0xff);
        assert_eq!(chunks[HEIGHT][0], 240);
        assert_eq!(chunks[HEIGHT][BYTES_PER_LINE], !0x55);
        assert!(chunks[1..=HEIGHT].iter().all(|line| line[51] == 0));
        assert_eq!(chunks[HEIGHT + 1], [0]);
    }

    #[cfg(feature = "sharp-radar")]
    #[test]
    fn fixed_iss_scene_renders_and_transfers_exactly() {
        let app = crate::coast_measurement::app("iss-ground-20", 0.0, 0.0, 20.0);
        assert_eq!(app.selected_id(), Some(25544));
        let mut coast = overhead_render::CoastLayer::prepare(app.observer());
        let mut screen = crate::coast_measurement::Screen::new();
        overhead_render::draw(
            &mut screen,
            &app,
            overhead_render::Layout::wide(),
            false,
            &mut coast,
        )
        .unwrap();
        assert!(coast.error().is_none());
        assert!(screen.on_writes > 0);
        let mut reconstructed = [0_u8; FRAME_BYTES];
        let mut chunk = 0;
        frame_pixels(false, &screen.pixels, |bytes| {
            if (1..=HEIGHT).contains(&chunk) {
                assert_eq!(bytes[0], chunk as u8);
                let y = chunk - 1;
                for (dest, &byte) in reconstructed[y * BYTES_PER_LINE..(y + 1) * BYTES_PER_LINE]
                    .iter_mut()
                    .zip(&bytes[1..=BYTES_PER_LINE])
                {
                    *dest = !byte;
                }
            }
            chunk += 1;
            Ok::<_, ()>(())
        })
        .unwrap();
        assert_eq!(chunk, HEIGHT + 2);
        assert_eq!(reconstructed, screen.pixels);
    }

    #[cfg(feature = "sharp-replay")]
    #[test]
    fn replay_stays_visible_changes_and_wraps_deterministically() {
        use overhead_core::sgp4::chrono::TimeDelta;
        let mut app = crate::coast_measurement::app("iss-ground-20", 0.0, 0.0, 5.0);
        let epoch = app.time().unwrap();
        let mut coast = overhead_render::CoastLayer::prepare(app.observer());
        let mut screen = crate::coast_measurement::Screen::new();
        let mut checksums = std::vec::Vec::new();
        let mut north = std::vec::Vec::new();
        for tick in 0..=REPLAY_FRAMES {
            let second = replay_second(tick);
            app.update(epoch + TimeDelta::seconds(second)).unwrap();
            assert_eq!(app.selected_id(), Some(25544), "replay second {second}");
            let marker = app.markers().next().expect("ISS in view").2;
            north.push(marker.north);
            overhead_render::draw(
                &mut screen,
                &app,
                overhead_render::Layout::wide(),
                false,
                &mut coast,
            )
            .unwrap();
            checksums.push(screen.checksum());
        }
        assert_eq!(checksums[0], checksums[REPLAY_FRAMES as usize]);
        assert_eq!(north[0], north[REPLAY_FRAMES as usize]);
        // Each intermediate marker moves >9px vertically in the 117px radius
        // of the wide view, with >60px total excursion (not a one-pixel flicker).
        assert!(
            north[..REPLAY_FRAMES as usize]
                .windows(2)
                .all(|pair| pair[1] - pair[0] > 0.08)
        );
        assert!(north[REPLAY_FRAMES as usize - 1] - north[0] > 0.5);
        assert!(
            checksums[..REPLAY_FRAMES as usize]
                .windows(2)
                .all(|pair| pair[0] != pair[1])
        );
        assert!(coast.error().is_none());
    }

    #[test]
    fn command_and_vcom_bits_toggle() {
        let mut bytes = std::vec::Vec::new();
        clear(true, |chunk| {
            bytes.extend_from_slice(chunk);
            Ok::<_, ()>(())
        })
        .unwrap();
        assert_eq!(bytes, [0x06, 0]);
        frame(true, false, |chunk| {
            bytes.extend_from_slice(chunk);
            Ok::<_, ()>(())
        })
        .unwrap();
        assert_eq!(bytes[2], 0x03);
    }
}
