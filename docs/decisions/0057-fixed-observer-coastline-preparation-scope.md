# 0057 — Fixed observer for this coastline iteration; keep globe navigation open

Date: 2026-10-24
Status: accepted scope direction; preparation contract and implementation open

## Decision

For this device iteration, geography may be prepared for a **fixed observer location during use**, then reused while satellites move and the lens zoom changes. For distribution to different users, a coastline-enabled device should support setting a location and preparing again; location-specific firmware builds are not the intended deployment solution. Scrolling/panning around the globe remains a possible **future iteration**, not a requirement of this coastline slice.

## Why

The user wants to preserve the accepted coastline appearance and the measured quadratic-representation work, but does not need a continuously moving observer in this version. The [single S3 check](../evaluations/m4-coastline-representation-target.md) improved rebuild time ~435× yet missed the provisional 5 ms and 1 KiB screens. A stationary observer offers reuse across frames and lens zoom without another campaign of global-renderer tuning. Future globe navigation should not be presented as supported by an observer-specific cache without additional work.

## Consequences

- Scope **set location → prepare geography → reuse** next; distinguish one-time setup cost from per-frame work, retain lens zoom and invalidate/reprepare on location changes. Investigate on-device preparation first, not per-user compiled geography. Do not promise that preparation meets frame timing or stack/memory screens until measured.
- No mechanism for setting the location, setup delay, retained geometry/bitmaps, memory limits, persistence, location-change threshold, panning semantics or production renderer was selected or authorized for implementation. Preserve [0053's ordinary-gap rule](0053-no-resolvable-coastline-omissions.md), [0055's deferred universal proof](0055-defer-all-view-coastline-proof-for-m4.md) and the exceptional-only `COAST ERROR` treatment.
- [0004](0004-hardcoded-location.md)'s compile-time constant remains the **first firmware prototype** contract; this direction does not silently change that prototype or decide its successor's configuration mechanism. Before distributing configurable devices, record how the observer is set and persisted. Designing later globe scrolling/panning and invalidation for a continuously moving observer is separate work, not blocked by accepting a fixed observer now.
- The [0056](0056-bounded-coastline-architecture-trial.md) one-host/one-target allowance is consumed. The visually accepted representation, host checks and exact S3 evidence remain preserved as references, not an adopted shipping renderer. Non-coast frame cost, Sharp and whole-device cadence remain separate blockers.
