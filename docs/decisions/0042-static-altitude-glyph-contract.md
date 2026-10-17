# 0042 — Static altitude-glyph contract

Date: 2026-10-17
Status: accepted (bounded M3 contract; implemented and user visually accepted)

## Decision
Add one static altitude field alongside [0041's implemented elevation](0041-circular-elevation-only-contract.md)
in the opt-in wide view. Use [0038's concentric-circle direction](0038-labelled-borderless-glyph-direction.md):
a fixed outer circle and an outlined inner circle that shrinks with true WGS-84
altitude. The user selected a fixed scale spanning low Earth, navigation and
geostationary heights, approved the native-size appearance/placement, then
approved the mapping and lifecycle rules below. Initial authorization covered
recording this contract and retaining its references only. The user subsequently
authorized implementation, which is now complete. They then reviewed the fresh
executable output and confirmed visual acceptance, separately from the mocks.

[DESIGN](../DESIGN.md#static-altitude-contract) owns the reviewed images and
provenance. They are illustrative composites, not executable or physical evidence.
[0043](0043-static-speed-glyph-contract.md) subsequently adds static speed without
changing altitude; the two-field/blank-speed scope below records this original slice.

## Why
True geodetic altitude is already available in selected app state and the square
diagnostic readout. A bounded altitude addition needs no category classification,
speed reference frame or period semantics. A compressed fixed scale conveys
broad height across the existing mixed replay without recalibrating the symbol
when zoom, selection or catalogue changes. Eight inner-circle sizes are a coarse
height cue, not a numeric instrument. Navigation versus geostationary heights
are deliberately a subtle 3px versus 2px inner-radius distinction.

This is a newly approved mapping for circles, not adoption of the historical
logarithmic diamond/bar preview. The 400 km compression parameter below is a
visual tuning constant, not an orbital boundary or production catalogue choice.

## Consequences

### Layout and raster
Coordinates are zero-based in the existing 400×240 image. Apply only to `--wide`;
square diagnostics remain default and unchanged.

- Preserve radar dimensions/projection, arrows, selection, marker ID and zoom
  hysteresis. Preserve 0041's elevation label/cutout and separator pixels, and
  [0040's notices](0040-white-backed-bottom-left-notice-contract.md), drawn last.
- Center `Alt` using the existing `FONT_6X10`: three 6×10 character cells start
  at `(367,104)`, baseline `(367,111)`. No altitude number, box or other new label.
- Keep the 24×24 altitude footprint at `(364,115)`. Its integer center is
  `(376,127)`. The fixed radius-11 outer circle occupies `x=365..387`,
  `y=116..138`, a 23px ink diameter with the extra blank row/column at top/left,
  matching elevation's alignment. The inner radius is an integer from 2 to 9.
- Define each circle of integer radius `r` by visiting integer `m=0..r`, choosing
  integer `n` nearest to `sqrt(r*r - m*m)`, and retaining pairs where `m<=n`.
  Reflect `(n,m)` across both axes and the diagonal; deduplicate joins. The
  square-root rounding has no exact half ties for these integer radicands.
  This specifies the preview's pixel set, not a requirement to compute square
  roots at runtime; bounded precomputed rasters are equally valid.
- Draw the union of the two circle outlines around the same center. This
  radius-11 rule exactly matches 0041's fixed outer-circle pixel set. Each inner
  outline is 8-connected and disjoint from the outer outline. No fill, spokes,
  antialiasing, dithering or extra stroke width. The smallest ring remains hollow;
  the largest never coincides with the outer ring.
- Category, speed and period slots remain blank. This is not the full stack.

### Meaning and mapping
Read `RadarApp::selected()` and `sample.physical.geodetic.altitude_km`, not camera
depth, observer altitude, slant range or distance from Earth's center. Use the
current WGS-84 ellipsoidal height, not an orbital average or a visual position.

For finite input, define:

```text
h = clamp(altitude_km, 0, 40_000)
t = ln(1 + h / 400) / ln(101)
inner_radius = floor(9 - 7*t + 0.5)
```

This is nearest-integer radius rounding, with exact half-pixel ties choosing
the larger circle. Clamp altitude before evaluating the logarithm. The mapping
is fixed, independent of view, selection history, observer and catalogue.
Quantization/saturation are drawing only; never mutate the physical sample.

| Illustrative altitude (km) | Inner radius (px) |
|---:|---:|
| 0 | 9 |
| 400 | 8 |
| 800 | 7 |
| 2,000 | 6 |
| 5,000 | 5 |
| 10,000 | 4 |
| 20,200 | 3 |
| 35,786 | 2 |
| 40,000 | 2 |

Finite negative altitude saturates at radius 9; values above 40,000 km saturate
at radius 2. Quantization already selects radius 2 above approximately 28,655 km;
40,000 km is the normalization endpoint, not the first minimum-size reading.
These are display limits, not tracking/eligibility limits. Nearby altitudes can
share a raster, and high-altitude differences can disappear entirely.

Nonfinite altitude suppresses both altitude circles and `Alt`; it must not
become a plausible zero/maximum reading. Keep any valid elevation and the
separator, and do not invent a new warning condition. The normal engine path
already supplies checked finite samples; defensive rendering does not replace
engine validation or existing explicit tracking failures.

### State and lifecycle
- Update immediately on physical frames, Tab selection and zoom-driven
  reselection. No animation, interpolation, retained altitude or drawing clock.
- Marker-ID visibility does not gate either field. Off-screen arrows never
  supply a selected reading.
- Without a selected sample, clear both field labels/glyphs, retaining the
  separator. This includes missing/invalid frames and no in-view markers.
  Full-frame clearing must remove all stale ink.
- A failed former selection cannot retain its old altitude. Another successful
  selected sample may supply the new reading while tracking notices remain.
- Altitude validity is field-local; do not suppress valid elevation because
  altitude is nonfinite or change 0041's elevation handling. No warning lifecycle,
  selection policy, physical state or counts change.
- These rules bound the static elevation-plus-altitude slice, not category,
  speed/period, full-stack lifecycle or transitions.

## Approval and verification gates
The user first approved trying the proposed mapping in a native-size comparison,
then approved the displayed circle appearance and placement. They subsequently
approved the mapping, saturation, invalid/no-selection behavior and immediate
updates, and authorized documentation/reference retention. They subsequently
authorized the bounded runtime implementation. After implementation and
verification, the user reviewed the fresh executable comparison and confirmed
that it looks good, closing the separate visual-acceptance gate.

Implementation verification requirements:
- Exact-pixel tests for all eight inner sizes, fixed outer circle, hollow centers,
  symmetry/connectivity, bounds, label placement and disjoint outlines. Preserve
  elevation and separator pixels exactly.
- Mapping tests for endpoints, representative values, each quantization boundary
  and rounding ties, monotonic nonincreasing radius, finite saturation and
  nonfinite suppression. Do not assert monotonic individual pixels as rings move.
- Lifecycle tests for frame/selection changes, ID-hidden independence, invalid
  altitude with valid elevation, missing frames/no selection, actual propagation
  failure, warnings alongside a valid selection, and recovery without stale ink.
- Independent real-data altitude assertions in `sim/tests/snapshot.rs`, deriving
  geodetic altitude separately from selected app state. Retain elevation,
  visibility/arrow, notice and square-default regressions.
- Before/after comparisons must preserve square output and all wide pixels
  outside the new altitude label/glyph, including radar, elevation, separator
  and notices. Physical samples, counts and selection must remain unchanged.
- Run `cargo check --workspace` and workspace tests serially after code changes;
  separately review fresh headless executable snapshots before claiming visual
  acceptance. Host images do not establish physical-display legibility or target
  performance.

Implementation lives in `render/src/glyphs.rs`, `render/src/lib.rs`,
`sim/tests/snapshot.rs` and `sim/README.md`. Existing app state and dependencies
suffice; no core/app semantics, new CLI control, animation, other glyphs,
collision search, prediction, fetching, scheduling, capacity, firmware, hardware
or frozen-M2 evidence change is part of this contract.

The initial design session changed no runtime code and ran no Cargo checks/tests.
Preview generation verified strictly 1-bit output, unchanged underlying radar
regions, the existing font bitmap, radius-11 parity and connected/disjoint inner
outlines. Those remain illustration checks, not implementation verification.

### Implementation verification
Serial workspace check/tests, formatting, app/render no-default-feature checks
and strict all-target app/render/simulator Clippy passed. The renderer now has
30 tests; the simulator has 7 unit and 9 CLI snapshot tests. Production circles
use exact integer half-step comparisons; independent test references use
rounded square roots. Mapping tests cover half-radius ties and bracket every
physical threshold using the inverse mapping, without pretending a rounded
inverse logarithm is an exact transcendental tie. Synthetic eccentric-orbit
controls demonstrate actual pixel changes on selection and frame updates.

CLI references calculate geodetic altitude independently of app state, covering
both standard catalogues at two times/five zooms and all 16 real pool objects
individually at their epochs. Synthetic fault controls remain explicitly labelled.
Twelve fresh before/after captures preserve all three square PNGs byte-for-byte
and all nine wide images outside the altitude label/glyph region, including radar,
elevation, separator and notices. Counts and diagnostic reports are unchanged.

The self-contained [altitude review recipe](../../sim/README.md#static-altitude-glyph)
was executed headlessly, producing low/navigation/GEO-height examples from
unmodified historical records. Selected, ID-hidden, empty/failure and
selected-plus-warning images were inspected. The user subsequently accepted the
presented executable comparison, including low/navigation/GEO-height samples,
ID-hidden, no-selection and warning states. This does not approve further glyphs
or establish physical-display legibility. No runtime change or new Cargo check/
test run accompanied that confirmation. No interactive, physical-display or
target-performance validation was performed.
