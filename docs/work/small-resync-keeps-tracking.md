# Small SNTP resyncs keep tracking state
Status: active
Started: 2026-11-03. Scope: PLAN M6 (time trust / device app); decisions
[0062](../decisions/0062-no-trusted-time-display-preflight.md),
[0063](../decisions/0063-bounded-utc-holdover-direction.md),
[0077](../decisions/0077-m2-operating-budget-visual-group.md)

## Outcome
An accepted SNTP renewal whose UTC is within 100 ms of the old anchor's
projected UTC adopts the new anchor but keeps app samples and selection: no
complete recompute, no BOOT selection reset. First anchors, larger jumps,
expiry, unsynchronized/leap replies and clock faults still clear everything.

## Done when
- Host tests: small forward and backward step keep samples/selection; >100 ms
  jump clears; renewal after expiry clears; staggered update tolerates a small
  backward step without a complete recompute.
- AGENTS.md checks and firmware host suites pass.
- Board: `live-fetch-radar` capture ≥30 min (~15 renewals) shows renewals as
  `Resynced` and `update_max_us` per 10 s window staying near the staggered
  cost (no ~175 ms recomputes after the first load), selection retained.

## Boundaries
No change to holdover, budget, leap or source rules (0063–0066). Fresh frame
after every renewal stays. No toolchain change. One CelesTrak request per boot.

## Human gates
- Agreed 2026-11-03: threshold = |new − old projected| ≤ 100 ms; backward step
  ≤ window staggered (symmetric rule); record as a note on 0063.
- Optional user BOOT check: not required for done-when.

## Plan
- [x] `utc_trust.rs`: compare against old anchor; `ObservationOutcome::Resynced { step_us }`; tests.
- [x] `app` `update_staggered`: symmetric `|elapsed| < window`; tests.
- [x] Check + tests + firmware host suites; commit.
- [ ] Build `live-fetch-radar`, gate, flash, ≥30 min capture; record evidence.
- [ ] 0063 note, firmware README, HANDOFF/PLAN/PROGRESS; delete this file; handoff.

## Discoveries and assumptions
- `update_staggered` fell back to complete `update` on any backward step.

## Verification

## Open questions
