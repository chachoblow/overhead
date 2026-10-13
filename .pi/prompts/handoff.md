---
description: Verify, prepare a fresh-session handoff, then commit and push
---
The session is ending. Make the repository ready for a fresh session, then
commit and push the session's changes. Proceed without another approval unless
there is ambiguity or a concern. Do not start a new implementation slice.

## Review and verify
1. Read AGENTS.md and the current HANDOFF/PLAN/DESIGN. Inspect Git status,
   staged and unstaged diffs, untracked files, the branch/upstream, and outgoing
   commits. Identify this session's changes; don't assume every change is ours.
   Preserve unrelated work. If ownership or the intended commit scope is
   unclear (including mixed changes in a file), ask before staging or committing.
2. Run verification appropriate to the actual diff. After code changes,
   `cargo check --workspace` is required; run relevant tests and other checks
   warranted by the affected crates and contracts. For docs/prompts-only work,
   check whitespace, local links/paths, and instruction consistency; explain
   why Rust checks were not rerun. Never pass off prior verification as new.
   Fix in-scope failures and rerun; stop before commit/push for unresolved
   failures, unavailable required checks, suspected secrets, or other concerns.
   Never flash hardware or change toolchains without explicit approval. Keep
   frozen M2 evidence isolated per AGENTS.md; don't refresh it to hide drift.

## Preserve resumable context
3. Update docs/HANDOFF.md (aim for ≤40 lines): current state, brief session
   changes with file paths, one concrete next step, unresolved decisions/risks,
   and exact verification status/limitations. Preserve still-relevant context
   from earlier sessions. Distinguish implemented, agreed, and unapproved work.
   Link to contracts/usage; a new session must not need this chat or /tmp files.
4. Append a dated 3–5 line entry to docs/PROGRESS.md. Check off genuinely
   finished items in docs/PLAN.md; leave it unchanged if milestone status did
   not change. Record settled choices in docs/decisions/NNNN-<slug>.md using
   the template. Update other docs only when their contracts/usage changed.
5. Review the final diff for accuracy, stale claims, secrets, unrelated files,
   and accidental build artifacts. Run `git diff --check` and check the staged
   diff as well. Save the docs directly; no routine preview/approval pause.

## Commit and push
6. Stage only the reviewed session changes, using explicit paths or hunks.
   Inspect the complete staged diff before committing with a descriptive
   message. Don't include unrelated pre-staged changes or create an empty
   commit. Don't amend existing commits unless explicitly requested.
7. Verify the intended remote/tracking branch and refresh its state before
   pushing. Review all outgoing commits, not just the newest one. If the
   destination or outgoing history is unclear, the branch is detached, the
   upstream is missing, or histories diverge, stop and ask. Do not force-push,
   reset, auto-stash, rebase, merge, or otherwise rewrite/discard work to make
   a push succeed. Push only the intended branch to its configured upstream.
   If a push fails, preserve the local commit and report the error; don't
   bypass hooks or change credentials/remotes/protections. If there is nothing
   new to commit, push only clearly intended, reviewed outgoing session commits.
8. Confirm final Git status and whether the upstream contains the commit.
   Report a concise summary: changes, verification, commit hash, push result,
   any remaining local changes/blockers, and the next session's starting point.
   Never claim a successful push without confirmation.
