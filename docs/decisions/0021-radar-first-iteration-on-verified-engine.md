# 0021 — Radar-first iteration on the verified engine

Date: 2026-10-11
Status: accepted

## Decision
Use the verified headless engine and existing S3 measurements to begin a minimal
real-data simulator radar before closing the remaining M2 operating-budget and
scheduling work. Prioritize a responsive, visually satisfying radar over pass
prediction; tune catalogue selection, density and the secondary information area
through UI iteration rather than fixing them from benchmark sizes.

This amends the scale expectation in [0003](0003-curated-catalogue.md) and the
remaining implementation sequence in [0005](0005-engine-first-implementation.md),
not the engine-first correctness requirement or physical calculation contracts.

## Why
The engine-first intent is trustworthy ingestion, calculation and information,
so presentation can evolve without changing physical truth. Checked offline
catalogue handling, propagation/geometry, headless prediction and independent
numerical checks now provide that foundation. The accepted
[S3 evidence](../evaluations/s3-m2.md) supplies measured costs, not general
catalogue capacity or whole-device guarantees.

The 400×240, 1-bit display makes legibility and useful density empirical product
questions. Tens of curated satellites may work better than hundreds; neither a
mandatory hundreds-object target nor a 16-object cap follows from the evidence.
Catalogue population and simultaneously visible markers are different counts.
The proposed 4-group/32-record/16-satellite operating package was not accepted;
experimental bounds must not become product limits by default.

Pass prediction remains a useful, tested capability, but its background workload
and display requirements should not dictate radar capacity or layout. Completing
its production scheduling before any visual feedback would settle product tradeoffs
prematurely. Discarding the engine or substituting synthetic physics is unnecessary.

## Consequences
- Begin the bounded M3 simulator slice in [PLAN](../PLAN.md): explicit time and
  observer, configurable local real data, shared state/rendering, projection,
  clipping, zoom, real markers and basic identification. Refine spatial semantics
  before implementation; do not expand immediately into full UI polish or firmware.
- Preserve checked ingestion/provenance, frame/time contracts, explicit failures,
  and independent numerical regression tests. Presentation state, including slowed
  motion, cannot become physical state or the source of true-position readouts.
- Retain the above-horizon eligibility rule. Zooming out reveals additional eligible
  catalogue objects inside the wider view, not below-horizon objects. Full-catalogue
  and Starlink layers remain deferred; curated sources remain the default.
- Keep pass prediction and its tests/tools intact. Integration is secondary, not
  required for the first useful radar. If exposed, preserve physical pass semantics,
  crossing uncertainty and incomplete-result honesty. No zoom-dependent prediction
  policy or screen-entry predictor is selected by this decision.
- A secondary information area remains desired. Its contents and proportions,
  including whether a countdown earns space, are provisional simulator questions;
  the earlier 160×240 column is a starting layout, not a fixed allocation.
- M2 remains open: supported input/catalogue/result limits, calculation/prediction
  cadences, overload behavior and bounded scheduling still require explicit choices
  and verification informed by the visual workload. No new numeric operating
  defaults, scheduler, safe stack size or memory guarantee are accepted here.
- Simulator visual success is not S3 performance evidence. Reuse existing measured
  constraints, then target specific unanswered feasibility questions. Allow
  application/display/network/refresh headroom; hardware access still needs approval.
- Live HTTPS retrieval, caching/refresh, time synchronization and stale-data policy
  remain M6 work. Verified offline ingestion does not mean live retrieval is done.
