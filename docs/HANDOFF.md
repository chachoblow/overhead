# Overhead — Handoff

_Last updated: 2026-10-18 UTC. Next: authorize and implement the approved 0047 debug layout._

## State
M3 opt-in wide radar, five selected fields and [0046's transitions](decisions/0046-selected-data-glyph-transition-contract.md)
are implemented and visually accepted. True samples, normal notices and static
square/default and headless output remain unchanged ([usage](../sim/README.md)).
[0047](decisions/0047-text-first-debug-access-contract.md) accepts radar-free
tracking/data-health debug, interactive wide `v` access and `--wide --debug`.
Its layout/overflow review is now approved. **No debug code exists yet.**

## This session
Design only: retained the approved [layout reference](mockups/0047-debug-layout-proposal.md)
and [six native-size illustrative pages](mockups/0047-debug-native-proposal.png).
0047 records approval; PLAN checks off only the preimplementation layout review;
DESIGN links that status and PROGRESS records the session. The reference owns
geometry, numeric formatting, two-row names and explicit host-log overflow rules.
The retained docs/image suffice without this chat or disposable preview scripts.
Layout approval is not implementation authorization or executable acceptance.

## Next bounded step
Obtain implementation authorization, then implement 0047 in `sim/src/config.rs`,
`sim/src/main.rs` and an independent renderer page, with regression coverage.
Preserve true state/geometry across toggles, Tab/zoom while debugging, normal
notices, complete host causes and historical square/wide-normal snapshots.
Reject debug snapshots; return to a current normal frame without queued motion.
Run workspace checks/tests, then seek separate executable visual acceptance.
No new selection/collision policy, fetching, prediction, slowdown or hardware.
M2 budgets/scheduling, RAM/safe stack, Sharp bring-up and RWX remain open;
keep [0023's frozen evidence](decisions/0023-frozen-m2-revision-isolation.md) isolated.

## Verification / limits
This session checked preview font/raw parity, native bounds, binary pixels,
deterministic regeneration and text-boundary controls; the user approved its
illustrative appearance, not physical samples. Wrap-up checked retained image
bounds/font cells, local links/anchors, doc consistency and unstaged/staged
whitespace. Rust checks/tests were not rerun: only docs and a reference PNG changed.
No simulator launch, executable debug review, physical-display or S3 validation;
no dependency, fixture, firmware/toolchain or frozen-M2 changes; no 20 Hz claim.
