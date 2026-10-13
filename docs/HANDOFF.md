# Overhead — Handoff

_Last updated: 2026-10-13 UTC. Next product session: revisit M3 panel design._

## Current state / user direction
Implemented: 240×240 borderless square, provisional 20° FOV, fixed 75,000 km
camera under [0028](decisions/0028-borderless-square-viewport.md). Keep breathing
room. [0029](decisions/0029-normal-and-debug-presentation-direction.md) agrees
normal/debug separation and routine diagnostic-field reduction, not a layout.
The brief is **alive and active, but not busy**; Are.na references inform DESIGN.
Both normal-panel mockups remain unapproved. Normal/debug splitting, debug
controls, graphical readouts and prediction integration are NOT implemented.
Physical tracking, true horizon eligibility and the diagnostic panel are unchanged.

## This session / workflow
Added `.pi/prompts/next.md`; expanded `.pi/prompts/handoff.md` and routed natural
language equivalents from `AGENTS.md` under
[0030](decisions/0030-session-orientation-and-wrap-up.md). `/next` recommends only;
`/handoff` verifies, updates resumable context, commits and pushes without another
approval unless ambiguity or concern arises. Run `/reload` in an existing Pi
session to refresh prompt commands. No application code or product scope changed.

## Next step / boundaries
Read DESIGN/0029, then scope panel exploration or a small normal/debug split and
field-reduction slice with the user. Layout/graphics, remaining fields (including
range), access controls and failure communication remain open. Do not implement
an earlier mockup by default; disposable `/tmp` mockups are not specifications
or required assets. Stay in M3; no slowdown, fetching or production population/
cadence decision. [Simulator usage](../sim/README.md) owns run/comparison commands.

## Verification / risks
This session: docs/prompt whitespace, local paths/links and instruction review;
no Rust checks rerun because only workflow/docs changed. Prior 2026-10-12 staged
source export passed workspace check/tests, no-default checks, Clippy/fmt,
simulator build and headless/dummy-loop checks; details in [PROGRESS](PROGRESS.md).
No new runtime, interactive or hardware verification. M2 budgets/scheduling,
whole-device RAM/safe stack, Sharp bring-up and RWX remain open. Frozen evidence
uses [0023's pinned revision](decisions/0023-frozen-m2-revision-isolation.md), not
this workspace. Host throughput is not target capacity; no evidence renewed.
