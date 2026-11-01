---
description: Recommend the next bounded Overhead task without starting work
---
What should we do next for Overhead?

1. Read AGENTS.md, docs/HANDOFF.md, docs/PLAN.md, and docs/DESIGN.md in
   that order. Follow the decision/usage links relevant to the next slice;
   read PROGRESS only if historical context is needed.
2. Inspect the current branch, working-tree status, and relevant code/tests.
   Treat the handoff as context, not proof that the checkout still matches it.
   Distinguish implemented work, agreed direction, and unapproved exploration.
   If `docs/work/` holds an active item, recommend resuming it unless there is
   a reason not to.
3. Recommend one work item inside the current milestone: an independently
   verifiable outcome, not an implementation step or a session-sized slice
   (see `.pi/prompts/work.md`). Give its outcome, done-when evidence,
   boundaries, likely files, and the human gates it will hit. Don't simply pick
   the first unchecked box or reopen settled decisions.
4. Briefly report current state, the recommendation, and blockers or decisions
   needed. Front-load every foreseeable decision so execution can run without
   stopping; ask the smallest useful questions, each with a recommendation,
   rather than silently choosing a design.

This is orientation only: do not edit files, start implementation, commit,
launch interactive programs, or access hardware. Do not claim checks were run
unless you actually ran them.
