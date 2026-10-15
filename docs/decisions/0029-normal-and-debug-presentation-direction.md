# 0029 — Normal and debug presentation direction

Date: 2026-10-12
Status: accepted (normal readout/layout direction amended by [0032](0032-wide-radar-and-selected-satellite-glyphs.md), including bare NORAD ID as the normal marker label; separation remains unimplemented)

## Decision
Separate the normal user-facing presentation from an objective diagnostic view.
Remove branding and routine debugging fields from the normal information area;
this accepts a direction and field reduction, not a panel layout or implementation.

## Why
The user wants a blend of sky overview and selected-satellite information that
feels alive and active, but not busy. The current panel mixes those purposes with
replay and development diagnostics. After reviewing graphical mockups and sharing
Are.na inspiration screenshots, the user deferred further visual design to another
session and explicitly retained the normal/debug distinction and field cutting
as useful outcomes. Neither mockup was accepted as the implementation target.

## Consequences
- Remove the project name from the normal view. Replay labels, simulated UTC/date,
  catalogue/error counts, NORAD ID and the source/control-hint footer belong outside
  the normal view; retain useful diagnostics in a separate objective presentation.
- Retain view/sky information with a different presentation, alongside selected-
  satellite information. Typography, hierarchy, graphics and exact remaining
  fields are not fixed. Range is questioned, not definitively removed.
- A simulator launch option and/or `v` toggle are candidates, not settled controls.
  This does not introduce user-facing menus/pages or a device debug-control policy.
  The existing simulator remains historical replay, not live data acquisition.
- Removing error counters is not a decision to conceal unavailable/incomplete
  tracking. Normal-view failure communication still needs explicit design.
- Pass-profile interest is not approval to integrate prediction. The mocked hill
  and current-time caret were illustrative, not calculated. Actual rise/peak/set
  context requires prediction integration; epoch offset is not a freshness score.
- Defer further visual iteration. Scope the next M3 slice before implementation;
  no normal/debug split, toggle or proposed panel graphics are implemented yet.
