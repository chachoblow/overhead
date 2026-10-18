# Overhead — Handoff

_Last updated: 2026-10-18 UTC. Next: scope a bounded M4 selection slice, not implement it._

## State
[M3 complete](PLAN.md#m3--minimal-real-data-radar-complete): moving, zoomable real-data
radar with opt-in wide view/five fields/transitions/debug; square diagnostics
remain default. M4's [0048 empty-view notices](decisions/0048-refined-empty-view-notices.md)
are implemented and user-approved on fresh executable output: arrows yield
`SKY OUTSIDE VIEW`, clean nonempty no-above-horizon samples yield
`NONE ABOVE HORIZON`, and uncertain/remaining empty cases yield `NO MARKERS`.
Independent warnings persist; [simulator usage](../sim/README.md#wide-radar-comparison)
owns reproduction. [0048 mocks](mockups/0048-empty-view-proposal.md) remain illustrative.

## This session
- From clean `main` at `01c62de`, user separately authorized 0048 implementation.
  `render/src/lib.rs` now classifies the wide status using current app truth;
  backing grows with the longest active line. Square, app truth, selection and
  warning lifecycle remain unchanged.
- Added focused raster/classification/recovery and real-data/omission coverage in
  `render/src/lib.rs` and `sim/tests/snapshot.rs`; updated `sim/README.md`,
  `PLAN.md`, `DESIGN.md`, 0048 and its mock provenance. User then accepted the
  fresh executable demo-arrow and one-ISS horizon screenshots separately from
  the earlier contract composites.

## Next bounded step / limits
Review current wide replay selection behavior and propose **one bounded stable
automatic-selection contract** for user approval before implementation. Scope
selection continuity, entry/loss and zoom changes without choosing new normal
readouts or changing physical eligibility. [PLAN](PLAN.md#m4--useful-display-behavior)
retains other M4 options; circular-radar/field/graph alternatives are unapproved.
M2 budgets/scheduling, RAM/safe stack, Sharp output and RWX remain open;
keep [0023's frozen evidence](decisions/0023-frozen-m2-revision-isolation.md) isolated.

## Verification
Workspace check/tests, app/render no-default-features check, strict app/render/sim
Clippy, formatting, local links and whitespace pass; headless real-data
snapshots were generated and checked without SDL. Fresh
executable visual acceptance is user feedback, not target/panel validation.
No hardware, firmware, dependency, fixture or frozen-M2 work this session.
