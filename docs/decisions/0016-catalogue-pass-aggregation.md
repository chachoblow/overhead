# 0016 — Catalogue pass aggregation and uncertain ordering

Date: 2026-10-08
Status: accepted

## Decision
Compose `search_satellite` across the accepted catalogue and retain every
satellite's report and pass records. Report earliest detected arrival candidates
without converting uncertain crossing brackets into exact times.

- Search synchronously in ascending NORAD ID order, sharing the total evaluation
  budget and applying the explicit per-satellite allowance. Continue after local
  failures/limits; retain unsearched entries after shared-budget exhaustion.
  This is reproducible traversal, not a scheduling or fairness policy.
- Aggregate completion requires every accepted satellite's search to complete.
  Rejected/conflicting ingestion records are outside that catalogue; preserve
  ingestion diagnostics and state this scope in the headless report.
- Among observed upward crossings, find the smallest bracket upper endpoint.
  Every arrival with a lower endpoint at or before that time remains a candidate
  for earliest detected arrival. Include coarse brackets retained on interruption.
  Touching brackets remain ambiguous; candidate display order is not time order.
- One candidate resolves ordering among detected arrivals; multiple candidates
  mean unresolved ordering, not simultaneous arrivals. Empty candidates mean no
  detected arrival, not a complete no-pass result unless all work completed.
  In-progress and equality-boundary starts remain separate from arrivals.
- Incomplete searches cannot establish a catalogue-wide next arrival. Complete
  searches retain the detection and model-accuracy limits of 0012–0015.

## Why
A midpoint or NORAD tie-break would invent timing precision. Transitive overlap
groups can also include an arrival definitely later than another detected event:
a long uncertain bracket must not pull in every event it overlaps. Comparing
against the minimum upper endpoint retains exactly the interval-based candidates
without further orbital evaluations or a new refinement policy.

## Consequences
The aggregation module uses alloc behind the existing `catalogue` feature;
the single-satellite kernel remains allocation-free. Stored records are not an
embedded capacity guarantee. No detection default, numeric allowance, catalogue
size, cadence, freshness cutoff, or background scheduler is selected.

API/tests live in [core/src/catalogue_passes.rs](../../core/src/catalogue_passes.rs)
and [core/tests/catalogue_passes.rs](../../core/tests/catalogue_passes.rs).
The offline CLI reuses catalogue loading/provenance and preserves all pass start,
end, progress, and stop states; usage and exit codes belong in
[tools/README.md](../../tools/README.md). Interval evaluation and measured
operating budgets remain M2 work in [PLAN](../PLAN.md).
