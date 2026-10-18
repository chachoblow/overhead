# Overhead — Handoff

_Last updated: 2026-10-18 UTC. Next: implement 0047 and review executable output._

## State
M3 opt-in wide radar and five static fields are implemented; [0046](decisions/0046-selected-data-glyph-transition-contract.md)
adds visually accepted 250 ms Elev/Alt/Spd/Per transitions. Cat, ID and selected
identity switch immediately; physical values stay true. Default square diagnostics
and static headless snapshots remain unchanged ([usage](../sim/README.md)).
[0029](decisions/0029-normal-and-debug-presentation-direction.md) established
normal/debug separation; [0047](decisions/0047-text-first-debug-access-contract.md)
now accepts a radar-free text-first tracking/data-health debug page, interactive
wide `v` toggle and explicit `--wide --debug` start. **No debug code exists yet.**

## This session
Design only: added `docs/decisions/0047-text-first-debug-access-contract.md`;
updated `docs/PLAN.md` (agreed vs open implementation), `docs/DESIGN.md` and
`docs/PROGRESS.md`. No new dependency, app/core semantics, fixtures, firmware,
toolchain or frozen-M2 evidence; preserve [0023's pinned revision](decisions/0023-frozen-m2-revision-isolation.md).

## Next bounded step
Implement 0047 in simulator config/input and the renderer, then seek separate
executable visual acceptance. Debug is text-first, not a square radar: `--debug`
requires `--wide`, is interactive-only, and `v` toggles without changing app
view/selection/time. Keep existing square/default and wide-normal snapshots,
true samples, host failure reporting and condition-driven normal notices. Review
400×240 1-bit text fit; long names/error causes must not silently truncate.
Do not add selection/collision policy, slowdown, fetching, prediction, scheduling,
production limits or hardware work. M2 budgets, RAM/safe stack, Sharp bring-up
and RWX remain open.

## Verification / limits
This session: documentation-only local links/paths and instruction consistency
checked; unstaged and staged whitespace checked at wrap-up. Rust checks/tests
not rerun because there are no code changes. Previous 0046 workspace check/tests,
fmt, app/render no-default-feature checks and strict Clippy passed in an earlier
session, **not** as 0047 verification. No debug executable review, physical-display
or S3 timing validation; no 20 Hz claim.
