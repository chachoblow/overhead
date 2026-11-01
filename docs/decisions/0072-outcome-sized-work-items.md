# 0072 — Outcome-sized work items with agent-owned execution

Date: 2026-11-01
Status: accepted (user request, 2026-11-01). Extends
[0030](0030-session-orientation-and-wrap-up.md).

## Decision
Take on work as **work items**: independently verifiable capabilities inside
the current PLAN milestone, sized by outcome rather than by session. Once a
work item is agreed, the implementing agent writes and maintains its own
execution plan in `docs/work/<slug>.md` and carries out every step until the
outcome is verified or genuinely blocked, asking the user only at the gates in
[`.pi/prompts/work.md`](../../.pi/prompts/work.md): the user's judgment,
changed requirements, high-risk or reserved actions, actions only the user can
perform, and unresolvable failures. On completion the agent promotes decisions,
evidence, unfinished concerns and the outcome to their owning documents,
deletes the work file and hands off.

## Why
Sessions were organized around session-sized slices (0030's "bounded slice",
PLAN's "refine each slice"), each started, committed and handed off
separately. 0071's slices 1–3, for example, form one capability but ran as
separate starts. The only durable state was HANDOFF, written at wrap-up, so an
agent that kept going risked losing its plan to context compaction, and
stopping after each step was the safe default. A single transient working
file makes continuing safe; explicit stop conditions make continuing the
default. An orchestrator, tracker or database was rejected: Markdown in the
repo plus existing prompts is enough for one developer and one active item.

## Consequences
- The work file is the recoverable state: AGENTS.md has agents read it first,
  including after compaction, and checkpoint commits include it. It holds the
  plan, discoveries, verification and open questions, not narrative history,
  and is deleted on completion; Git keeps the history.
- The user's authorization attaches to the work item, not to each step. Gates
  that an accepted decision or AGENTS.md/CLAUDE.md reserves still apply
  (toolchains, machine setup, credentials, frozen M2 evidence, visual and
  physical acceptance), and so does "separately authorized" wording in an
  existing decision. 0069's flashing policy is unchanged.
- Orientation (`next.md`) now recommends a work item with its done-when
  evidence and foreseeable gates, front-loading questions. Wrap-up
  (`handoff.md`) is normally triggered by work-item completion and links,
  rather than copies, an unfinished work file.
- Milestones, PLAN checkboxes and existing decisions are unchanged.
