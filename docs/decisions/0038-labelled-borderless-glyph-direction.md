# 0038 — Labelled borderless glyph direction

Date: 2026-10-17
Status: accepted (M3 visual direction; five static fields implemented under 0041–0045, transitions/debug access open)

## Decision
Use a borderless selected-satellite glyph stack with short centered labels and
one padded vertical separator from the radar. Replace the earlier glyph shapes
with an outlined circular elevation cutout, concentric altitude circles, a speed
arc and a period spiral. Retain the thin navigation cross and choose the simple
outlined body-and-wings station icon (option B in the visual review).

[DESIGN's retained references](../DESIGN.md#selected-satellite-presentation) own
the current visual brief, mock provenance and layout observations. The user
accepted the final 10px top/bottom padding and selected station B, then asked to
retain the mock and record this revised direction before further work.

This amends [0032's glyph shapes and no-routine-label direction](0032-wide-radar-and-selected-satellite-glyphs.md)
and replaces [0036's shared capsule direction](0036-shared-glyph-capsule.md).
It also changes the intended elevation appearance from
[0037's filled square](0037-static-elevation-glyph-contract.md), but did **not**
itself replace that executable contract. [0040](0040-white-backed-bottom-left-notice-contract.md)
subsequently moved notices to a bottom-left overlay.
[0041](0041-circular-elevation-only-contract.md) now implements the bounded
circular elevation-only slice, label and separator.
[0042](0042-static-altitude-glyph-contract.md) subsequently specifies and implements
the static altitude mapping/raster and two-field lifecycle after illustrative
review and separate authorization; the user subsequently accepted its executable output.
[0043](0043-static-speed-glyph-contract.md) now implements static TEME speed
after contract/illustration approval and implementation authorization; the user
subsequently accepted its fresh executable output.
[0044](0044-static-period-glyph-contract.md) now implements nominal period after
meaning/scale/appearance/lifecycle approval and implementation authorization;
the user subsequently accepted its fresh executable output.
[0045](0045-static-group-category-contract.md) now implements the approved static
category icons, explicit group-role mapping, priority and five-field lifecycle;
the user subsequently accepted its fresh executable output. Transitions and final
debug access remain open. Square diagnostics remain default.

## Why
The user iterated through circular glyphs, outlined elevation, continuous
capsules, separate category/data enclosures and then removed the enclosures.
The selected treatment keeps the thin glyphs together spatially without boxes.
Short labels help distinguish otherwise ambiguous instruments; they were
restored after comparison with the unlabelled stack. Equal outer padding makes
the label-plus-glyph stack, not just the symbols, sit evenly on the display.
The elevation wedge was refined to avoid doubled raster edges. A station body
with straight wings was preferred to detailed solar-panel diagrams.

## Consequences
- Preserve the implemented wide radar, camera, arrows and visible-only selection
  under [0033](0033-wide-radar-comparison-contract.md), plus marker identification
  and its configurable zoom hysteresis under
  [0034](0034-selected-marker-identification.md) /
  [0035](0035-label-zoom-visibility.md). The separator is not a new radar perimeter.
- Stack category, elevation, altitude, orbital speed and orbital period, with
  centered `Cat`, `Elev`, `Alt`, `Spd`, `Per` above them. Use black-on-white,
  strictly 1-bit marks, no enclosing borders or routine numeric values. Use a
  crisp bitmap font in the next executable comparison rather than copying the
  temporary mock font's poorly rasterized small letters.
- Elevation is the **outline of the circle remaining after an angular cutout**,
  not a filled disc or the isolated wedge. The reviewed example is a 45° cutout
  opening upward from the rightward horizontal radius, with single-pixel radial
  edges. It encodes true observer elevation; 0041 subsequently specifies and
  implements the endpoint and raster behavior.
- Altitude uses a fixed outer circle and an outlined inner circle that shrinks
  with increasing true WGS-84 altitude. Speed uses increasing circle-outline
  coverage, from absent at zero to complete at maximum; the mock starts at the
  top and grows clockwise. Period uses a spiral rather than a coil, with more
  turns for a shorter period. This decision selects visual encodings, not
  scales; 0042 subsequently supplies the approved altitude scale and 0043 the
  approved instantaneous TEME speed meaning and fixed linear scale. 0044 supplies
  nominal element-set period and its fixed logarithmic 2½-to-one-turn mapping.
- The category observations below record the original visual review; 0045 now
  settles the previously open assignment, overlap and raster rules.
  Navigation is a 1px square-ended cross. Station is a small outlined body with
  single-line horizontal wings. Weather-cloud and unknown-question-mark icons
  were shown but not explicitly selected. Category assignment, overlapping/
  unknown membership and operational-status interpretation remain unresolved;
  group membership alone does not establish purpose or active status.
- The initial logarithmic diamond/bar altitude preview is not the selected
  shape or an approved mapping for the concentric circles. Filled elevation,
  the isolated wedge, capsules, unlabelled variants and other station options
  were intermediate exploration, not parallel approved implementations.
- Retaining this visual direction does not authorize coding the full stack.
  0042 subsequently bounds altitude ranges, normalization, rounding, saturation,
  invalid values, geometry and static two-field lifecycle. 0043 subsequently
  bounds static speed semantics, scale, raster and three-field lifecycle.
  0044 subsequently bounds period semantics, scale, raster and static four-field
  lifecycle. 0045 subsequently bounds category rules/geometry and static
  five-field lifecycle/no-selection behavior. Transitions remain unspecified.
  Existing true-state readouts must remain separate from any future animation.
- The full-height stack occupies the old warning area. Explicit tracking and
  ingestion warnings must remain visible, including with valid markers.
  A reserved radar-corner notice area was initially recommended but not approved
  or mocked. [0039](0039-bottom-left-condition-driven-notices.md) subsequently
  selects bottom-left placement and condition-driven notices after mock review.
  [0040](0040-white-backed-bottom-left-notice-contract.md) subsequently bounds and
  implements the white-backed overlay independently of the revised glyphs.
  Do not silently remove notices or treat a failed sky as complete.
  Keeping the existing no-selection clearing rule was the recommendation here;
  0045 subsequently accepts it for the five static fields.
- No new animation, selection policy, debug-access control, panning, slowdown,
  prediction, fetching, scheduling, production capacity, dependency, hardware
  or frozen-M2 evidence change. No physical-display legibility or performance
  claim follows from these mocks.

## Retained evidence
The two native 400×240 PNGs linked from DESIGN are the final labelled navigation
mock and the same layout with the selected station-B icon. Both are illustrative
composites over the same replay radar, not executable snapshots, golden tests or
physically consistent readings. No temporary scripts or chat images are needed
to recover the chosen direction. Earlier references and decisions remain as
history; the evolving executable is still documented in
[simulator usage](../../sim/README.md#static-elevation-glyph).
