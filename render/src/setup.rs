//! Full-screen setup and info frames (0087): the BLE setup window's screens,
//! the boot-hold feedback and the info page. Text only, 1-bit; wording and
//! layout are provisional until the user reviews them on the panel.
//!
//! Non-ASCII characters (an SSID can hold any UTF-8) are drawn as `?`: the
//! fonts are ASCII only.
use core::fmt::{self, Write};
use embedded_graphics::{
    mono_font::{
        MonoFont, MonoTextStyle,
        ascii::{FONT_6X10, FONT_10X20},
    },
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};

use crate::DISPLAY_SIZE;

/// Why the setup window opened, as the waiting screen explains it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitReason {
    Unconfigured,
    JoinTimeout,
    Button,
    FactoryReset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupScreen<'a> {
    /// The knob switch has been held 3 s at boot.
    HoldForSetup,
    /// Held 10 s: releasing erases settings and data.
    HoldForReset,
    /// Erasing after a 10 s hold.
    Erasing,
    /// Advertising; `name` is the BLE name, `ssid` the stored network (for
    /// `JoinTimeout`).
    Waiting {
        name: &'a str,
        reason: WaitReason,
        ssid: &'a str,
    },
    /// Pairing: the code to type on the phone.
    Passkey { code: u32 },
    /// Paired; the app drives the rest.
    Paired,
    /// Trying the new network.
    Testing { ssid: &'a str },
    /// The new network failed; `why` is a short phrase.
    Failed { ssid: &'a str, why: &'a str },
    /// Saved; restarting on the new settings.
    Saved { ssid: &'a str },
}

/// The info page (a short knob press while running).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InfoPage<'a> {
    pub name: &'a str,
    pub version: &'a str,
    pub build: u32,
    pub channel: &'a str,
    pub mac: [u8; 6],
    /// Stored network, if any, and whether it is joined now.
    pub ssid: Option<&'a str>,
    pub wifi_joined: bool,
    /// Age of the elements on screen, if any are loaded and the time is known.
    pub data_age_s: Option<u64>,
    pub failed_build: Option<u32>,
}

const MARGIN: i32 = 16;
const BODY_TOP: i32 = 60;
const LINE: i32 = 14;
const FOOTER_TOP: i32 = 206;

pub fn draw_setup<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    screen: &SetupScreen,
) -> Result<(), D::Error> {
    let mut d = display.clipped(&Rectangle::new(Point::zero(), DISPLAY_SIZE));
    d.clear(BinaryColor::Off)?;
    match *screen {
        SetupScreen::HoldForSetup => {
            title(&mut d, "SETUP")?;
            body(
                &mut d,
                &[
                    "Release the knob now to set up.",
                    "",
                    "Keep holding to erase everything.",
                ],
            )?;
        }
        SetupScreen::HoldForReset => {
            title(&mut d, "ERASE EVERYTHING")?;
            body(
                &mut d,
                &[
                    "Release the knob now to erase the network,",
                    "location and satellite data, then set up.",
                    "",
                    "Switch off instead to keep them.",
                ],
            )?;
        }
        SetupScreen::Erasing => {
            title(&mut d, "ERASING")?;
        }
        SetupScreen::Waiting { name, reason, ssid } => {
            title(&mut d, "SETUP")?;
            let mut why = Line::<64>::new();
            match reason {
                WaitReason::Unconfigured => why.push("No network or location is saved."),
                WaitReason::JoinTimeout => {
                    why.push("Can't join ");
                    why.push_ssid(ssid);
                    why.push(".");
                }
                WaitReason::Button => why.push("Change the network or location."),
                WaitReason::FactoryReset => why.push("Everything was erased."),
            }
            let mut choose = Line::<64>::new();
            choose.push("and choose ");
            choose.push(name);
            choose.push(".");
            body(
                &mut d,
                &[
                    why.as_str(),
                    "",
                    "Open the Overhead app on your iPhone",
                    choose.as_str(),
                ],
            )?;
            if reason == WaitReason::JoinTimeout {
                footer(&mut d, "Trying the saved network again in 10 minutes.")?;
            }
        }
        SetupScreen::Passkey { code } => {
            title(&mut d, "PAIRING")?;
            body(&mut d, &["Type this code on your iPhone:"])?;
            let mut digits = Line::<8>::new();
            let _ = write!(digits, "{:06}", code % 1_000_000);
            let scale = 3;
            let w = digits.as_str().len() as i32 * FONT_10X20.character_size.width as i32 * scale;
            let mut big = Scaled {
                inner: &mut d,
                origin: Point::new((DISPLAY_SIZE.width as i32 - w) / 2, BODY_TOP + 34),
                scale,
            };
            text(&mut big, &FONT_10X20, digits.as_str(), Point::zero())?;
        }
        SetupScreen::Paired => {
            title(&mut d, "CONNECTED")?;
            body(&mut d, &["Choose your network in the app."])?;
        }
        SetupScreen::Testing { ssid } => {
            title(&mut d, "JOINING")?;
            let mut l = Line::<64>::new();
            l.push("Trying ");
            l.push_ssid(ssid);
            l.push("...");
            body(&mut d, &[l.as_str()])?;
        }
        SetupScreen::Failed { ssid, why } => {
            title(&mut d, "NOT JOINED")?;
            let mut l = Line::<64>::new();
            l.push_ssid(ssid);
            l.push(": ");
            l.push(why);
            body(&mut d, &[l.as_str(), "", "Try again in the app."])?;
        }
        SetupScreen::Saved { ssid } => {
            title(&mut d, "SAVED")?;
            let mut l = Line::<64>::new();
            l.push("Joined ");
            l.push_ssid(ssid);
            l.push(". Starting...");
            body(&mut d, &[l.as_str()])?;
        }
    }
    Ok(())
}

