---
description: Audit and consolidate documentation without losing valuable context
---
Clean up Overhead's documentation. Aim for a smaller, clearer default reading
path, not the fewest possible lines. This is documentation maintenance, not a
new implementation slice or an automatic session wrap-up.

## Audit before editing
1. Read AGENTS.md, docs/HANDOFF.md, docs/PLAN.md, and docs/DESIGN.md. Inspect Git
   status and existing changes; preserve unrelated work and ask if ownership
   is unclear. Inventory tracked documentation and project prompts, including
   crate READMEs, decisions, fixture provenance and evaluation guides.
2. Map each document's purpose and the normal session-reading path. Read the
   documents needed to assess duplication, stale guidance, contradictions,
   broken links, and misplaced detail. Inspect historical records only when
   relevant; don't load every log or raw evidence file just to clean prose.
   State any coverage limits rather than claiming an exhaustive audit.
3. Check suspect claims against relevant code/tests and accepted decisions.
   Distinguish implementation, intended behavior, agreed direction, unapproved
   exploration and historical observations. Code establishes what exists, not
   necessarily what is intended; the newest document does not automatically win.
   Briefly summarize the findings and intended edits before making changes;
   straightforward editorial fixes do not require a routine approval pause.

## Consolidate safely
4. Give each fact or contract one authoritative home; replace duplicate detail
   with useful links, retaining enough local context to understand the document.
   Follow AGENTS.md's document roles. Keep HANDOFF focused on resuming work,
   PLAN on scope/status, DESIGN on intent, decisions on rationale/contracts,
   and usage/provenance instructions near their owners. Code/tests own API detail.
5. Preserve rationale, constraints, unresolved questions, rejected/unapproved
   directions that guard against accidental implementation, and verification
   limitations. Remove redundant wording, not unique meaning. Don't add a new
   index, summary or archive unless it actually reduces navigation/loading cost;
   don't move clutter into another document on the default reading path.
6. Treat PROGRESS as append-only history and accepted decisions as historical
   records, not prose to rewrite to match today. An older statement is not by
   itself a contradiction. Link or clearly annotate supersession only when
   supported by an accepted decision; never silently reverse one. Leave frozen
   evidence, raw captures, fixture data and provenance facts intact. Don't
   regenerate evidence or claim old verification applies to new changes.
   Before editing fixture or evaluation documentation, check whether the file
   itself is hashed by a frozen manifest. Preserve pinned document bytes, not
   just the data and provenance facts; clarify current status from an unpinned
   owner document instead.
7. Ask focused questions before resolving anything that would choose product
   intent, reverse a decision, change milestone scope, or discard potentially
   valuable context. Explain the conflict with file paths and a recommendation.
   Leave unresolved material intact; safe independent edits may continue.
   Don't change application code, dependencies or toolchains, access hardware,
   or run interactive programs as part of this cleanup.

## Verify and report
8. Review the final diff for lost meaning and new contradictions. Check local
   links (including changed heading anchors), moved-file references, referenced
   paths, and whitespace with `git diff --check`. Inspect existing staged
   changes without altering the index. Do not run every documented command:
   validate risky commands from their contracts and report unverified claims.
   For docs-only edits, Rust checks are not required; state what was actually
   checked and distinguish it from prior verification.
9. Summarize the audit scope, what was consolidated/removed and why, where
   preserved context now lives, and outstanding questions or unchecked areas.
   Don't claim the entire documentation set is contradiction-free. Do not
   commit or push as part of this prompt; use /handoff when the user requests
   session wrap-up. Avoid adding duplicate audit summaries to project docs.
