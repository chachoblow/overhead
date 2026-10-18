# 0049 — Stable selection across the wide viewport

Date: 2026-10-18
Status: accepted M4 behavior contract; implemented after separate authorization and user visually accepted

## Decision

In the opt-in wide radar, keep the selected NORAD ID while its current sample is
successful and its true observer elevation remains above zero, even if zoom or
orbital motion moves its marker outside the viewport. A manually Tab-selected
object receives the same continuity: automatic selection must not override it
while it remains eligible. This amends
[0033's visible-only selection](0033-wide-radar-comparison-contract.md) and
selected-data lifecycle in [0041](0041-circular-elevation-only-contract.md),
[0045](0045-static-group-category-contract.md),
[0046](0046-selected-data-glyph-transition-contract.md) and
[0047](0047-text-first-debug-access-contract.md). Those decisions describe the
historical visible-only behavior; 0049 now amends the implemented wide view.
Physical marker/arrow rules do not change.

## Why

Losing selection when an object merely crosses the lens boundary makes Tab
inspection and its true-state strip jump to an unrelated in-view object. Zoom
and orbital motion should not differ at that boundary; unlike a viewport exit,
falling below the observer horizon or failing to track removes the current
eligible sample. Retain identity, not an old reading. Keep the existing first
in-view fallback rather than introducing an unreviewed ranking policy.

## Consequences — bounded behavior

- On a valid frame with no retained selection, acquire the first **in-view**
  successful marker in ascending NORAD ID order, as today. Never acquire an
  off-screen object automatically. While a selection remains successfully tracked
  above the horizon, retain it regardless of viewport inclusion; do not switch
  just because another marker enters the view or the lens changes. This holds
  for both manually and automatically acquired selections; no separate manual
  lock, timer, ranking or sticky historical sample is needed.
- If the selected object's current tracking fails, its elevation reaches or
  crosses the horizon, its ID is absent from the current accepted catalogue, or
  there is no valid frame, end that selection. In a valid frame, immediately
  choose the first in-view successful marker if any; otherwise clear selection.
  No failed/below-horizon object's old data may survive. Frame updates and zoom
  use the same eligibility rule; valid forward projection is required for an
  in-view marker or arrow, **not** to retain a successful above-horizon selection.
- Tab cycles through **in-view** successful markers in ascending ID order,
  choosing the next higher ID than the current selection or wrapping to the
  first. Thus Tab from an off-screen retained ID selects an in-view marker;
  when none is in view, Tab leaves the retained selection unchanged. Tab does not
  select arrows or below-horizon objects. Zooming back to a selected object's
  position reveals its selected diamond again unless Tab or genuine loss
  changed selection meanwhile.
- A retained off-screen selection has no marker diamond or marker-adjacent ID
  label in the radar. Its current successful physical sample and element/group
  metadata still supply the five strip fields and debug selected-object values.
  The existing field-local invalid suppression and 0046 true-state/presentation
  separation remain; no freezing or interpolation of physical readings. The
  off-screen arrow, if one exists, is not itself a source of selected data and
  gains no highlight or ID. An above-horizon object clipped by camera depth may
  have neither marker nor arrow yet remain selected with true-state fields.
- [0048's empty-view status](0048-refined-empty-view-notices.md) continues to
  describe the viewport and current catalogue, independently of selection.
  `SKY OUTSIDE VIEW` can coexist with a selected object's strip; `NO MARKERS`
  remains the conservative fallback if no arrow qualifies. Independent tracking
  and ingestion warnings persist. Label-zoom hysteresis still gates only an
  in-view selected marker's text, not the strip or selection.
- A changed selected ID follows [0046's transition](0046-selected-data-glyph-transition-contract.md)
  when both old and new successful selections have data; continued off-screen
  selection is the **same ID**, not selection loss or a new transition trigger.
  Clearing on genuine loss without a replacement and first acquisition from none
  remain immediate. Debug toggling still discards hidden presentation history.

This is an explicit amendment to the older "no in-view markers means no selected
fields" and "off-screen arrows never supply selected data" wording in 0041–0045:
the successfully tracked selected **object** supplies fields, not its arrow.
The wide debug page must likewise distinguish an off-screen selected object
from no selection rather than reporting "no selected visible object". Historical
mockups and older test descriptions remain historical; [simulator usage](../../sim/README.md#wide-radar-comparison)
now describes the executable.

## Boundary and next gate

Only the opt-in wide comparison changes. Keep square/circular diagnostic
selection visible-only, the 352×240 geometry, counts, physical horizon and pass
semantics, arrow multiplicity, status wording, strip raster, and category/field
mappings unchanged. No selected-arrow styling, panning, auto-follow, new ranking,
countdown, prediction, slowdown, production catalogue/cadence or device control.
The user separately authorized implementation after this behavior was recorded.
After implementation, the user confirmed reviewing the interactive wide replay
as well as the code/contract and accepted the executable behavior. This visual
acceptance is distinct from the host checks below.

Implementation updates `app/src/lib.rs` reconciliation/Tab for wide views only;
existing renderer selection lookup already supplies current off-screen strip and
debug values without drawing an absent marker/ID. Debug's no-selection wording
no longer implies visibility. App, renderer and simulator tests exercise zoom and
replay continuity, manual Tab/fallback, physical loss and recovery, invalid
depth, strip/debug values, transition behavior and square isolation. A standalone
snapshot has no prior selection and cannot demonstrate continuity; the
[interactive review recipe](../../sim/README.md#wide-radar-comparison) does.
Serial workspace check/tests, app/render no-default-feature checks, formatting
check, strict all-target app/render/sim Clippy, local-link and whitespace checks
pass. Tests cover synthetic zoom/motion, Tab/fallback, horizon/failure/invalid
frame, invalid depth, current off-screen strip and debug values, transition
continuity, unchanged square selection and independent real-data snapshot
regressions. The user separately accepted the interactive executable behavior;
neither host checks nor that review establish physical-display validation or
target performance. No hardware or frozen-M2 evidence was changed.
