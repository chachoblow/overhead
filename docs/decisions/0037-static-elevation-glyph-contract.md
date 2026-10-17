# 0037 — Static elevation glyph contract

Date: 2026-10-17
Status: accepted (bounded M3 contract; implemented in the opt-in wide view and user visually accepted)

## Subsequent visual direction
[0038](0038-labelled-borderless-glyph-direction.md) selects a future outlined
circular elevation cutout in a labelled borderless stack. It is not implemented
and does not yet replace this bounded elevation/capsule contract or its
no-selection clearing. [0040](0040-white-backed-bottom-left-notice-contract.md)
subsequently replaces only the strip-notice placement with a bottom-left,
white-backed overlay; the original notice geometry below remains history.

## Decision
Implement one unanimated true-elevation glyph inside a shortened shared capsule
in [0033's opt-in wide view](0033-wide-radar-comparison-contract.md). The user
approved an elevation-only slice, then the concrete geometry and separate warning
area below. This bounds [0036's capsule direction](0036-shared-glyph-capsule.md)
without selecting the other glyph mappings or the eventual full-stack layout.
The user subsequently reviewed the rendered snapshot set and accepted the
current elevation/capsule treatment. This does not approve the other glyphs,
full-stack geometry or transitions, or establish physical-display legibility.

## Why
Elevation already has an agreed visual encoding and a true physical sample in
app state. It lets us test the shared enclosure and warning coexistence without
inventing category classifications, speed/period semantics or non-elevation scales.
The temporary shorter capsule leaves explicit space for notices; it is not a
replacement design for the complete five-glyph stack in the retained reference.

## Consequences

### Drawing and state contract
All coordinates are zero-based display pixels in the existing 400×240 image.

- Keep the 352×240 radar and 48-pixel strip, camera, projection, arrows, selection,
  marker labels and configurable label hysteresis unchanged. Keep the square
  diagnostic presentation as default. Apply this slice only to the existing
  opt-in wide presentation; no new launch flag or device control.
- Capsule outer bounds: origin `(355, 7)`, size `42×146`, inclusive extent
  `x=355..396`, `y=7..152`. Use a 1px rounded outline with 10px corner radius,
  black on white. Keep the stroke inside those bounds. Omit pixels `y=77..82`
  on each straight long side: one centered 6px break per side. No internal
  dividers, separate glyph box or decorative placeholder marks.
- Elevation footprint: origin `(364, 60)`, size `24×24`, inclusive extent
  `x=364..387`, `y=60..83`. Draw only the filled region below a ray from the
  square's lower-left corner, at the selected sample's true elevation above the
  horizontal. No footprint border, angle arc, field label or number.
- Rasterize by pixel centers. Within the footprint, let `u = column + 0.5` and
  `v = 23.5 - row`, measured rightward/upward from the lower-left corner. Fill
  where `v * cos(elevation) <= u * sin(elevation)`, including the boundary.
  Treat 0° as empty, 45° as the lower-right triangle including its diagonal,
  and 90° as the full square; endpoint handling must not depend on tangent
  singularities. Numerical implementation should preserve these reference cases.
  The physical domain is 0°–90°; only above-horizon visible samples can be selected.
  Defensive drawing may saturate finite values to that domain, but must not
  fabricate a glyph from a nonfinite value. Never modify physical samples.
- Read `RadarApp::selected()` and its `sample.physical.look.elevation_rad`.
  Selection and frame changes update immediately: no interpolation, retained
  history or clock. ID-label visibility does not gate the glyph or capsule.
- When selection is absent, draw neither glyph nor capsule. Clear old ink on
  every redraw, including selection loss, invalid frames and tracking failure.
  Do not retain the last selected value or choose an off-screen arrow instead.
- Reserve the strip below the capsule for existing explicit notices. Start text
  at `(356, 166)` using the existing 6×10 font and 13px baseline spacing. Preserve
  the current order and wording: `NO` / `FRAME` for absent valid-time state,
  otherwise `NO` / `MARKERS` for zero visible markers, followed by `TRACK` /
  `ERROR` and `INGEST` / `WARN` when applicable. Six lines end at baseline 231.
  Use this fixed notice position even when the capsule is absent. Warnings must
  remain visible alongside a valid selected glyph and must clear on recovery.
  `NO MARKERS` still describes only the viewport, not a complete empty sky.
- Leave category, altitude, speed and period glyphs absent. Their eventual slots,
  scales, meanings and the full-height capsule/warning allocation are not settled
  by this temporary layout. Do not copy illustrative values from the mock.

### Verification and boundaries
- Renderer pixel tests: capsule bounds/stroke/corners/side openings; glyph bounds,
  endpoint and intermediate-angle fill, monotonic area and nonfinite handling;
  no labels or footprint outline; no drawing outside the strip.
- Lifecycle tests: true selected-sample input, immediate selection/frame changes,
  independence from label hiding, absent selection and invalid-frame clearing.
- Exercise empty/missing frames, tracking errors and ingestion warnings, including
  both warning types alongside valid markers and recovery. Update the existing
  tests that currently require a blank healthy strip rather than weakening the
  warning assertions.
- Assert unchanged radar pixels, counts, physical samples and square diagnostic
  output. Retain arrow multiplicity and label/hysteresis regressions.
- Use deterministic headless snapshots from the existing local catalogues at
  explicit UTC/site/FOV, including selected and no-selection cases. Synthetic
  raster cases establish drawing behavior, not new physical evidence. The retained
  mock remains a design reference, not a golden executable image.
- After implementation, run `cargo check --workspace` and
  `cargo test -p overhead-app -p overhead-render -p overhead-sim` serially. Visual
  review of the real-data output remains separate from contract approval and
  automated tests; no physical-display or performance claim follows.

Implementation lives in `render/src/glyphs.rs`, the wide rendering path and its
tests, with headless checks in `sim/tests/snapshot.rs` and usage in `sim/README.md`.
Existing app state supplies the data without changes. The renderer now directly
uses the workspace's existing `libm` package for allocation-free no_std sine/cosine;
no new package or version is introduced. No core physics changes, animations, new
selection policy, panning, slowdown, prediction, fetching, scheduling, production
capacity/cadence, firmware or frozen-M2 evidence changes are included.
