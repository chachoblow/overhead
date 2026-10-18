# 0046 — Selected data-glyph transition contract

Date: 2026-10-17
Status: accepted (M3 contract; implemented and user visually accepted)

## Decision

The user approved a first transition for the four numeric glyphs (`Elev`,
`Alt`, `Spd`, `Per`) on selected-ID changes; `Cat` switches immediately. They
subsequently approved the 250 ms timing, mapped drawing-parameter interpolation,
interruption/frame-retarget behavior and immediate selection-loss/invalid-field
clearing below. The user separately authorized implementation, then reviewed
the interactive animation and said it looks good. Contract, implementation and
executable visual acceptance were separate gates.

This builds on the accepted static [0041](0041-circular-elevation-only-contract.md)–
[0045](0045-static-group-category-contract.md) fields and
[0038's direction](0038-labelled-borderless-glyph-direction.md). The accepted
static rasters and true physical inputs remain the endpoints. The 400×240 1-bit
wide comparison, square default, notices, selection, ID and category rules do
not change. [DESIGN](../DESIGN.md#selected-satellite-presentation) owns the visual
intent; [simulator usage](../../sim/README.md#static-category-glyph) reproduces
current executable *headless static* output; interactive animation is separate.

## Bounded behavior

- **Trigger:** only a change from one successful visible selected NORAD ID to
  another starts a transition. Explicit Tab and automatic reselection after
  zoom/frame updates use the same rule; do not alter the existing visible-only
  selection policy. Selection identity, marker diamond and ID label change
  immediately, as does the static Cat icon. Same-ID physical-frame changes
  outside an active transition keep their current immediate static behavior.
- **Clock and duration:** supply monotonic presentation time explicitly to the
  future transition layer, separate from the orbital UTC supplied to `RadarApp`.
  Use 250 ms, with smoothstep progress `q=t²(3−2t)` for elapsed fraction
  `t=clamp((now−start)/250 ms,0,1)`. At or after the deadline, show exactly the
  accepted static target raster. No wall-clock reads in app/render and no
  assumption that orbital replay time advances at display cadence. A single
  isolated headless snapshot stays static at its selected true state.
- **Representation:** interpolate *drawing parameters*, never physical samples:
  clamped elevation angle before 0041's pixel snapping, the continuous altitude
  inner-radius mapping before 0042's integer rounding, speed sweep before 0043's
  arc quantization, and period winding (`turns`) after 0044's logarithmic mapping.
  Quantize/rasterize each intermediate with the existing field's endpoint rules;
  preserve the fixed outer altitude circle and labels. No cross-fade, gray,
  dithering, numeric interpolation presented as truth, interpolation of orbital
  positions, or arbitrary morph between icon bitmaps. Individual intermediate
  rasters can repeat or change discontinuously by one pixel; only the scalar
  trajectory and final raster are specified as smooth/exact.
- **Rapid changes:** on A→B→C, sample the currently displayed parameter at the
  event time and retarget toward C for a fresh 250 ms. Never flash B's old
  endpoint or accumulate queued transitions. If a new true frame changes the
  selected B target *during* A→B, retarget each active valid field from its
  currently displayed parameter toward the latest B reading over the remaining
  time, retaining the original deadline. A new ID instead starts a fresh full
  duration. At equal timestamps progress does not regress; reject or defensively
  clamp a decreasing presentation timestamp rather than replaying backwards.
- **Absence/validity:** first selection from none appears at current true values
  immediately. Losing selection (including no frame, invalid UTC, tracking
  failure with no replacement or zoom clipping) clears all five fields at once,
  retaining the separator, and discards transition history. Reacquisition from
  none starts static. A missing/nonfinite field clears its own label/glyph
  immediately and cancels only its field's motion; if it becomes valid later,
  show its current value immediately rather than inventing a previous endpoint.
  Other valid fields continue. If another successful object becomes selected
  while failures or ingestion warnings are present, its fields may transition;
  the existing explicit notices still draw last and never animate.
- **Availability:** label-zoom hiding does not affect transitions. Off-screen
  arrows never supply selected data. The true selected physical readouts and
  diagnostic counts/reporting remain immediate and independent of presentation.
  No ambient motion, pulse or per-frame easing for a stable selected ID.

### Implementation seam

`RadarApp` remains clockless and owns true selected samples. The renderer's
explicit-time `Presentation` holds a selected ID and four bounded drawing
parameters/motions without retaining physical samples or reading a wall clock.
`draw_at` advances that state only for the wide viewport; existing `draw` still
renders static endpoints, including headless snapshots. The simulator supplies
one monotonic elapsed value after its frame update, independent of replay UTC.
Square diagnostics remain on the direct static path. Code/tests own API details.

## Subsequent gates

The user separately authorized implementation and subsequently confirmed that
the interactive animations look good. This fresh executable visual acceptance
is distinct from design approval; a mock or static headless snapshot would not
have demonstrated the animation.

Implementation tests should cover explicit time at 0/half/deadline,
interrupted A→B→C, same-ID frame retarget and normal immediate updates, Tab and
zoom-driven reselection, absent/invalid fields, no selection/recovery, ID hiding,
notices and square-byte parity. At transition completion all four field pixels
must equal existing static rasters; at the same selected ID/frame, category
and all pixels outside data-field slots must match the static rendering. Verify
physical samples, selected ID, counts and diagnostics never acquire interpolated
values. Use independent real-data headless selected/empty/ID-hidden snapshots
for baseline parity, plus deterministic synthetic multi-ID frames for motion; a single
snapshot cannot prove a temporal effect. Run serial workspace check/tests,
formatting, app/render no-default-feature checks and strict all-target Clippy
on code changes, then seek separate fresh visual acceptance.
No hardware, display-legibility or target 20 Hz budget claim follows.

### Implementation verification

Serial workspace check/tests, app/render no-default-feature checks, formatting
and strict all-target app/render/sim/tools Clippy passed. Renderer tests exercise
explicit timeline/Tab/zoom reselection, frame retarget, validity/loss/recovery,
square parity and final endpoint/scene isolation; existing independent real-data
headless snapshots exercise the unchanged static path. The user reviewed and
accepted the interactive animation. No physical-display or S3 timing measurement
is claimed.

Out of scope: category animation, marker slowdown or motion smoothing, a new
selection policy, normal/debug access, prediction, fetching, scheduling,
production limits, firmware, toolchain, frozen-M2 evidence or new dependencies.
