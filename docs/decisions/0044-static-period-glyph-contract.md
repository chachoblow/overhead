# 0044 — Static period-glyph contract

Date: 2026-10-17
Status: accepted (bounded M3 contract; implemented and user visually accepted)

## Decision
Add one static `Per` spiral alongside the accepted elevation, altitude and speed
in the opt-in wide view. The user approved nominal element-set period, a fixed
logarithmic scale from 90 minutes / 2½ turns to 24 hours / one turn, endpoint
saturation, invalid-value suppression and the immediate static lifecycle below.
They selected the 2½-turn appearance after comparing it with three turns, then
authorized recording the contract, spelling out exact placement/raster rules
and implementation. The user subsequently reviewed and accepted the fresh
executable period comparisons, closing the separate visual-acceptance gate.

This bounds [0038's period direction](0038-labelled-borderless-glyph-direction.md),
not category or the animated full stack. [0041](0041-circular-elevation-only-contract.md),
[0042](0042-static-altitude-glyph-contract.md), [0043](0043-static-speed-glyph-contract.md)
and [0040's notices](0040-white-backed-bottom-left-notice-contract.md) remain unchanged.
[DESIGN](../DESIGN.md#static-period-contract) owns the retained illustrative
comparison/provenance; [simulator usage](../../sim/README.md#static-period-glyph)
owns fresh executable reproduction.

## Why
`CatalogueEntry` already retains the validated element set, including mean motion
in revolutions/day. Its reciprocal supplies a useful nominal orbit-duration cue
without an extra propagation, osculating-orbit calculation or predicted return
search. This is **element metadata**, not an instantaneous propagated period,
physical pass duration, time until arrival or countdown. It is not inferred from
altitude, TEME speed, camera motion or screen coordinates.

A fixed compressed scale covers low Earth, half-day and day-long examples without
recalibrating the symbol with selection, observer, zoom or catalogue population.
Three turns looked crowded at native size; the user preferred 2½ turns. Scale
endpoints are drawing choices, not accepted-orbit limits or production policy.

## Consequences

### Meaning and mapping
`RadarApp::selected_period_minutes()` obtains `1440 / mean_motion` from the
currently successful selected entry. Validate finite positive input and result;
an unrepresentable reciprocal is unavailable, not a new tracking failure. No
per-sample scalar, retained period history, dependency or propagation change is
needed. Although ingestion already checks mean motion, the conversion remains
defensive. Keep the unclamped nominal period available to the caller.

For finite positive period `p` in minutes:

```text
turns = 2.5 - 1.5 * ln(clamp(p, 90, 1440) / 90) / ln(16)
```

| Nominal period | Ideal turns |
|---|---:|
| ≤90 minutes | 2.5 |
| 3 hours | 2.125 |
| 6 hours | 1.75 |
| 12 hours | 1.375 |
| ≥24 hours | 1 |

Only the drawing saturates. Nonfinite, zero, negative or unavailable period
omits both `Per` and its spiral, preserving all other valid fields. There is no
zero-period label-only state, new warning condition or eligibility change.
Nearby inputs can share pixels; increasing period reduces the ideal winding,
not necessarily every individual black pixel or total ink monotonically.

### Layout and raster
Apply only to `--wide`, preserving the 352×240 radar, selection, arrows, marker ID
and hysteresis, separator, all three existing fields and notices. Square remains
default and byte-identical. Category stays blank; notices still draw last.

- Center `Per` in existing `FONT_6X10`: character cells start at `(367,198)`,
  baseline `(367,205)`. No routine number, units, box or extra separator.
- Use the intended 24×24 footprint `(364,209)`. Preserve the reviewed spiral's
  integer center `(376,220)`, inner radius 1.5px and outer radius 9px. The spiral
  is smaller than the radius-11 circles; do not expand it to fill the slot.
- The following is the exact sampled raster used by the reviewed preview.
  For every integer `k` from 0 through 4096 inclusive:

  ```text
  u = k / 4096
  r = 1.5 + 7.5*u
  angle = -π/2 + 2π*turns*(u - 1)
  x = floor(376 + r*cos(angle) + 0.5)
  y = floor(220 + r*sin(angle) + 0.5)
  ```

  Coordinates here are display-right/down. Round absolute coordinates to nearest
  pixels, half ties toward positive infinity. Draw the union of those points,
  deduplicating pixels. Use the written mapping/order with binary64 math; do not
  add a separate turn-count quantizer. Transcendental roundoff is not a promise
  of arbitrary-precision arithmetic or cross-platform physical validation.
- The curve winds clockwise outward, ending at the top `(376,211)`. Its inner
  endpoint changes angle as winding changes. There is no heading meaning.
  At most 4097 samples are evaluated; their continuous separation is <0.04px
  across this scale. Consecutive rounded points therefore remain 8-connected
  without line interpolation. The preview used Pillow lines between these
  adjacent points, which gives the same pixel union.
- Potential ink stays within `x=367..385`, `y=211..229`; not every raster reaches
  every bound. The center stays hollow. No enclosing circle, fill, radial spoke,
  antialiasing, dithering or temporal smoothing. Do not normalize each raster's
  bounding box, change sample count, or thicken it for a particular period.
- The renderer uses a local 23-row bitset (92 bytes), not a new framebuffer or
  heap allocation. Direct bounded sampling preserves the approved raster; its
  cost has **not** been measured on S3 or justified against the device's 20 Hz
  intent. Fewer samples can change pixels, so an optimization must preserve the
  contract or receive explicit visual/contract review. No target budget follows.

### State and lifecycle
- Recompute the selected readout immediately with frames, Tab selection and
  zoom-driven reselection; no clock, animation, interpolation or stale cache.
  The same element set gives the same period and raster across frames, even
  while instantaneous altitude and speed change. A different element set can
  supply a different value; no live refresh system is introduced.
- Period is independent of observer and zoom except for selection availability.
  Hiding the marker ID does not hide any of the four data fields. Arrows cannot
  supply a selected reading. A notice-covered selected marker still supplies it.
- Without a successful visible selection, clear all four labels/glyphs while
  retaining the separator. This covers missing/invalid frames, no visible
  markers and failed former selections. Another successful selected object may
  supply readings alongside explicit tracking/ingestion notices.
- Validity is field-local. Invalid period cannot hide valid elevation, altitude
  or speed, and their invalidity cannot hide valid period. Existing notice
  meanings, physical state, counts and selection policy remain unchanged.

## Verification and scope
Implementation lives in `app/src/lib.rs`, `render/src/{glyphs,lib}.rs` and
`sim/tests/snapshot.rs`. Tests cover nominal conversion and reciprocal overflow,
selected-entry traceability, constant period across frames, selection changes,
fixed mapping/endpoints/saturation, invalid suppression, independent exact-pixel
references, bounds/connectivity/hollow center, field-local validity, ID-hidden
views, missing/failed-frame and selection clearing/recovery, and warnings beside
valid fields. Independent CLI references use the original element mean motion,
not app period state; spiral references use host trigonometry/direct screen
points rather than the production libm/local bitset helper. They share the
specified sampled-curve contract, not an independent physical period prediction.

Before/after captures must preserve square PNG bytes, all wide pixels outside
the period label/footprint, and count/diagnostic reports. Run workspace check/tests
serially, formatting, app/render no-default-feature checks and strict all-target
app/render/simulator Clippy. Review fresh executable output separately from the
illustrations. No physical-display legibility, whole-device RAM, target timing
or scheduling claim follows from host tests/images.

No category semantics, animation, new CLI control, selection/collision policy,
prediction, fetching, dependency, firmware, hardware, fixture, lockfile or frozen
M2 evidence change. Historical M2 reproduction remains isolated under
[0023](0023-frozen-m2-revision-isolation.md).

### Implementation verification
Serial `cargo check --workspace`, `cargo test --workspace`, app/render
no-default-feature checks, strict all-target app/render/simulator Clippy and
formatting checks passed. The current slices have 24 app, 38 renderer, 7 simulator
unit and 9 CLI snapshot tests. The full workspace test run required a longer
command timeout; the completed rerun passed, rather than treating the interrupted
first run as success.

Fifteen fresh before/after captures preserved all three square PNGs byte-for-byte,
all twelve wide images outside the new period label/footprint and every count/
diagnostic report (normalizing only output destination paths). Cases included
selected/ID-hidden/empty standard replay, a later frame, empty demo, real low/
navigation/GEO-height objects, synthetic failures and a real selected ISS marker
under the notice backing. The documented mixed-orbit, speed and notice recipes
also ran headlessly with SDL unavailable.

A separate explicitly synthetic 90-minute orbit control produced exactly the
retained approved 2½-turn glyph and `Per` label pixels. This one-off preview parity
check is not a physical period reference or a golden-file test. Real examples
have nominal periods approximately 101.80, 717.99 and 1436.20 minutes, so their
spirals need not equal the illustrative endpoint/mock rasters. Fresh executable
comparison sheets were presented and the user confirmed acceptance, including
real mixed-orbit examples, selected/ID-hidden/no-selection and warning states.
This is separate from approval of the illustrative appearance and numeric scale.
No runtime change or new Cargo check/test run accompanied that confirmation.
Retained illustrations are not replaced by executable output. No interactive,
physical-display, firmware or target-performance validation was performed.
