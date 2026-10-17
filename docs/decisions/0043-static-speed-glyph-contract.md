# 0043 — Static speed-glyph contract

Date: 2026-10-17
Status: accepted (bounded M3 contract; implemented and user visually accepted)

## Decision
Add one static speed field beside [0041's elevation](0041-circular-elevation-only-contract.md)
and [0042's altitude](0042-static-altitude-glyph-contract.md) in the opt-in wide
view. Use [0038's clockwise circle-outline arc](0038-labelled-borderless-glyph-direction.md)
with a bitmap `Spd` label. The user approved instantaneous TEME velocity magnitude,
a fixed linear 0–12 km/s comparison, the displayed appearance/placement and zero
label, then the remaining raster/lifecycle rules and runtime implementation.
The user subsequently reviewed and accepted the fresh executable output,
closing the separate visual-acceptance gate.

[DESIGN](../DESIGN.md#static-speed-contract) owns the approved illustrations and
provenance; [simulator usage](../../sim/README.md#static-speed-glyph) owns executable
reproduction. The illustrations are not physical measurements or golden tests.

## Why
The intended field is orbital speed, not ground speed or apparent motion. The
engine already exposes timestamp-bound TEME velocity in km/s, checked against
independent Vallado vectors. Its magnitude needs no new frame conversion,
dependency, orbit approximation or propagation call. Unlike altitude, the app
previously discarded the required input; it now retains one scalar true speed.
[0006](0006-sgp4-crate-and-conventions.md), [0007](0007-coordinate-implementation-conventions.md)
and [0009](0009-core-time-and-frame-api.md) remain unchanged. In particular,
rotating a position is not a valid Earth-fixed velocity conversion.

A fixed linear scale makes relative speeds comparable across selections and
zoom. Twelve km/s is an approved drawing endpoint, not a physical speed limit,
production catalogue constraint or a claim that every tracked object fits it.

## Consequences

### Meaning and mapping
Read the magnitude of `TemeState::velocity()` from the same propagation used for
the physical frame. Store it as `PhysicalState::teme_speed_km_s`; do not infer it
from camera motion, consecutive frames, observer range rate, altitude or orbital
period. Use a scaled norm (nested `hypot`) rather than overflow-prone component
squares. The value is independent of observer, zoom, ID visibility and catalogue
population. Existing position, look angles, counts and selection are unchanged.

For finite speed `v` in km/s, the ideal clockwise sweep from the top is:

```text
sweep = 2π * clamp(v, 0, 12) / 12
```

- At zero (including defensive finite negatives), retain `Spd` with no arc.
- Any positive speed draws at least the top circle pixel.
- At or above 12 km/s draw the complete circle. Saturation is rendering only;
  the retained physical speed is never clamped.
- Quantize the endpoint as specified below. Nearby speeds share a raster;
  the last quantization bucket is already a full circle slightly below 12.
- Nonfinite speed omits both `Spd` and its arc, preserving valid elevation and
  altitude. Do not invent a new warning or change tracking eligibility. SGP4
  already rejects nonfinite velocity components; if the magnitude of finite
  components itself overflows, it is unavailable to this readout rather than a
  new failure of an otherwise successful position sample.

### Layout and raster
Apply only to `--wide`; square diagnostics remain default and unchanged.

- Preserve radar/projection, arrows, selection, marker ID/hysteresis, elevation,
  altitude, separator and [0040's notices](0040-white-backed-bottom-left-notice-contract.md).
  Notices still draw last. No extra warning area or collision policy.
- Center `Spd` using existing `FONT_6X10`: three character cells start at
  `(367,151)`, baseline `(367,158)`. No routine number, units or box.
- Use the 24×24 footprint at `(364,162)`, integer center `(376,174)` and radius
  11. Full-circle ink occupies `x=365..387`, `y=163..185`: the same 23px circle
  and extra blank top/left row/column as elevation and altitude.
- Use exactly 0041's radius-11 circle pixel set. Order its 64 unique pixels
  clockwise from the top, assigning angles in `[0,2π)`. The first quadrant
  includes top `(0,11)` through `(11,1)` in glyph-local right/up coordinates;
  rotate it clockwise for the remaining quadrants. Each adjacent pair,
  including the closing join, is 8-connected.
- For positive submaximum speed, choose the endpoint nearest the ideal sweep
  in **unwrapped** angle, ties toward the shorter arc. Draw the inclusive
  prefix from the top through that endpoint. Never wrap a nearly full arc
  back to the first pixel. Midpoint thresholds in km/s are
  `12 * ((angle_previous + angle_next) / 2) / (2π)`; `<=` chooses the earlier
  endpoint, including exact representable threshold ties.
- Draw only those circle pixels: no radial line, fill, moving pointer,
  antialiasing, dithering or second stroke. Every nonempty prefix is connected;
  increasing speed never removes an arc pixel.
- Category and period remain blank. This is not a full-stack implementation.

### State and lifecycle
- Update immediately with physical frames, Tab selection and zoom-driven
  reselection; no clock, animation, interpolation or retained speed value.
- ID hiding never gates the three data fields. Arrows cannot supply a selected
  reading. A selected marker covered by a notice still supplies its fields.
- Without selection, clear all three labels/glyphs, retaining the separator.
  Invalid/missing frames, no visible markers and failed former selections
  cannot leave stale ink. A different successful selection may provide new
  readings while tracking notices remain visible.
- Validity is field-local. Invalid speed cannot hide valid elevation/altitude,
  nor can invalid elevation/altitude hide valid speed. Existing notice meanings
  and recovery behavior remain unchanged.

## Verification and scope
Implementation lives in `app/src/lib.rs`, `render/src/{glyphs,lib}.rs` and
`sim/tests/snapshot.rs`. Tests cover magnitude traceability, observer/zoom
independence, large/small-component norm behavior, every snapping threshold and
its ties, endpoints/saturation, exact arc pixels, bounds, connectivity, monotonic
growth, zero-label versus invalid suppression, field-local validity, actual
frame/selection pixel changes, ID-hidden independence, loss/failure/recovery and
warnings alongside valid fields. Independent CLI references derive speed from
TEME velocity outside app state and classify the circle independently of the
production octant traversal.

Before/after headless captures must preserve square PNG bytes, all wide pixels
outside the new speed slot and count/diagnostic reports. Run workspace check/tests
serially, plus formatting, app/render no-default-feature checks and strict
app/render/simulator Clippy. Fresh executable images need separate user review;
neither host images nor these tests establish physical-display legibility,
whole-device RAM or target timing.

No new dependency, CLI control, core propagation/frame semantics, category,
period, animation, prediction, scheduling, production capacity, firmware,
hardware or frozen-M2 evidence change. Adding one scalar per app sample is not a
new measured RAM or operating-budget result. Reproduction of historical M2
remains isolated at [0023's pinned revision](0023-frozen-m2-revision-isolation.md).

### Implementation verification
Serial `cargo check --workspace`, `cargo test --workspace`, formatting,
app/render no-default-feature checks and strict all-target app/render/simulator
Clippy passed. There are 22 app tests, 34 renderer tests, 7 simulator unit tests
and 9 CLI snapshot tests. Synthetic eccentric-orbit controls demonstrate actual
speed-raster changes on both Tab selection and physical-frame updates; existing
failure/recovery and warning tests now check all three fields.

Fifteen fresh before/after captures preserved all three square PNGs byte-for-byte,
all twelve wide images outside the speed label/glyph slot, and every count and
diagnostic report. The standard replay comparisons include selected/ID-hidden,
empty and later frames; separate cases cover actual low/navigation/GEO-height
objects, synthetic faults and a real selected marker under the notice mask.

The documented speed, mixed-orbit and notice recipes ran headlessly with SDL
unavailable. Fresh executable images were inspected and presented for review;
representative real TEME speeds were approximately 7.427, 3.883 and 3.087 km/s.
Those readings differ from the illustrative mock values. The user subsequently
accepted the fresh executable comparison, including the mixed-orbit samples,
ID-hidden, no-selection and warning states, separately from the contract/mocks.
No runtime change or new Cargo check/test run accompanied that confirmation.
No interactive or physical-device validation was performed, and retained contract
PNGs were not replaced with executable output. This acceptance does not authorize
category, period or animation.
