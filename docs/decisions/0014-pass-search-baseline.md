# 0014 — Bounded pass-search baseline

Date: 2026-10-08
Status: accepted

## Decision
Use fixed-step elevation sampling with bisection of detected threshold transitions
as the first headless pass-search implementation. Preserve the physical-pass
semantics and detection limits in [0012](0012-physical-pass-semantics.md) and
[0013](0013-pass-detection-resolution.md).

### Search
- Sample at the window start, regularly spaced times, and the exact window end.
- Refine detected transitions by halving their time brackets until they meet the
  configured crossing tolerance. Refinement does not move the regular sampling grid.
- Retain crossing brackets rather than implying exact times.
- Require an explicit detection interval initially. No detection default or
  numeric work budget is settled.
- Short excursions or gaps can remain undetected, including additional crossings
  inside a refinement bracket. Refinement does not prove event completeness or
  unique crossings.
- Retain detected short passes; do not impose a minimum duration.

### Boundaries
- Strictly above threshold at search start means a pass is in progress; its start
  is unknown.
- Equality at search start followed by above-threshold samples identifies a
  boundary-start interval, not an observed arrival.
- Below → equality → above establishes an upward transition; the reverse
  establishes a downward transition.
- Touching the threshold from below without going above is not a pass.
- Touching from above without a detected below-threshold gap does not split the
  predicted pass.
- Equality at the window end alone does not establish a crossing. Preserve
  boundary uncertainty.

### Partial results and work limits
- Distinguish upcoming passes, passes underway, completed searches without
  upcoming arrivals, and incomplete searches.
- Distinguish a pass whose end lies beyond a successfully searched window from
  one whose end could not be calculated.
- Count every attempted elevation evaluation, including refinement and failed
  evaluations.
- Enforce explicit work limits. Preserve established results when a satellite
  fails or exhausts its allowance; continue other satellites while the overall
  allowance permits.
- Identify satellites left unsearched or partially searched. Incomplete results
  cannot establish a catalogue-wide earliest pass or a complete no-pass result.

## Why
This baseline is understandable, reproducible, allocation-free in principle, and
measurable. Adaptive searching is deferred until measurements justify its
additional complexity.

## Consequences
Prediction remains independent of radar visibility, zoom, selection, and visually
slowed motion. Exact Rust types, validation details, tests, numeric limits,
catalogue scheduling, and display wording remain follow-up work. Work limits
alone do not make prediction a nonblocking background task.
