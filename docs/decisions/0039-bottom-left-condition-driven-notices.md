# 0039 — Bottom-left condition-driven notices

Date: 2026-10-17
Status: accepted (M3 placement/lifecycle; implemented under 0040; empty status refined by 0048)

## Decision
Place explicit status notices at the bottom-left of the radar when implementing
[0038's revised glyph stack](0038-labelled-borderless-glyph-direction.md), rather
than in its occupied right-hand strip. The user selected comparison B over the
top-left alternative and accepted the explanation of condition-driven notices:
no timeout or acknowledgement button; unresolved warnings remain visible.

[DESIGN](../DESIGN.md#bottom-left-notice-reference) owns the retained mock images,
provenance and layout observations. This accepts placement and existing notice
conditions, not an exact raster or overlap-handling contract.
[0040](0040-white-backed-bottom-left-notice-contract.md) subsequently bounds and
implements the white-backed bottom-left overlay, retaining
[0037's elevation-only capsule](0037-static-elevation-glyph-contract.md).

## Why
The revised full-height stack occupies the former warning area. Bottom-left
placement keeps notices separate from the north indicator and frees the strip;
it was selected over the top-left comparison. Condition-driven clearing avoids
presenting incomplete tracking as healthy merely because a timeout expired.

## Consequences

### Notice conditions
Preserve the current meanings when relocating notices:

- `NO MARKERS`: a valid-time frame has no in-view markers. Off-screen arrows
  may remain; this is not a claim that the entire sky is empty. Remove it when
  motion or zoom brings a marker into view. [0048](0048-refined-empty-view-notices.md)
  later refines this wide-view status into bounded empty-view subtypes; the
  behavior recorded here describes the original implemented M3 slice.
- `TRACK ERROR`: at least one catalogue object failed its current tracking
  calculation. Show it alongside valid markers; remove it on a subsequent
  calculation frame with no failures. Merely zooming does not repair a failure.
- `INGEST WARN`: catalogue loading produced diagnostics, such as omitted source
  records. Retain it until a clean catalogue load replaces that input. The current
  simulator loads once, so this notice persists for that run; no live reload or
  fetching is introduced.
- `NO FRAME`: no valid-time frame is available. Use it instead of `NO MARKERS`,
  and remove it once a valid frame exists. The renderer supports this state;
  the current simulator exits on invalid-time errors rather than remaining on
  this screen. No change to that host error behavior is approved here.
- Notices can coexist when applicable. A healthy frame with visible markers has
  no notice ink. Clear resolved conditions on redraw; do not hide unresolved
  warnings after a timeout or require dismissal.

### Boundaries and remaining contract
- Bottom-left placement does not reserve an exclusion zone in the current
  projection. Transparent mock text can collide with markers or their labels;
  the retained synthetic stress example deliberately exposes that problem.
  This direction initially left overlap treatment open. The user subsequently
  approved [0040's small white backing](0040-white-backed-bottom-left-notice-contract.md),
  allowing local visual occlusion while preserving eligibility and selection.
  No displacement, viewport resize or collision search follows.
- Keep the accepted radar, arrows, visible-only selection and marker ID/hysteresis
  rules. Do not silently weaken the requirement to show incomplete tracking
  alongside successful markers.
- The comparison uses crisp existing 6x10 bitmap text and preserves 0038's
  illustrative glyphs. It does not establish glyph mappings, category semantics
  or final strip/font geometry. Empty-state mocks clear the full stack and retain
  the separator as a proposal, not an accepted full-stack lifecycle contract.
- No revised glyph implementation, animation, new selection policy, debug access,
  prediction, fetching, scheduling, production capacity, hardware or frozen-M2
  evidence changes follow from this decision.

## Verification scope
Three headless replays at 20°, 0.1° and 100° supplied healthy radar backgrounds
using the documented local weather/GPS inputs. Twelve native mock panels were
checked as 400x240 strictly 1-bit, with notice changes bounded and no underlying
radar ink erased; the label-updated stack retains equal 10px outer padding.
These are static composites, not executable warning-state snapshots, golden
tests, physical-display validation or performance evidence. Rust check/test
suites were not rerun for this initial docs/images-only decision; 0040 records
separate implementation verification.
