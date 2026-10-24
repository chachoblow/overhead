//! Fixed-observer coastline layer (decision 0058). Prepared once for the
//! configured observer, then reused as satellites move and the lens zooms.
//!
//! Preparation validates the embedded asset and observer, builds the observer's
//! camera transform/facing plane, and records which packed pieces have any
//! front-facing interval. Drawing applies the current lens to exactly the same
//! per-piece arithmetic as `candidate_draw`, skipping only pieces that the
//! candidate would classify as wholly hidden. No heap and no retained curves.
//!
//! Decision 0059 adds one cached 352x240 1-bit raster of the current view.
//! The coastline depends only on observer and lens, so frames with an unchanged
//! view copy cached pixels; zoom (or a failed raster) triggers a fresh raster.
//! A raster that fails part-way never reaches the display. This does not claim
//! a device frame-time, stack or 0053 all-view result.
use super::*;
use crate::Layout;

/// A `static`, not a `const`: one flash copy however many call sites use it
/// (a `const` was duplicated per codegen unit on the S3).
pub(crate) static ASSET: &[u8] =
    include_bytes!("../../../assets/coastline-110m-40km-quadratic.bin");
/// Packed pieces in the pinned asset; a different count is rejected.
pub(crate) const PIECES: usize = 2031;
const WORDS: usize = PIECES.div_ceil(32);
const MASK_WIDTH: usize = 352;
const MASK_HEIGHT: usize = 240;
const MASK_BYTES: usize = MASK_WIDTH * MASK_HEIGHT / 8;

/// Observer-keyed coastline geography for the wide view. Preparation failure is
/// retained explicitly so the frame shows `COAST ERROR` rather than an empty sea.
/// About 11 KB, dominated by the current-view raster: keep it in long-lived
/// storage (static or owning struct), not a transient stack frame.
pub struct CoastLayer {
    state: Result<Prepared, &'static str>,
    /// View whose complete, successful raster is in `mask`.
    cached: Option<View>,
    mask: Mask,
    #[cfg(test)]
    rasters: usize,
}

/// 1-bit radar-sized raster. Writes outside the wide radar are a bug and
/// invalidate the raster rather than being silently dropped.
struct Mask {
    bits: [u8; MASK_BYTES],
    outside: bool,
}

impl Mask {
    fn clear(&mut self) {
        self.bits = [0; MASK_BYTES];
        self.outside = false;
    }
    fn lit(&self) -> impl Iterator<Item = Point> + '_ {
        // Most of the raster is empty: skip zero bytes before testing bits.
        let set = self.bits.iter().enumerate().filter(|(_, byte)| **byte != 0);
        set.flat_map(|(i, &byte)| {
            (0..8)
                .filter(move |bit| byte & (1 << bit) != 0)
                .map(move |bit| {
                    let n = i * 8 + bit;
                    Point::new((n % MASK_WIDTH) as i32, (n / MASK_WIDTH) as i32)
                })
        })
    }
}

impl OriginDimensions for Mask {
    fn size(&self) -> Size {
        Size::new(MASK_WIDTH as u32, MASK_HEIGHT as u32)
    }
}

