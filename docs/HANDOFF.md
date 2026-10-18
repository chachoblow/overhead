# Overhead — Handoff

_Last updated: 2026-10-18 UTC. Next: scope one bounded M4 secondary-information comparison; do not implement by default._

## State
[M3 complete](PLAN.md#m3--minimal-real-data-radar-complete). The square/diagnostic
simulator remains default; opt-in wide radar has five true-state fields,
transitions, debug and [0048's user-accepted empty-view notices](decisions/0048-refined-empty-view-notices.md).
[0049's stable wide selection](decisions/0049-stable-wide-selection.md) is
implemented and user-accepted after interactive review: a successfully tracked
above-horizon selection survives zoom- and motion-driven viewport exits, with
current strip/debug values. Tab chooses an in-view marker; genuine loss falls
back to the first in-view ID or clears. Square/circular remain visible-only.
[Simulator usage](../sim/README.md#wide-radar-comparison) has an interactive recipe;
a standalone snapshot cannot show continuity.

## This session
- Began on clean `main` at `4a99d80`. Agreed 0049, then separately authorized
  implementation. `app/src/lib.rs` changes wide reconciliation and Tab;
  `render/src/debug.rs` avoids a misleading no-selection message. Added
  app/render/sim regressions in those crates and updated `sim/README.md`,
  `DESIGN.md`, `PLAN.md` and the 0049 decision. User confirmed review and
  acceptance of both code/contract and interactive executable behavior.
- No new field, selected-arrow highlight, ranking, or device policy was approved.

## Next bounded step / limits
Review the current wide normal view (including selected and empty-view states)
and propose **one** bounded secondary true-state readout comparison for user
approval, or recommend adding none if it would reduce radar clarity. Do not
implement or assume time/data age is required; [PLAN](PLAN.md#m4--useful-display-behavior)
retains separate empty-sky/countdown and coastline options. Circular radar,
field and graph alternatives are exploration, not selected. M2 budgets/scheduling,
RAM/safe stack, Sharp output and RWX remain open; keep
[0023's frozen evidence](decisions/0023-frozen-m2-revision-isolation.md) isolated.

## Verification / limits
Fresh wrap-up: workspace check/tests, app/render no-default-feature checks,
formatting, strict all-target app/render/sim Clippy, local links and whitespace
pass. User interactive acceptance is separate; no agent-run SDL replay, hardware,
firmware or frozen-M2 change. No physical-display or target-performance claim.
