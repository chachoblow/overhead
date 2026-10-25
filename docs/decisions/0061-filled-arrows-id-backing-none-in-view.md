# 0061 — Filled arrows, backed selected ID, and `NONE IN VIEW`

Date: 2026-10-24
Status: implemented in the wide view; user chose each change from rendered variants

## Decision

Three normal-view refinements, chosen after reviewing current snapshots and
throwaway variants of the real renderer:

- **Filled arrows:** off-screen edge indicators keep their size, position,
  direction and one-per-satellite rule, but are drawn solid black instead of
  hollow (amends [0033](0033-wide-radar-comparison-contract.md) and DESIGN's
  "hollow" indicator). Overlaps still merge; nothing is suppressed or displaced.
- **Backed selected ID:** the marker-adjacent ID gets a borderless 1px white
  backing around its text bounds, so coastline, limb or arrows cannot cross it
  (amends [0034](0034-selected-marker-identification.md)'s transparent text).
  Placement, fallback, clamping and [0035](0035-label-zoom-visibility.md)'s zoom
  gate are unchanged. The backing covers only the text box: the selected diamond
  can still be crossed by scene ink. No collision search or leader line.
- **`NONE IN VIEW`** replaces `SKY OUTSIDE VIEW` for zero in-view markers with at
  least one off-screen arrow (amends [0048](0048-refined-empty-view-notices.md)'s
  wording only; its conditions, priority and backing are unchanged). The user
  found the old phrasing unclear. It still makes no promise that zooming reveals
  every object.

## Consequences

Selection, eligibility, counts, physical readouts, square/circular diagnostics,
debug and other notices are unchanged. Host tests pin the filled interior, the
label's exact change region and the new string. Earlier mocks and decision text
that show hollow arrows, transparent IDs or `SKY OUTSIDE VIEW` remain historical
references.

## Follow-up — 2026-10-24: backed selected diamond

At the user's request, the wide view's selected diamond also gets a 1px
diamond-shaped white backing (all pixels within Manhattan distance 4 of its
centre are cleared, clipped to the radar), and it is drawn after every other
marker so the backing and diamond sit on top. Coastline, limb, arrows, the aim
cross or a coincident marker no longer cross it. Unselected markers keep their
unbacked 5px `+` (the user reviewed square, diamond and halo alternatives and
kept the current markers). Square/circular diagnostics are unchanged. Other
selected-marker behavior (0035, 0040, 0049) is unchanged.
