# M4 coastline — fixed-observer preparation/reuse proposal

Status: **implemented as a host/simulator slice in [0058](../decisions/0058-prepared-wide-coastline-slice.md); observer-space geometry accepted as the first host feasibility path;
remaining lifecycle/resource choices are proposals, not an implementation
contract or device adoption**.
[0057](../decisions/0057-fixed-observer-coastline-preparation-scope.md) accepts a
fixed observer during use, with re-preparation when location changes. The user
subsequently confirmed that a **distinct, visible one-time preparation phase**
after setting/changing location is acceptable, chose **observer-space geometry
over per-zoom bitmaps for the first host feasibility path**, and agreed that the
old coastline should disappear until the new site's geography is ready. The
user selected a **full loading screen** for this one-time location preparation,
not a coast-free radar with a preparing notice. None of this authorizes a cache
implementation or target experiment. Setup duration, loading-screen wording and
layout, location-setting mechanism and memory allocation are **not** approved.
The first firmware prototype still has [0004's compile-time location](../decisions/0004-hardcoded-location.md);
configurable distribution needs a successor decision.

## Why this is a different cost model

The [one S3 representation check](m4-coastline-representation-target.md) drew the
visually accepted quadratic candidate from its 32,211 B Earth-fixed asset on
every call: 9,961–22,489 µs coast-only across eleven scenes, over the provisional
5 ms complete-rebuild screen. Its candidate-specific nested stack reservations
also exceed 1 KiB. Neither number measures a prepared observer layer. The
original production comparison still draws the older asset each wide frame;
the candidate is reachable only in tests or the explicit measurement feature.
Stop global per-frame renderer tuning under [0056](../decisions/0056-bounded-coastline-architecture-trial.md),
not silently extend that experiment into an approved cache.

For a fixed observer and camera, camera coordinates and front-facing geography
need not change as satellites move. Lens FOV still changes screen scale, clipping,
refinement and rasterization. The existing candidate's `Context::new` depends on
both site and `View`; `flatten` uses lens scale and viewport pixels. Moving
`candidate_draw` wholesale into a one-time setup would therefore **not** support
zoom reuse. App state already holds an observer/camera and changes FOV without
changing location; there is no runtime location-change API today. Preparation
should be a separately owned presentation resource, not a modification to
physical propagation or a claim that the existing app supports changing sites.

## Proposed lifecycle to review

1. **Location accepted:** validate the observer before building anything. Treat
   any changed configured observer as a new preparation generation; do not
   silently retain old coast for a nearby site. A distance/rounding equivalence
   threshold is unselected. Define the supported site/altitude domain before
   making a shipping guarantee.
2. **Preparing:** show a full loading screen, not a coast-free radar. Do not
   display geography prepared for a different observer. Preparation is distinct
   from an invalid coastline error and must not present `COAST ERROR` as normal
   progress. Exact screen wording, whether tracking continues behind it, and
   whether preparation is blocking or incremental remain open.
3. **Ready:** publish a complete, validated observer-keyed resource atomically.
   Reuse it across satellite updates and lens FOV changes; reproject/rasterize
   per FOV as necessary. Preserve the foreground order, square/circular baseline,
   and the exceptional-only `COAST ERROR` behavior in
   [0054](../decisions/0054-first-executable-coastline-comparison.md).
4. **Change/failure:** invalidate or version the old resource before displaying
   a new site's geography. A preparation failure must not publish a partial or
   stale layer. The exact visible not-ready/failure treatment remains a review
   decision, not permission to use `COAST ERROR` for ordinary setup or budget
   exhaustion. On each frame, an unexpected coast-only calculation failure still
   requires whole-coast rollback; draw-target errors still fail the frame.
5. **Restart:** persistence is optional, not required for the first experiment.
   If proposed later, key stored data by location, asset/algorithm version and
   validation metadata; a cold boot cannot assume a cached site is current.
   Flash endurance and update/atomicity would need separate review.

## First feasibility path and alternatives

| Option | Zoom reuse | Cost that needs evidence |
| --- | --- | --- |
| Observer-space curves (**accepted first host feasibility path, not a retained format contract**) | Keep camera-space, front-facing runs independent of lens; clip/refine/raster at each FOV | Retained coordinates, run breaks and tangent/near-limb handling; per-zoom raster cost and live scratch. A naive three-control `f32` camera curve for all 2,031 pieces alone takes **73,116 B** (2,031 × 3 × 3 × 4), before visibility metadata or the 32,211 B source asset. This is an upper-shape illustration, **not** an allocated buffer, lower bound or approved RAM allowance. Compact/streamed variants would need their own evidence. |
| Retained 1-bit geography mask | Cheap reuse only at its recorded FOV/site | One 352×240 radar mask is **10,560 B** at minimum; a full 400×240 mask is **12,000 B**. A single pixel mask cannot be rescaled across 0.1°–100° without appearance/edge errors. Multiple masks, regeneration on zoom, staging and atomic rollback cost more memory/time. No persistent bitmap was allowed by default in the [provisional gates](m4-coastline-budget-preflight.md). |
| Retain no observer-specific result | No preparation memory | Continues the measured rebuild/scratch failures; not the 0057 reuse direction. |

The existing packed asset lives in flash in the measurement image; it is **not**
observer-specific RAM. Host fitting/audit in
`render/src/coast/representation_spike/packing.rs` is offline, allocates freely,
and must not be mistaken for a measured on-device preparation path. Neither
proposed retention scheme has passed the provisional **zero incremental heap**,
**1 KiB live scratch** or **5 ms per-frame** screens. Persistent bytes are a
separate line item, not scratch or proved free internal RAM/PSRAM. If a design
needs a different memory or timing policy, request explicit revision before
adoption rather than hiding setup work or relaxing [0053's ordinary-gap rule](../decisions/0053-no-resolvable-coastline-omissions.md).

## Evidence gate beyond the first host trial

A [first host-only prepare/draw trial](m4-coastline-observer-preparation-host.md)
now measures an uncompressed observer-space baseline against the checked asset
and candidate. It does **not** establish a device resource policy or close this
contract. Further bounded experiments should record, on the existing eleven
scenes plus zoom sweeps at the *same* site:
preparation time and peak temporary/retained bytes; cold start and changed-site
behavior; per-frame coast time, heap and stack inventory; and masks for tight,
limb, viewport, polar and dateline views. Require complete scene output and
isolated-segment comparisons, foreground/notice and forced-late-failure rollback
controls, plus tests that satellite updates do not rebuild and FOV changes do not
use a stale raster. Record invalid site, preparation failure and old/new-location
isolation. Keep [0055's deferred all-view proof](../decisions/0055-defer-all-view-coastline-proof-for-m4.md)
deferred; a reproduced ordinary visible gap is still a bug.

If host results justify a device check, **seek separate approval** for any target
build, flash or board access. Time preparation (including validation) separately
from the steady-state coast draw; report maximum observed invocation and scene
coverage, not a guaranteed worst-case latency. Inventory flash, internal RAM,
optional PSRAM, retained state, temporary peak, heap, nested stack frames and
full-frame buffers independently. Compare with the old and candidate evidence
without treating the isolated benchmark heap or stack watermark as spare capacity.
Sharp transfer, selected non-coast frame cost, scheduling and whole-device 20 Hz
remain independent blockers. Do not alter the frozen M2 evidence.

## Decisions required before that experiment

The first host path is observer-space geometry, not a specific format, RAM budget
or permission to implement it. The user also selected a full loading screen
until the new site's geography is ready. The following still require review:

1. Loading-screen wording/layout, progress indication and whether tracking
   continues behind it; preparation must not masquerade as `COAST ERROR`.
2. What measured setup delay and retained-memory ceiling would be acceptable
   for review? Recommendation: set these after a bounded host memory/time
   estimate, before target/device adoption; visible preparation is **not** an
   unbounded-delay authorization. Persistence and the mechanism for setting a
   location can be deferred beyond this host seam.