pub fn draw_info<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    info: &InfoPage,
) -> Result<(), D::Error> {
    let mut d = display.clipped(&Rectangle::new(Point::zero(), DISPLAY_SIZE));
    d.clear(BinaryColor::Off)?;
    title(&mut d, info.name)?;
    let mut version = Line::<64>::new();
    let _ = write!(
        version,
        "Version {} (build {}), {} updates",
        info.version, info.build, info.channel
    );
    let mut wifi = Line::<64>::new();
    wifi.push("Wi-Fi: ");
    match info.ssid {
        Some(ssid) => {
            wifi.push_ssid(ssid);
            wifi.push(if info.wifi_joined {
                ", connected"
            } else {
                ", not connected"
            });
        }
        None => wifi.push("none saved"),
    }
    let mut data = Line::<64>::new();
    match info.data_age_s {
        Some(s) if s < 120 => data.push("Satellite data: just updated"),
        Some(s) if s < 2 * 3600 => {
            let _ = write!(data, "Satellite data: {} minutes old", s / 60);
        }
        Some(s) if s < 48 * 3600 => {
            let _ = write!(data, "Satellite data: {} hours old", s / 3600);
        }
        Some(s) => {
            let _ = write!(data, "Satellite data: {} days old", s / 86_400);
        }
        None => data.push("Satellite data: none yet"),
    }
    let mut update = Line::<64>::new();
    match info.failed_build {
        Some(b) => {
            let _ = write!(update, "Last update: build {b} failed, rolled back");
        }
        None => update.push("Last update: no failures"),
    }
    let m = info.mac;
    let mut mac = Line::<64>::new();
    let _ = write!(
        mac,
        "MAC {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        m[0], m[1], m[2], m[3], m[4], m[5]
    );
    body(
        &mut d,
        &[
            version.as_str(),
            wifi.as_str(),
            data.as_str(),
            update.as_str(),
            mac.as_str(),
        ],
    )?;
    footer(&mut d, "Press the knob to go back.")?;
    text(
        &mut d,
        &FONT_6X10,
        "Hold it while switching on: 3 s to set up, 10 s to erase.",
        Point::new(MARGIN, FOOTER_TOP + LINE),
    )?;
    Ok(())
}

