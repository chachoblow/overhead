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
Deleted `.pi/prompts/bootstrap.md` at the user's request: one-time setup is
complete. Use [resume](../.pi/prompts/resume.md) for orientation or
[next](../.pi/prompts/next.md) for a bounded task recommendation; historical
bootstrap mentions in `PROGRESS.md` are retained. No application, milestone or
accepted-decision changes; [0030's workflow](decisions/0030-session-orientation-and-wrap-up.md)
and [docs-cleanup](../.pi/prompts/docs-cleanup.md) safeguards remain unchanged.
Run `/reload` in an existing Pi session to refresh prompt commands.

## Next step / boundaries
Read DESIGN/0029, then scope panel exploration or a small normal/debug split and
field-reduction slice with the user. Do not implement an earlier mockup by default;
disposable `/tmp` mockups are not specifications or required assets. Stay in M3;
no slowdown, fetching or production population/cadence decision.
[Simulator usage](../sim/README.md) owns run/comparison commands.

## Verification / risks
Prompt deletion and handoff/history only: final diff, whitespace, handoff local
links/anchors/paths, remaining bootstrap references and instruction consistency
checked. No Rust checks rerun because no code/build inputs changed; no runtime,
interactive or hardware checks or external-source revalidation. Prior runtime checks remain in
[2026-10-12 verification](PROGRESS.md#2026-10-12-utc-square-default-fresh-session-verification-before-commit),
not evidence of later changes.
M2 budgets/scheduling, whole-device RAM/safe stack, Sharp bring-up and RWX remain
open. Frozen evidence uses [0023's pinned revision](decisions/0023-frozen-m2-revision-isolation.md),
not this workspace. Host throughput is not target capacity; no evidence renewed.
