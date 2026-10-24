# 0056 — One bounded coastline architecture trial, then choose

Date: 2026-10-24
Status: host trial and single authorized target check complete; appearance accepted; target screens not met; scope location-preparation fallback, no production replacement

## Decision

Try **one host-only representation experiment** combining precomputed Earth-fixed
geometry and screen-scale-dependent refinement, with conservative visibility and
clipping. Stop further culling-only tuning of the current renderer. If the result
is convincing in shape, storage and remaining work, consider **at most one separately
authorized target feasibility check** before choosing the architecture. Host speedup
alone is not a target forecast or permission to extend the investigation indefinitely.

If the trial is unconvincing, pivot to scoping **set location → prepare geography →
reuse**, rather than another sequence of general-renderer optimizations. Different
users must be able to use different locations; per-person firmware builds are not
the intended solution. On-device observer-specific preparation is the preferred
fallback to investigate, not an implemented or fully specified configuration,
persistence, memory or scheduling contract.

## Why

The accepted appearance, checked export and regression suite are useful references
for either architecture. The current global resampling path takes seconds on the S3;
[the rejection/reuse spike](../evaluations/m4-coastline-candidate-spike.md) did not
provide a credible stand-alone remedy. The user is willing to try a bounded
optimization before pivoting, but does not want this feature to delay the device
through another open-ended research sequence.

A fixed observer/camera makes coastline visibility and pre-lens projection independent
of satellite motion and FOV. Observer-specific projected curves could support lens
zoom without rebuilding geographic geometry. Setup latency, retained storage and
location changes would need explicit contracts; they are not silently charged to
or exempted from the current full-rebuild screen.

## Consequences

- The initial host experiment was test-only: production asset, renderer, tracking
  engine and firmware remained unchanged. That approval did not authorize a new
  dependency, target build, flash, board access or toolchain change. The subsequent
  bounded target authorization below is separate.
- Report representation size, remaining work in all eleven existing measurement
  scenes, isolated shape differences at tight zoom/limb/viewport boundaries, and
  risks to code size and scratch. State any experimental precision and approximation
  thresholds explicitly; these are not adopted appearance tolerances.
- Exact pixel parity with the old renderer is a diagnostic, not a mandatory
  replacement criterion. Preserve the accepted angular orientation look and
  [0053's ordinary-gap-as-bug rule](0053-no-resolvable-coastline-omissions.md).
  [0054's exceptional-only COAST ERROR](0054-first-executable-coastline-comparison.md)
  and [0055's deferred all-view proof](0055-defer-all-view-coastline-proof-for-m4.md)
  remain intact. Do not restart theorem work to prolong the trial.
- A modest host speedup, success only in tight views, unacceptable shape differences,
  or needing several more research studies is grounds to stop. A promising result
  is permission to recommend a target check, not to perform one or claim device
  viability. Provisional resource ceilings and whole-device 20 Hz remain unproven.
- The fallback is a direction to scope separately, not authorization to add a cache,
  persistence, location UI or a slower preparation policy now.

## Follow-up — 2026-10-24 UTC

The [host representation trial](../evaluations/m4-coastline-representation-spike.md)
is complete and recommended for the single target feasibility check. The user
reviewed the candidate comparison and accepted its appearance ("The candidate
looks great"). This closes the host candidate's appearance-review gate, separately
from its numerical tests. The user then explicitly authorized the measurement-only
implementation, target build and — after exact-image gates pass — flash/run on the
S3. This covers the single feasibility comparison, not a toolchain change, Sharp
work, production-renderer replacement or additional optimization campaign. Measured
resource gates and device adoption remained open at authorization; the stop/pivot
rule was unchanged.

The [single target check](../evaluations/m4-coastline-representation-target.md) is
now complete: maximum rebuild 22,489 µs versus 5,000 µs, compiled increment
58,156 B, and candidate-specific nested stack reservations above 1 KiB. All 48
captured framebuffers match host bytes and no measured draw allocated heap. Despite
the ~435× S3 improvement, the candidate does not meet the current time/scratch
screens. Stop general-renderer tuning and scope the location-preparation fallback;
no further target run or cache implementation follows automatically. The accepted
appearance/representation remains useful work, not a discarded product direction.
