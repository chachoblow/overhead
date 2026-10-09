# 0012 — Physical pass semantics

Date: 2026-10-08
Status: accepted (detection/completeness clarified by [0013](0013-pass-detection-resolution.md))

## Decision
A physical pass is a continuous interval when a satellite is above a configured
minimum elevation from the configured observer. Predict physical events
independently of radar projection, zoom, clipping, and visually slowed motion.

| Tuning input | Initial default |
|---|---|
| Minimum elevation | 10° |
| Look-ahead from search start | 24 hours |
| Threshold-crossing time tolerance | 5 seconds |

All three are configurable tuning inputs, not necessarily device settings.
The tolerance is relative to the crossing predicted by the orbital model, not
a guarantee of real-world timing accuracy. Defaults remain subject to measured
search cost on the target hardware. Detection resolution is separate; complete
search and earliest-arrival claims are subject to the numerical detection limits
clarified in [0013](0013-pass-detection-resolution.md).

### Events and results
- A pass starts at an upward threshold crossing and ends at a downward crossing.
  Peak time/elevation prediction is deferred from the first slice.
- “Next pass” means the earliest upcoming upward crossing across the catalogue,
  not the nearest satellite in space or the next marker to enter the screen.
- A satellite already above threshold at search start is a pass in progress.
  Do not invent its start time or ignore it; predict its end where possible.
  In-progress passes and upcoming arrivals are distinct results.
- If no upcoming crossing is found in a complete search, report none within
  the look-ahead window, not that no pass will ever occur. This can coexist
  with a pass in progress.
- If a pass has not ended by the search limit, its end remains unknown beyond
  the window. A satellite above threshold throughout the window is not an
  upcoming arrival; do not invent an end or claim it will remain above forever.
- If prediction fails for a satellite, retain usable results from others and
  mark the aggregate search incomplete. Its earliest result is only earliest
  among successfully searched satellites, not guaranteed catalogue-wide.
  Never label an incomplete search with no results as a complete no-pass result.
  Preserve failure details for the headless tool; compact UI treatment is later.

### Radar eligibility
Markers are eligible only above the observer's geometric horizon (0°), then
subject to projection and screen clipping. This remains true at whole-Earth
zoom: the view represents the observer's sky, not all satellites worldwide.
A marker can therefore appear below the pass threshold before its announced
pass begins. Horizon eligibility and the configurable pass threshold are
separate checks; zoom changes neither.

Use the existing geometric elevation convention in
[0008](0008-observer-geometry-conventions.md). These checks do not promise
naked-eye visibility or account for terrain, refraction, lighting, or weather.

## Why
A catalogue-wide forward search gives useful advance notice during empty sky;
a nearby-position filter would not. Physical events keep the countdown stable
under zoom. A separate horizon cutoff preserves the local-sky framing while
allowing a higher threshold for meaningful arrivals. Bounded searches and
explicit unknown/incomplete results avoid false promises and unbounded work.

## Consequences
- Product semantics are settled; prediction is not implemented. Search strategy,
  API, tests, and operating budgets are the next M2 work in [PLAN](../PLAN.md).
- Technical design must address short-pass detection as well as crossing
  refinement; a 5-second refinement target alone does not prevent missed passes.
- Exact threshold/window boundary rules, tangencies, configuration validation,
  search limits, and over-budget behavior still require technical design.
- Radar projection, selection policy, display wording/layout, and scheduling
  remain future work. This decision does not bring UI implementation into M2.
