# 0031 — Consolidate orientation prompts

Date: 2026-10-13
Status: accepted

## Decision
Remove `.pi/prompts/resume.md` and retain `.pi/prompts/next.md` as the session
orientation and bounded next-task recommendation prompt.

## Why
The prompts overlap: both recap current state, identify next work and stop
before implementation. Keeping a separate recap-only prompt was considered,
but the user chose removal; `/next` already checks the checkout and provides
scope and verification for its recommendation.

## Consequences
Use `/next` for orientation; its behavior is unchanged. Historical `/resume`
mentions remain in the progress log, not active instructions. This supplements
[0030](0030-session-orientation-and-wrap-up.md) without changing wrap-up
safeguards or product scope.
