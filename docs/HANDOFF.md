# Overhead — Handoff

_Last updated: 2026-10-13 UTC. Next product session: M3 panel design._

## Current state / user direction
The borderless-square perspective simulator is implemented under
[0028](decisions/0028-borderless-square-viewport.md); the diagnostic panel is unchanged.
[0029](decisions/0029-normal-and-debug-presentation-direction.md) agrees normal/debug
separation and routine field reduction, not a layout. Neither panel mockup is
approved. The split, debug controls, graphical readouts and prediction integration
are NOT implemented. [DESIGN](DESIGN.md#interface-intent) owns the visual brief and
breathing-room preference; physical tracking and true horizon eligibility remain unchanged.

## This session
Deleted `.pi/prompts/resume.md` at the user's request; use
[next](../.pi/prompts/next.md) for orientation and a bounded task recommendation.
[0031](decisions/0031-consolidate-orientation-prompts.md) records the consolidation;
`/next` and [0030's wrap-up workflow](decisions/0030-session-orientation-and-wrap-up.md)
are unchanged. Updated this handoff and `docs/PROGRESS.md`; historical prompt
mentions remain in the log. No application or milestone changes.

## Next step / boundaries
Read DESIGN/0029, then scope panel exploration or a small normal/debug split and
field-reduction slice with the user. Do not implement an earlier mockup by default;
disposable `/tmp` mockups are not specifications or required assets. Stay in M3;
no slowdown, fetching or production population/cadence decision.
[Simulator usage](../sim/README.md) owns run/comparison commands.

## Verification / risks
Docs/prompts only: final diff, whitespace, changed-document local links/anchors/
paths, remaining resume references and instruction consistency checked.
No Rust checks rerun because no code/build inputs changed; no runtime,
interactive or hardware checks or external-source revalidation.
Prior runtime checks remain in
[2026-10-12 verification](PROGRESS.md#2026-10-12-utc-square-default-fresh-session-verification-before-commit),
not evidence of later changes.
M2 budgets/scheduling, whole-device RAM/safe stack, Sharp bring-up and RWX remain
open. Frozen evidence uses [0023's pinned revision](decisions/0023-frozen-m2-revision-isolation.md),
not this workspace. Host throughput is not target capacity; no evidence renewed.
