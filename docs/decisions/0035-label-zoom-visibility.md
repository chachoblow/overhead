# 0035 — Label zoom visibility

Date: 2026-10-16
Status: accepted (bounded M3 refinement; implemented and user-reviewed, with configurable thresholds)

## Decision
Amend [0034's selected-marker identification](0034-selected-marker-identification.md)
with clean wide-view label hide/show at configurable vertical-FOV thresholds.
Default: hide at or above 70° while zooming out; restore at or below 60° while
zooming in. Preserve the current visibility between them. The selected diamond
remains visible whenever the selected object is an in-view marker.

## Why
The user found the labelled 100° weather/GPS view crowded. Static spatial-dither
mockups were explored, but not implemented; the user ultimately chose simple
hide/show with hysteresis rather than a fade/dissolve. Intermediate real-data
snapshots showed space around the label at 60°, with little separation from
Earth by 70°. The user approved 70°/60° as provisional defaults and asked to keep
them configurable. These snapshots motivate a first comparison, not a universal
collision threshold or production device policy. In the following visual-design
session (recorded 2026-10-17), the user confirmed having reviewed the existing
70°/60° behavior and accepted it for now; no threshold or code change followed.

## Consequences
- Hide/show only the selected ID/ISS text, only in the opt-in wide presentation.
  Keep the selected diamond, all other markers/arrows, Earth, warnings, physical
  samples, counts and selection rules unchanged. Baseline diagnostics and other
  viewport comparisons are unaffected.
- Hysteresis belongs to clockless app presentation state, not the simulator or
  renderer. Update it on zoom, independently of whether a marker is currently
  selected. Frame updates, selection changes and absent/invalid frames do not
  reset zoom history. The renderer consumes that state without mutating it.
- At initialization (including snapshots), there is no history: show below the
  hide threshold, hide at/above it. The dead band therefore starts visible.
  Reconfiguring the policy explicitly resets that history at the current lens.
- Expose simulator tuning as `--label-hide-deg` and `--label-show-deg`, requiring
  `--wide`. Either may be overridden, with defaults for omitted values. Require
  finite `0.1 <= show < hide <= 100` degrees; reject invalid/repeated inputs.
  These are launch-time comparison knobs, not new device controls.
- No dithering, opacity, animation, timer, density heuristic, collision search or
  selection ranking. When a label is shown, 0034's placement and transparent
  overlaps still apply. This does not guarantee absence of crowding at other
  times, sites or selected objects.
- Verify inclusive boundaries, history in both directions, initialization,
  invalid configuration, frame/selection independence, unchanged physical state,
  retained diamonds/warnings, no stale label ink, baseline isolation and
  deterministic headless CLI overrides. Keep new visual review separate from
  physical-display or hardware-performance claims.

Usage and reproducible comparisons live in the
[simulator README](../../sim/README.md#label-zoom-visibility).