fn title<D: DrawTarget<Color = BinaryColor>>(d: &mut D, s: &str) -> Result<(), D::Error> {
    text(d, &FONT_10X20, s, Point::new(MARGIN, 18))?;
    Rectangle::new(
        Point::new(MARGIN, 42),
        Size::new(400 - 2 * MARGIN as u32, 2),
    )
    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
    .draw(d)
}

fn body<D: DrawTarget<Color = BinaryColor>>(d: &mut D, lines: &[&str]) -> Result<(), D::Error> {
    for (i, l) in lines.iter().enumerate() {
        text(
            d,
            &FONT_6X10,
            l,
            Point::new(MARGIN, BODY_TOP + i as i32 * LINE),
        )?;
    }
    Ok(())
}

fn footer<D: DrawTarget<Color = BinaryColor>>(d: &mut D, s: &str) -> Result<(), D::Error> {
    text(d, &FONT_6X10, s, Point::new(MARGIN, FOOTER_TOP))
}

fn text<D: DrawTarget<Color = BinaryColor>>(
    d: &mut D,
    font: &MonoFont,
    s: &str,
    at: Point,
) -> Result<(), D::Error> {
    Text::with_baseline(
        s,
        at,
        MonoTextStyle::new(font, BinaryColor::On),
        Baseline::Top,
    )
    .draw(d)?;
    Ok(())
}

/// One line of at most `N` bytes; longer writes are cut, non-ASCII becomes `?`.
struct Line<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> Line<N> {
    const fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
        }
    }
    fn push(&mut self, s: &str) {
        for c in s.chars() {
            if self.len == N {
                return;
            }
            self.buf[self.len] = if c.is_ascii() && !c.is_ascii_control() {
                c as u8
            } else {
                b'?'
            };
            self.len += 1;
        }
    }
    /// An SSID, cut to 24 characters with `...` so the line still fits.
    fn push_ssid(&mut self, ssid: &str) {
        if ssid.chars().count() <= 24 {
            self.push(ssid);
        } else {
            for c in ssid.chars().take(21) {
                let mut b = [0; 4];
                self.push(c.encode_utf8(&mut b));
            }
            self.push("...");
        }
    }
    fn as_str(&self) -> &str {
        // Only ASCII is ever stored.
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

impl<const N: usize> Write for Line<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push(s);
        Ok(())
    }
}

/// Draws each pixel as a `scale`×`scale` block from `origin` (large passkey).
struct Scaled<'a, D> {
    inner: &'a mut D,
    origin: Point,
    scale: i32,
}

impl<D: DrawTarget<Color = BinaryColor>> OriginDimensions for Scaled<'_, D> {
    fn size(&self) -> Size {
        self.inner.bounding_box().size
    }
}

