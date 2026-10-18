# Overhead — Handoff

_Last updated: 2026-10-17 UTC. Next: scope M3 normal/debug access._

## Current state / agreed direction
[0045](decisions/0045-static-group-category-contract.md) completes the five
static selected fields in opt-in wide view. [0046](decisions/0046-selected-data-glyph-transition-contract.md)
is implemented and user visually accepted: Elev/Alt/Spd/Per transition over
250 ms on selected-ID changes, while Cat, ID and selection switch immediately.
Physical readouts stay true; square diagnostics remain default and headless
snapshots remain static. [0029](decisions/0029-normal-and-debug-presentation-direction.md)
agrees normal/debug separation as direction, not a control or implementation.

## Session changes
`render/src/presentation.rs` retains bounded explicit-time drawing parameters;
`render/src/{glyphs,lib}.rs` share static endpoints with wide-only `draw_at`.
`sim/src/main.rs` supplies monotonic elapsed time for interactive replay; PNGs
still use `draw`. Renderer timeline/pixel tests and [simulator usage](../sim/README.md)
cover motion versus static output. No new dependencies, app/core semantics,
orbital fixtures, firmware or frozen-M2 changes. The user separately approved
0046's design, authorized implementation and accepted interactive appearance;
no physical-display approval follows. 0040–0045 field, category and notice
contracts stay accepted. Preserve [0023's pinned M2 revision](decisions/0023-frozen-m2-revision-isolation.md).

## Next step / boundaries
Recommend a **design-only** normal/debug access contract: settle simulator
entry/toggle and how objective diagnostics remain reachable before code or a
default-mode change. Preserve wide radar, true state and explicit notices. No
selection/collision policy, marker slowdown, production catalogue, prediction,
fetching, scheduling or hardware work is approved by 0046.

## Verification / risks
Wrap-up passed workspace check/tests, fmt, app/render no-default-feature checks
and strict all-target app/render/sim/tools Clippy; local links and whitespace
were checked. Renderer tests cover interruption, loss/recovery, retargeting and
static endpoint/square isolation; CLI snapshots cover static real data. User
reviewed the interactive animation; no agent-run interactive, physical-display
or S3 timing validation, and no 20 Hz claim. M2 budgets/scheduling, RAM/safe
stack, Sharp bring-up and RWX remain open.
