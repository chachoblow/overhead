# 0013 — Pass detection resolution

Date: 2026-10-08
Status: accepted

## Decision
Clarify [0012](0012-physical-pass-semantics.md): prediction uses a bounded
numerical search with explicit detection resolution, separate from
threshold-crossing time tolerance. Physical passes remain continuous intervals
above the configured elevation threshold; detecting every arbitrarily brief
interval is not required.

- Very short above-threshold excursions or below-threshold gaps may go
  undetected. A missed gap can make separate passes appear continuous.
- Do not deliberately discard short passes that are detected. This is a search
  accuracy limit, not a minimum-duration eligibility filter.
- A complete search means the configured procedure finished over the requested
  window, not proof that no shorter event exists. No-pass and earliest-arrival
  claims are subject to the declared detection limits.
- Propagation failures and exhausted work budgets still make results incomplete;
  retain usable results and failure details rather than reporting no pass.
- Radar eligibility remains based on current geometric elevation above the
  horizon, independent of whether a pass was predicted. Keep the intent to make
  fast screen transits watchable without altering true-position readouts.

## Why
Time above the pass threshold, time above the horizon, and time inside a zoomed
view are different. Accepting missed brief threshold grazes does not exclude
fast-moving satellites from the radar or reverse the visual-slowing intent.
A practical, measurable search is sufficient; an explicit duration filter would
add a product rule and require duration classification without a demonstrated
need.

## Consequences
- Search strategy, shared-core API, boundary/tangency handling, tests, detection
  resolution, and work limits remain open M2 design work in [PLAN](../PLAN.md).
- Choose resolution and budgets through representative tests and measurements.
  The discussed 10-second detection limit is not an accepted default; neither
  fixed-step sampling nor a particular sample interval is settled.
- The existing 0012 defaults remain unchanged. Crossing refinement alone does
  not establish detection coverage; document and test those limits separately.
- No UI implementation, prediction-based marker filter, or change to the
  physical pass definition is introduced.