impl DrawTarget for Mask {
    type Color = BinaryColor;
    type Error = Infallible;
    fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(
        &mut self,
        pixels: I,
    ) -> Result<(), Infallible> {
        for Pixel(p, color) in pixels {
            if !(0..MASK_WIDTH as i32).contains(&p.x) || !(0..MASK_HEIGHT as i32).contains(&p.y) {
                self.outside = true;
                continue;
            }
            let n = p.y as usize * MASK_WIDTH + p.x as usize;
            if color == BinaryColor::On {
                self.bits[n / 8] |= 1 << (n % 8);
            } else {
                self.bits[n / 8] &= !(1 << (n % 8));
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
struct Prepared {
    site: GeodeticPosition,
    context: Context,
    front: [u32; WORDS],
}

impl CoastLayer {
    /// Unprepared layer; draws show `COAST ERROR` until `prepare_for` succeeds.
    /// `const` so long-lived storage can be initialized without a stack temporary.
    pub const fn new() -> Self {
        Self {
            state: Err("not prepared"),
            cached: None,
            mask: Mask {
                bits: [0; MASK_BYTES],
                outside: false,
            },
            #[cfg(test)]
            rasters: 0,
        }
    }

    /// Convenience for owners that can hold an 11 KB value on the stack (host).
    pub fn prepare(observer: GeodeticPosition) -> Self {
        let mut layer = Self::new();
        layer.prepare_for(observer);
        layer
    }

    /// One-time, allocation-free preparation in place for `observer`, replacing
    /// any previous observer and discarding its raster. A layer refuses to draw
    /// for any other observer rather than show stale geography.
    pub fn prepare_for(&mut self, observer: GeodeticPosition) {
        self.cached = None;
        self.state = Prepared::new(observer, ASSET);
    }

    /// The preparation failure, if any.
    pub fn error(&self) -> Option<&'static str> {
        self.state.as_ref().err().copied()
    }

    /// Observer this layer was successfully prepared for.
    pub fn observer(&self) -> Option<GeodeticPosition> {
        self.state.as_ref().ok().map(|p| p.site)
    }

    /// Force the next draw to raster afresh (measurement of a zoom step).
    #[cfg(any(test, feature = "measurement"))]
    pub(crate) fn invalidate(&mut self) {
        self.cached = None;
    }

    /// Draw On pixels for the coastline (the caller has cleared the frame).
    /// `Ok(false)` is a coastline-only failure (unprepared, wrong observer or
    /// layout, numeric failure); no coastline ink is written in that case.
    pub(crate) fn draw<D: DrawTarget<Color = BinaryColor>>(
        &mut self,
        target: &mut D,
        observer: GeodeticPosition,
        view: View,
        layout: Layout,
    ) -> Result<bool, D::Error> {
        let Ok(prepared) = &self.state else {
            return Ok(false);
        };
        // The flattening raster is fixed to the 352x240 wide radar.
        if prepared.site != observer
            || (layout.radar_width, layout.radar_height) != (MASK_WIDTH as u32, MASK_HEIGHT as u32)
        {
            return Ok(false);
        }
        if self.cached != Some(view) {
            self.cached = None;
            self.mask.clear();
            #[cfg(test)]
            {
                self.rasters += 1;
            }
            let rastered = prepared.draw(&mut self.mask, view);
            if rastered.is_err() || self.mask.outside {
                self.mask.clear();
                return Ok(false);
            }
            self.cached = Some(view);
        }
        target.draw_iter(self.mask.lit().map(|p| Pixel(p, BinaryColor::On)))?;
        Ok(true)
    }
}

impl Default for CoastLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Prepared {
    fn new(site: GeodeticPosition, bytes: &[u8]) -> Result<Self, &'static str> {
        if !decode::checked(bytes) {
            return Err("invalid coast asset");
        }
        Camera::new(site).map_err(|_| "invalid observer")?;
        // The lens is replaced at draw time; neither facing nor transform uses it.
        let context = Context::new(site, View::new(20.0).unwrap());
        if !context
            .rows
            .iter()
            .flatten()
            .chain(&context.offset)
            .chain(&context.facing)
            .all(|x| x.is_finite())
        {
            return Err("invalid observer");
        }
        let mut front = [0_u32; WORDS];
        let mut index = 0;
        for part in 0..decode::PARTS {
            let range = decode::part_range(bytes, part);
            for at in (range.start..range.end - 9).step_by(15) {
                if index == PIECES {
                    return Err("invalid coast asset");
                }
                let f = decode::decode_fast(bytes, at).map(|p| context.front(p));
                if f.iter().any(|x| !x.is_finite()) {
                    return Err("invalid facing");
                }
                // Same hidden test as render_piece: all endpoints/controls <= 0.
                if f.iter().any(|x| *x > 0.0) {
                    front[index / 32] |= 1 << (index % 32);
                }
                index += 1;
            }
        }
        if index != PIECES {
            return Err("invalid coast asset");
        }
        Ok(Self {
            site,
            context,
            front,
        })
    }

    fn draw<D: DrawTarget<Color = BinaryColor>>(
        &self,
        target: &mut D,
        view: View,
    ) -> Result<(), Fail<D::Error>> {
        let context = self.context.with_view(view);
        let mut counts = Counts::default();
        let mut index = 0;
        for part in 0..decode::PARTS {
            let range = decode::part_range(ASSET, part);
            for at in (range.start..range.end - 9).step_by(15) {
                if self.front[index / 32] & (1 << (index % 32)) != 0 {
                    render_piece(
                        target,
                        &context,
                        decode::decode_fast(ASSET, at),
                        &mut counts,
                    )?;
                }
                index += 1;
            }
        }
        Ok(())
    }

    #[cfg(test)]
    fn front_count(&self) -> usize {
        self.front.iter().map(|w| w.count_ones() as usize).sum()
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use crate::coast::{cost_preflight::Screen, work_review};
    use std::{println, time::Instant};

    fn wide(fov: f64) -> View {
        View::rectangular(fov, Layout::wide().aspect_ratio()).unwrap()
    }

    #[test]
    fn embedded_asset_is_the_regenerated_candidate() {
        assert_eq!(packing::prepare().bytes, ASSET);
    }

    #[test]
    fn prepared_layer_matches_candidate_across_sites_and_zoom() {
        let layout = Layout::wide();
        let mut sites: std::vec::Vec<_> = work_review::cases().map(|(_, site, _)| site).to_vec();
        sites.push(GeodeticPosition {
            altitude_km: 2.187,
            ..crate::coast::cost_preflight::observer(39.007, -104.883)
        });
        for site in sites {
            let mut layer = CoastLayer::prepare(site);
            assert_eq!(layer.error(), None);
            assert_eq!(layer.observer(), Some(site));
            for fov in [0.1, 1.0, 5.0, 20.0, 60.0, 100.0] {
                let mut fresh = Screen::new();
                let mut reused = Screen::new();
                candidate_draw(&mut fresh, site, wide(fov), ASSET).unwrap();
                assert!(layer.draw(&mut reused, site, wide(fov), layout).unwrap());
                assert_eq!(fresh.pixels, reused.pixels, "fov={fov}");
            }
        }
    }

    #[test]
    fn invalid_inputs_and_other_observers_never_draw_geography() {
        let site = work_review::cases()[0].1;
        let other = work_review::cases()[5].1;
        let layout = Layout::wide();
        assert!(Prepared::new(site, &ASSET[..ASSET.len() - 1]).is_err());
        let mut truncated = ASSET.to_vec();
        truncated[0] ^= 1;
        assert!(Prepared::new(site, &truncated).is_err());
        let mut invalid = CoastLayer::prepare(GeodeticPosition {
            latitude_rad: f64::NAN,
            ..site
        });
        assert_eq!(invalid.error(), Some("invalid observer"));
        let mut screen = Screen::new();
        assert!(!invalid.draw(&mut screen, site, wide(20.0), layout).unwrap());

        let mut layer = CoastLayer::prepare(site);
        assert!(!layer.draw(&mut screen, other, wide(20.0), layout).unwrap());
        assert!(
            !layer
                .draw(
                    &mut screen,
                    site,
                    View::new(20.0).unwrap(),
                    Layout::new(240).unwrap()
                )
                .unwrap()
        );
        assert!(screen.pixels.iter().flatten().all(|&p| p == 0));

        // A numeric failure surfaces as a coast-only failure, not a panic or
        // target error; the frame compositor owns rollback of any partial ink.
        let mut broken = CoastLayer::prepare(site);
        if let Ok(prepared) = &mut broken.state {
            prepared.context.rows[0][0] = f32::NAN;
        }
        assert!(!broken.draw(&mut screen, site, wide(20.0), layout).unwrap());
        assert!(screen.pixels.iter().flatten().all(|&p| p == 0));
        assert_eq!(broken.cached, None);
    }

    #[test]
    fn current_view_raster_is_reused_until_zoom_changes() {
        let site = work_review::cases()[0].1;
        let layout = Layout::wide();
        let mut layer = CoastLayer::prepare(site);
        let direct = |fov: f64| {
            let mut screen = Screen::new();
            candidate_draw(&mut screen, site, wide(fov), ASSET).unwrap();
            screen.pixels
        };
        for (fov, rasters) in [
            (20.0, 1),
            (20.0, 1),
            (20.0, 1),
            (5.0, 2),
            (5.0, 2),
            (20.0, 3),
        ] {
            let mut screen = Screen::new();
            assert!(layer.draw(&mut screen, site, wide(fov), layout).unwrap());
            assert_eq!(screen.pixels, direct(fov), "fov={fov}");
            assert_eq!(layer.rasters, rasters, "fov={fov}");
        }
        // A refused draw (other observer) neither uses nor disturbs the cache.
        let mut screen = Screen::new();
        let other = work_review::cases()[5].1;
        assert!(!layer.draw(&mut screen, other, wide(20.0), layout).unwrap());
        assert!(screen.pixels.iter().flatten().all(|&p| p == 0));
        assert!(layer.draw(&mut screen, site, wide(20.0), layout).unwrap());
        assert_eq!(layer.rasters, 3);
        // A failed raster invalidates the cache and retries on the next frame.
        if let Ok(prepared) = &mut layer.state {
            prepared.context.rows[0][0] = f32::NAN;
        }
        assert!(!layer.draw(&mut screen, site, wide(10.0), layout).unwrap());
        assert_eq!(layer.cached, None);
        assert!(!layer.draw(&mut screen, site, wide(10.0), layout).unwrap());
        assert_eq!(layer.rasters, 5);
    }

    #[test]
    fn in_place_preparation_replaces_observer_and_cache() {
        let site = work_review::cases()[0].1;
        let other = work_review::cases()[5].1;
        let layout = Layout::wide();
        let mut layer = CoastLayer::new();
        assert_eq!(layer.error(), Some("not prepared"));
        let mut screen = Screen::new();
        assert!(!layer.draw(&mut screen, site, wide(20.0), layout).unwrap());
        layer.prepare_for(site);
        assert!(layer.draw(&mut screen, site, wide(20.0), layout).unwrap());
        layer.prepare_for(other);
        assert_eq!(layer.cached, None);
        assert!(!layer.draw(&mut screen, site, wide(20.0), layout).unwrap());
        let mut fresh = Screen::new();
        let mut reused = Screen::new();
        candidate_draw(&mut fresh, other, wide(20.0), ASSET).unwrap();
        assert!(layer.draw(&mut reused, other, wide(20.0), layout).unwrap());
        assert_eq!(fresh.pixels, reused.pixels);
    }

    #[test]
    fn target_errors_propagate_instead_of_rolling_back() {
        struct Refuse;
        impl OriginDimensions for Refuse {
            fn size(&self) -> Size {
                crate::DISPLAY_SIZE
            }
        }
        impl DrawTarget for Refuse {
            type Color = BinaryColor;
            type Error = ();
            fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(
                &mut self,
                _: I,
            ) -> Result<(), ()> {
                Err(())
            }
        }
        let site = work_review::cases()[0].1;
        let mut layer = CoastLayer::prepare(site);
        assert_eq!(
            layer.draw(&mut Refuse, site, wide(20.0), Layout::wide()),
            Err(())
        );
    }

    #[test]
    #[ignore = "explicit host timing/size diagnostic; not an S3 forecast"]
    fn coast_prepared_layer_report() {
        println!(
            "layer_bytes={} prepared_bytes={}",
            core::mem::size_of::<CoastLayer>(),
            core::mem::size_of::<Prepared>()
        );
        println!(
            "case,prepare_us,front_pieces,zoom,raster_draw_us,cached_draw_us,candidate_us,lit"
        );
        for (name, site, fov) in work_review::cases() {
            let start = Instant::now();
            let mut layer = CoastLayer::prepare(site);
            let prepare_us = start.elapsed().as_secs_f64() * 1e6;
            let front = layer.state.as_ref().unwrap().front_count();
            for zoom in [fov, 0.1, 5.0, 20.0, 100.0] {
                layer.cached = None;
                let mut screen = Screen::new();
                let start = Instant::now();
                assert!(
                    layer
                        .draw(&mut screen, site, wide(zoom), Layout::wide())
                        .unwrap()
                );
                let raster_us = start.elapsed().as_secs_f64() * 1e6;
                let start = Instant::now();
                assert!(
                    layer
                        .draw(&mut screen, site, wide(zoom), Layout::wide())
                        .unwrap()
                );
                let cached_us = start.elapsed().as_secs_f64() * 1e6;
                let mut fresh = Screen::new();
                let start = Instant::now();
                candidate_draw(&mut fresh, site, wide(zoom), ASSET).unwrap();
                let candidate_us = start.elapsed().as_secs_f64() * 1e6;
                println!(
                    "{name},{prepare_us:.1},{front},{zoom},{raster_us:.1},{cached_us:.1},{candidate_us:.1},{}",
                    layer.mask.lit().count()
                );
            }
        }
    }
}