impl<D: DrawTarget<Color = BinaryColor>> DrawTarget for Scaled<'_, D> {
    type Color = BinaryColor;
    type Error = D::Error;

    fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(
        &mut self,
        pixels: I,
    ) -> Result<(), D::Error> {
        let s = self.scale;
        for Pixel(p, c) in pixels {
            let at = self.origin + Point::new(p.x * s, p.y * s);
            self.inner
                .fill_solid(&Rectangle::new(at, Size::new(s as u32, s as u32)), c)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use embedded_graphics::mock_display::MockDisplay;

    /// A 400×240 bitmap for checks (MockDisplay is limited to 64×64).
    struct Bits(std::vec::Vec<bool>);
    impl Bits {
        fn new() -> Self {
            Self(std::vec![false; 400 * 240])
        }
        fn ink(&self) -> usize {
            self.0.iter().filter(|b| **b).count()
        }
        fn ink_in(&self, r: Rectangle) -> usize {
            r.points()
                .filter(|p| self.0[(p.y * 400 + p.x) as usize])
                .count()
        }
    }
    impl OriginDimensions for Bits {
        fn size(&self) -> Size {
            DISPLAY_SIZE
        }
    }
    impl DrawTarget for Bits {
        type Color = BinaryColor;
        type Error = core::convert::Infallible;
        fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(
            &mut self,
            pixels: I,
        ) -> Result<(), Self::Error> {
            for Pixel(p, c) in pixels {
                assert!(
                    (0..400).contains(&p.x) && (0..240).contains(&p.y),
                    "pixel off the panel at {p:?}"
                );
                self.0[(p.y * 400 + p.x) as usize] = c.is_on();
            }
            Ok(())
        }
    }

    const SCREENS: [SetupScreen<'static>; 13] = [
        SetupScreen::HoldForSetup,
        SetupScreen::HoldForReset,
        SetupScreen::Erasing,
        SetupScreen::Waiting {
            name: "OVERHEAD-4508",
            reason: WaitReason::Unconfigured,
            ssid: "",
        },
        SetupScreen::Waiting {
            name: "OVERHEAD-4508",
            reason: WaitReason::JoinTimeout,
            ssid: "a very long network name that is 32",
        },
        SetupScreen::Waiting {
            name: "OVERHEAD-4508",
            reason: WaitReason::Button,
            ssid: "home",
        },
        SetupScreen::Waiting {
            name: "OVERHEAD-4508",
            reason: WaitReason::FactoryReset,
            ssid: "",
        },
        SetupScreen::Passkey { code: 42 },
        SetupScreen::Passkey { code: 999_999 },
        SetupScreen::Paired,
        SetupScreen::Testing { ssid: "café" },
        SetupScreen::Failed {
            ssid: "home",
            why: "wrong password",
        },
        SetupScreen::Saved { ssid: "home" },
    ];

    #[test]
    fn every_screen_stays_on_the_panel_and_draws_ink() {
        for s in SCREENS {
            let mut b = Bits::new();
            draw_setup(&mut b, &s).unwrap();
            assert!(b.ink() > 50, "{s:?}");
        }
    }

    #[test]
    fn passkey_is_large_and_zero_padded() {
        let mut b = Bits::new();
        draw_setup(&mut b, &SetupScreen::Passkey { code: 42 }).unwrap();
        // Six 30×60 digits centred under the prompt.
        let digits = Rectangle::new(Point::new(20, BODY_TOP + 34), Size::new(360, 60));
        assert!(b.ink_in(digits) > 1500);
        let mut l = Line::<8>::new();
        let _ = write!(l, "{:06}", 42);
        assert_eq!(l.as_str(), "000042");
    }

    #[test]
    fn lines_cut_and_replace_non_ascii() {
        let mut l = Line::<64>::new();
        l.push_ssid("café ☕");
        assert_eq!(l.as_str(), "caf? ?");
        let mut l = Line::<64>::new();
        l.push_ssid("abcdefghijklmnopqrstuvwxyz0123");
        assert_eq!(l.as_str(), "abcdefghijklmnopqrstu...");
        let mut short = Line::<4>::new();
        short.push("abcdef");
        assert_eq!(short.as_str(), "abcd");
    }

    #[test]
    fn info_page_draws_every_state() {
        let base = InfoPage {
            name: "OVERHEAD-4508",
            version: "0.1.12-beta",
            build: 12,
            channel: "beta",
            mac: [0x80, 0x45, 0x6b, 0x2a, 0x45, 0x08],
            ssid: Some("home"),
            wifi_joined: true,
            data_age_s: Some(3 * 3600),
            failed_build: None,
        };
        for page in [
            base,
            InfoPage {
                ssid: None,
                wifi_joined: false,
                data_age_s: None,
                failed_build: Some(9),
                ..base
            },
            InfoPage {
                data_age_s: Some(30),
                ..base
            },
            InfoPage {
                data_age_s: Some(40 * 60),
                ..base
            },
            InfoPage {
                data_age_s: Some(5 * 86_400),
                ..base
            },
        ] {
            let mut b = Bits::new();
            draw_info(&mut b, &page).unwrap();
            assert!(b.ink() > 500);
        }
    }

    #[test]
    fn scaled_target_fills_blocks() {
        let mut m: MockDisplay<BinaryColor> = MockDisplay::new();
        let mut s = Scaled {
            inner: &mut m,
            origin: Point::new(1, 1),
            scale: 2,
        };
        s.draw_iter([Pixel(Point::new(0, 0), BinaryColor::On)])
            .unwrap();
        m.assert_pattern(&[
            "   ", //
            " ##", //
            " ##", //
        ]);
    }
}
