# Overhead — Handoff

_Last updated: 2026-10-06. Next: refine M2; implementation has not started._

## State
M1 and pre-M2 hardening are complete: local OMM → propagation → Earth-fixed/
geodetic → observer measurements. Render is a stub; sim is static; no firmware.
See [PLAN](PLAN.md) for scope and [tools usage](../tools/README.md) to try it.

## This session
Documentation only: shortened `PLAN.md`, `DESIGN.md`, decisions 0005–0010,
`tools/README.md`, and the fixture README; corrected `AGENTS.md`'s crate map.
Removed redundant `docs/notes/` (rationale/checks retained in decisions,
fixtures, and progress). Updated navigation and the handoff prompt to discourage
repetition. No milestone, contract, code, dependency, or fixture changes.

## Next step
Refine M2's first headless slice: local group configuration, NORAD-ID merge,
conflict/invalid-record policy, and provenance. Do not silently choose policy;
element epoch and fetch time are distinct. Use checked OMM ingestion
([0010](decisions/0010-propagation-and-omm-validation.md)).

Confirm hardware availability for propagation benchmarks and minimal Sharp
refresh testing; ask before flashing or changing toolchains. Capacity,
scheduling, and prediction accuracy/cost remain unmeasured (see PLAN).

## Risks / follow-ups
- Broaden independent references to resonant/low-inclination GEO, applicable
  half-day resonances, negative propagation times, and a non-LEO observer
  pipeline. These are coverage recommendations, not known pipeline failures.
- sgp4 2.4's epoch helper mishandles dates after February 2100; only our rotation
  avoids it ([0007](decisions/0007-coordinate-implementation-conventions.md)).
- Prefer timestamp-bound `state.to_ecef()`; low-level time/frame escape hatches
  remain caller responsibilities ([0009](decisions/0009-core-time-and-frame-api.md)).

## Verification
This session: local Markdown links and `git diff --check`; no build/test rerun.
Last implementation session: workspace/default/all-feature tests (58 + 3
compile-fail doctests), standalone core with/without OMM, Clippy, and fmt passed.
No known failing checks; hardware performance is still unverified.
