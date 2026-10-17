# 0041 — Circular elevation-only contract

Date: 2026-10-17
Status: accepted (bounded M3 contract; implemented and user visually accepted)

## Decision
Take one executable step toward [0038's labelled borderless stack](0038-labelled-borderless-glyph-direction.md):
replace only [0037's elevation capsule and filled glyph](0037-static-elevation-glyph-contract.md)
with a labelled circular elevation cutout and the padded separator. The user
confirmed visual acceptance of [0040's executable notices](0040-white-backed-bottom-left-notice-contract.md)
and subsequently approved this elevation-only contract, including the circle/
endpoints, label placement and retained separator without selection. The user
initially kept implementation as a separate step, then authorized it. This
contract is now implemented and replaces 0037's elevation drawing/lifecycle;
0040's notices and the square default are unchanged.
[0042](0042-static-altitude-glyph-contract.md) subsequently adds static altitude
beside elevation without changing this field's raster/lifecycle or the separator;
the elevation-only/blank-slot scope below records this original slice.

[DESIGN](../DESIGN.md#circular-elevation-only-contract) owns the retained reference
images and provenance. They are illustrative composites, not executable output.

## Why
True observer elevation already exists in selected app state. A static
one-field comparison can resolve the outline and bitmap label without inventing
category assignment, altitude normalization, speed reference frame or period
semantics. Leave the other four slots blank, rather than showing placeholders or
moving elevation into the future category slot.

## Consequences

### Layout
Coordinates are zero-based in the existing 400×240 image. Apply only to `--wide`;
square diagnostics remain default and unchanged.

- Preserve the 352×240 radar, camera, projection, arrows, visible-only selection,
  ID placement and configurable label hysteresis. No viewport resize or exclusion
  zone. The separator does not alter eligibility or clip radar geometry.
- Remove the capsule and old filled elevation glyph. Draw a 1px separator at
  `x=352`, `y=12..227`, including endpoints, even without selection. This is an
  interior divider, not an enclosing border or a reserved warning area.
- Put `Elev` in the existing `FONT_6X10`, black on white, with the four 6×10
  character cells starting at `(364,57)` (baseline `(364,64)`). No number or
  other field label. Its cell block is centered in the 48px strip.
- Keep the mock's 24×24 elevation footprint at `(364,68)`. Use integer center
  `(376,80)` and radius 11: circle ink is bounded by `x=365..387`, `y=69..91`.
  This deliberately uses a **23px ink diameter inside the 24px footprint** so
  both radial edges meet on one pixel rather than between four pixels. The
  extra blank row/column are at the top/left. This is an explicitly accepted
  raster choice, not an inference that the earlier mock settled odd/even diameter.
- All other strip space stays blank. The accepted full-stack 10px outer padding
  remains its future layout direction; an isolated elevation slot does not
  pretend to provide a full-height stack or settle the other label baselines.
- Keep 0040's notices exactly as implemented, drawn last with their white
  backing. No change to their conditions, geometry or occlusion permission.

### Meaning and raster
Use `RadarApp::selected()` and `sample.physical.look.elevation_rad` directly.
The opening grows **upward from the rightward horizontal radius**, from 0° to
90°. Draw the outline remaining around the cutout, never a filled disc or the
isolated wedge. Increasing elevation removes more of the upper-right arc; it
does not turn into a full disc at zenith.

Define glyph-local integer coordinates with origin at its center, x rightward
and y upward (display y is inverted):

1. Build the thin circle by reflecting these first-octant pixel centers across
   both axes and the diagonal, deduplicating joins:
   `(11,0), (11,1), (11,2), (11,3), (10,4), (10,5), (9,6), (8,7), (8,8)`.
   This fixes the small-circle raster independently of a library's stroke rules.
2. Clamp finite input to `[0, π/2]`. Choose the first-quadrant circle pixel with
   polar angle nearest that elevation; exact ties choose the smaller angle.
   Quantization is drawing only; never write it back into physical state.
3. If the chosen endpoint is `(11,0)`, draw the complete circle **without a
   radial line**. This includes 0° and angles below the first pixel transition;
   it avoids a duplicate spoke for a zero-width cutout.
4. Otherwise remove circle pixels strictly between angle zero and the chosen
   endpoint angle, retaining both arc endpoints. Draw inclusive, single-pixel
   Bresenham lines from `(0,0)` to `(11,0)` and to the chosen endpoint. Draw the
   union of these pixels and the retained arc: no separate stroked polygon,
   second outline, antialiasing, dithering or thickened joins.
   For deterministic line ties, start `dx=end_x`, `dy=-end_y`, `err=dx+dy`;
   each step caches `e2=2*err`, advances x when `e2>=dy` (`err+=dy`), and advances
   y when `e2<=dx` (`err+=dx`). Both conditions use that cached value.
5. At 45° the moving edge ends at `(8,8)`; at 90° it ends at `(0,11)`, leaving
   three quarters of the circle plus horizontal/vertical radial edges. At 0°
   there is only the full circle. Nearby physical angles may share a raster;
   no subpixel smoothing or temporal interpolation is implied.

Each radial is one pixel per major-axis step. Shared hub/endpoints and overlaps
at very narrow openings are a pixel union, not parallel duplicate strokes.
Finite out-of-range values saturate only for defensive rendering. Nonfinite
input must not become a plausible zero/maximum reading: omit both glyph and
`Elev`, retain the separator, and do not invent a new warning condition. The
normal engine path already supplies checked finite physical samples.

### State and lifecycle
- Update immediately on a new physical frame, Tab selection or zoom-driven
  reselection. No clock, animation, interpolation or retained value.
- Hiding the marker ID at wide FOV does not hide the elevation label/glyph.
- Without a selected sample, omit both `Elev` and the glyph; retain only the
  separator in the strip. This includes no in-view markers and invalid/missing
  frames. A failed former selection must not leave a stale reading; another
  successfully selected sample may still supply one.
- Full-frame clearing restores the blank strip on selection loss. Off-screen
  arrows never supply a selected glyph. Tracking/ingestion notices remain
  explicit even when a successful selected glyph is present.
- This establishes lifecycle for the elevation-only slice, **not** the final
  category/altitude/speed/period stack or its animation policy.

## Approval and verification gates
The user approved the contract after presentation of the native illustrative
scene, the 0°/15°/45°/75°/90° strip comparisons and enlarged raster details, and
the no-selection strip. This covers the outline/endpoints, label/slot position,
23px circle choice and retained separator. Implementation was subsequently
authorized and completed. The user then reviewed the fresh executable output
and confirmed that it looks good, closing the separate visual-acceptance gate.

Implementation verification requirements:
- Pixel tests for exact circle/line sets at endpoints and intermediate angles,
  snapping thresholds/ties, inclusive joins, 8-connected outlines, bounds,
  label/separator pixels, finite saturation and nonfinite suppression. Check
  monotonically increasing removed arc, not monotonic total black-pixel area.
- Lifecycle tests for true selected input, frame/selection changes, ID-hidden
  independence, no-selection/missing-frame clearing, valid selection alongside
  failures/warnings, and recovery without stale glyph/label ink.
- Assert unchanged radar pixels, physical samples, counts, selection and notice
  pixels; retain square-default and label/arrow regressions.
- Update real-data headless glyph assertions in `sim/tests/snapshot.rs` using
  independently calculated true elevations, including selected, no-selection
  and ID-hidden cases. Synthetic angle panels are raster cases, not physical
  measurements or substitute executable evidence.
- Run `cargo check --workspace` and app/render/simulator tests serially; then
  review fresh executable snapshots separately. No display-legibility or target
  performance claim follows from host images.

Implementation lives in `render/src/glyphs.rs`, `render/src/lib.rs`,
`sim/tests/snapshot.rs` and `sim/README.md`. Existing app state and dependencies
suffice. No core/app semantics, other glyphs, new CLI option, animation,
collision search, selection policy, debug access, prediction, fetching,
production budgets, firmware, hardware or frozen-M2 evidence changes.

### Implementation verification
Serial `cargo check --workspace`, `cargo test --workspace`, app/render
no-default-feature checks, strict all-target app/render/simulator Clippy and
formatting checks passed. The current slice has 25 renderer tests plus the
existing 21 app, 7 simulator unit and 8 CLI snapshot tests. Independent test
references construct the small circle through rounded square roots and radial
pixels through major-axis rounding, not the production table/Bresenham routine;
CLI selected elevations come from independent real-data look-angle calculations.

Twelve headless before/after captures covered square and wide weather/GPS at
0.1°/20°/100°, a later wide frame, the empty demo, and the four synthetic-fault/
real-ISS notice examples. All three square PNGs were byte-identical. All nine
wide radar regions (`x<352`), including notice pixels, were identical; differences
were confined to the strip. Counts and diagnostic reports were unchanged.
Selected, ID-hidden, empty and warning/crossing outputs were inspected; the user
subsequently accepted the presented executable output. This does not approve
additional glyphs or establish physical-display legibility. Reproduction lives
in the simulator's
[static elevation](../../sim/README.md#static-elevation-glyph) and
[notice review](../../sim/README.md#notice-overlay-review) recipes. No interactive,
physical-display or target-performance validation was performed.
