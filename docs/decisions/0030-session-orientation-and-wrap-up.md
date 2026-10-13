# 0030 — Session orientation and wrap-up

Date: 2026-10-13
Status: accepted

## Decision
Use `.pi/prompts/next.md` for next-task recommendations and
`.pi/prompts/handoff.md` for verified, resumable session wrap-up. A wrap-up
request authorizes committing and pushing the reviewed session changes
without another approval, unless ambiguity or a concern requires user input.

## Why
These are recurring user requests. Documentation-only handoffs and mandatory
preview pauses leave routine completion work unfinished; automatic
implementation during orientation would go beyond the user's request.

## Consequences
AGENTS.md routes equivalent natural-language requests to the same workflows.
Orientation recommends a bounded slice without editing. Wrap-up preserves
fresh-session context, verifies the actual changes, and reviews commit scope
and push destination; uncertainty, unresolved verification failures, or Git
problems stop the workflow rather than triggering destructive recovery.
Hardware and toolchain approval requirements remain unchanged. The prompt
files own the procedural details; this decision does not alter M3 scope.
