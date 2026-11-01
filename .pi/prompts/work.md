---
description: Own one work item from definition to verified outcome
---
Carry one Overhead work item through to its verified outcome. You own the
execution plan; the user owns intent and the gates listed below. Contract:
[0072](../../docs/decisions/0072-outcome-sized-work-items.md).

A **work item** is an independently verifiable capability inside the current
PLAN milestone: one coherent outcome a reviewer can check from fresh evidence
without reading how it was built. Size it by outcome, not by session; it may
span sessions. A module, test, build, flash or capture is a step, not a work
item. Split an outcome only where each part is verifiable and useful alone.

## Define or resume
1. If `docs/work/` holds a file, resume that item: read the file, then check
   the checkout against it (the file and the repo outrank your memory). Do not
   start a second item while one is active unless the user asks.
2. Otherwise orient as in `.pi/prompts/next.md` steps 1–2 and create
   `docs/work/<slug>.md` from the template below. Its outcome, done-when and
   boundaries come from the user's request, PLAN and accepted decisions; the
   plan is yours.
3. List the human gates the item will foreseeably hit. Raise the open ones
   together, once, with recommendations, before starting work that depends on
   them; do independent steps meanwhile. If the user already authorized the
   item (by request or an accepted decision covering it), don't ask again.

## Execute
4. Work through the plan step by step without pausing for confirmation or
   progress reports. Revise the plan freely when you learn something; the
   outcome and boundaries change only with the user.
5. Keep the work file current enough that a fresh agent with only the repo
   could resume: update it when a step completes, when a discovery or
   assumption changes the plan, before any long or risky action (flash, long
   capture, broad refactor), and before asking the user anything. Record
   facts, not narrative; replace stale entries rather than appending a diary.
6. Commit locally at verified checkpoints (`cargo check --workspace` passes
   after code changes), including the work file, with descriptive messages.
   Push at handoff (`.pi/prompts/handoff.md`), not per checkpoint.
7. For a question that does not block the next steps, record it under Open
   questions with the reversible default you are using, keep going, and ask
   at the next gate or at completion.

## Stop and ask only for
- **Judgment that is the user's:** product intent, visual or physical
  acceptance, or a choice an accepted decision leaves to the user.
- **Changed requirements:** what you found contradicts the outcome, its
  boundaries or an accepted decision; propose a new decision rather than
  working around it.
- **High-risk or reserved actions:** toolchain or machine-setup changes,
  credentials, anything ~/.claude/CLAUDE.md or AGENTS.md reserves, frozen M2
  evidence, destructive Git, external requests beyond agreed limits, or
  anything a decision gates on separate authorization. Flashing is not one
  ([0069](../../docs/decisions/0069-flash-without-per-flash-approval.md)).
- **A user action:** the user must observe the panel or press buttons. Finish
  all preparation first, then ask once with exact instructions.
- **Genuine blockage:** verification still fails after reasonable diagnosis
  and no in-scope fix exists.

Before asking, set the work file's Status to `blocked: <what is needed>` and
give a recommendation. Everything else (routine design choices inside the
contract, refactors, test strategy, dependency choices an accepted decision
covers) is yours: decide, record it if it matters, and continue.

## Complete
8. Check every done-when item against fresh evidence and run the AGENTS.md
   checks. Record what was actually run.
9. Promote what future sessions need, then delete the work file (Git keeps
   it):
   - settled choices and rejected alternatives worth guarding → a decision
     (new, or a status note on the governing one);
   - usage, evidence and limits → the owning README or evaluation;
   - status → PLAN; unfinished concerns and risks → HANDOFF or PLAN;
   - the outcome and its verification → PROGRESS (handoff step 4).
   Drop step history, dead ends and logs that change no future choice.
10. Completion is the normal handoff boundary: run `.pi/prompts/handoff.md`,
    then stop. Don't start the next item unless the user asked for more.

## Template
```markdown
# <Outcome, as a capability>
Status: active | blocked: <what is needed> | verifying
Started: YYYY-MM-DD. Scope: PLAN <milestone/item>; decisions <links>

## Outcome
What will be true and checkable when this is done.

## Done when
- Fresh evidence a reviewer can check (tests, captures, user observation).

## Boundaries
Out of scope; limits (request budgets, frozen evidence, etc.).

## Human gates
- <gate>: needed / asked YYYY-MM-DD / answered: <answer>

## Plan
- [ ] Steps, revised freely.

## Discoveries and assumptions
Facts that changed the plan; defaults chosen pending an answer.

## Verification
Command or observation → result, at revision.

## Open questions
Non-blocking; asked at the next gate.
```
