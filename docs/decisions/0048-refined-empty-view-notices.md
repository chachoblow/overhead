# 0048 — Refined empty-view notices for the wide radar

Date: 2026-10-18
Status: accepted (M4 contract; implementation pending separate authorization)

## Decision

Refine the **opt-in wide radar's** empty-view status using the current app frame.
This amends [0039's `NO MARKERS` condition](0039-bottom-left-condition-driven-notices.md)
and [0040's notice backing width](0040-white-backed-bottom-left-notice-contract.md),
not their warning lifecycle, placement or compositing order. The user approved
both the wording and the wider backing after reviewing the
[native-size comparisons and their provenance](../mockups/0048-empty-view-proposal.md).
Approval of this contract does **not** authorize renderer implementation or
constitute executable visual acceptance.

## Why

The accepted wide radar uses `NO MARKERS` for every valid frame without in-view
markers. That is honest but conflates arrows showing tracked objects just
outside the viewport with a clean catalogue with no tracked object above
the observer horizon. Distinguish those states without treating missing
input, failed samples or invalid camera depth as complete empty sky. Reuse the
accepted bottom-left overlay and its independent warning lines rather than
adding a new page, pass prediction, zoom advice or selection policy.

## Consequences

### Status rules

Evaluate the first matching row for the bottom status line. `visible` and
`above_horizon` count successful current samples; an indicator means an
above-horizon object with valid forward projection outside the viewport.
All statements concern only the **loaded, accepted catalogue at the current
UTC**, not the complete sky, missing input, future passes or freshness.

| State | Bottom status |
| --- | --- |
| No valid-time frame | `NO FRAME` |
| At least one visible marker | none |
| Zero visible, at least one off-screen indicator | `SKY OUTSIDE VIEW` |
| Zero visible/indicators, accepted catalogue nonempty, zero above-horizon objects, zero tracking failures, no ingestion warning | `NONE ABOVE HORIZON` |
| All remaining zero-visible cases | `NO MARKERS` |

`SKY OUTSIDE VIEW` identifies at least one successfully tracked object outside
the viewport; other above-horizon objects can have invalid camera depth and
no arrow. It does not promise that zoom can reveal every object. In particular,
above-horizon objects with no valid arrow fall back to `NO MARKERS`, not zoom
advice. `NONE ABOVE HORIZON` means only that no **accepted, successfully tracked**
object is above the observer's horizon; require clean ingestion and no tracking
failures to avoid implying completeness from a known-incomplete frame. An empty
catalogue falls back to `NO MARKERS`, rather than claiming horizon knowledge.

Keep `TRACK ERROR` for any current per-object tracking failure (including when
other objects are visible or indicated) and `INGEST WARN` for the current load's
omissions. Append these lines in [0039's established order](0039-bottom-left-condition-driven-notices.md),
independently of the bottom status line. `NO FRAME` replaces any empty subtype
when no valid frame exists. Every line clears as its underlying condition
resolves; no timeout or dismissal. Current simulator fatal load/clock handling
still exits rather than displaying a persistent `NO FRAME` screen.

### Geometry and boundaries

Keep 0040's 6×10 bitmap, bottom-left `x=12` text, final baseline `y=222`,
13px line spacing, 2px padded borderless white backing, and draw-last local
occlusion. Width follows the longest active line; the new 18-cell `NONE ABOVE
HORIZON` allows a **112px** backing (`x=10..121`) instead of the previous 70px
maximum. Three lines still occupy at most 40px height (`y=187..226`). The
backing may cover scene ink but never changes eligibility, arrows, counts,
physical samples or selection. Remove old masks completely on redraw.

Square diagnostics, debug access, markers, arrows/overlap policy, visible-only
selection, five fields, transitions, camera and physical pass semantics are
unchanged. This contract does not select a whole-sky inventory, prediction or
countdown policy, production catalogue/cadence, scheduler, hardware behavior
or frozen-M2 evidence update. The historical screenshots used for review
are **illustrative composites**, not simulator output of the new wording.

### Verification for a separately authorized implementation

- Headless real-data cases: the 16-object default demo at 20° has four arrows,
  zero visible markers and the off-screen status; the one-object ISS comparison
  has no object above the horizon and the horizon status. When a marker enters
  view, remove the bottom status immediately. A no-arrow above-horizon sample
  clipped by depth must not become a zoom suggestion.
- Synthetic controls: failed samples with/without valid arrows, ingestion
  omissions with/without valid arrows, zero accepted entries, missing frame,
  mixed visible-and-failed frames and recovery. Warnings coexist with off-screen
  status where applicable; no known-incomplete frame uses the horizon status.
- Exact bitmap/backing, three-line bounds, reused-screen clearing and pixel
  invariants outside the union of old/new notice masks. Preserve square output,
  app truth, selection, strip, debug behavior and warning-state regressions.
- Run workspace check/tests and app/render no-default-feature checks. Review
  fresh **executable** headless output separately from the contract composites.
  Host snapshots do not establish physical-display legibility or target costs.

[Simulator usage](../../sim/README.md#notice-overlay-review) remains the
executable reproduction reference; until implementation it still documents
`NO MARKERS` for both empty situations.
