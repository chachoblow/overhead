---
description: One-time setup only — completed in Overhead; use /resume normally
---
Overhead's bootstrap is complete. If AGENTS.md has no `TODO(bootstrap)` sections,
stop and recommend `/resume` for orientation or `/next` for a bounded task;
do not rerun setup or reopen settled product choices. The procedure below applies
only to an explicitly requested, unfinished bootstrap.

1. Read AGENTS.md, docs/HANDOFF.md, docs/PLAN.md, and docs/DESIGN.md, then
   decisions and usage docs relevant to the missing setup information.
2. Inspect the Cargo workspace: Cargo.toml files, crate layout, entry points.
   Don't change any code.
3. Fill in the TODO(bootstrap) sections in AGENTS.md: the crate list and the
   exact commands. Run each command to confirm it works, except the
   simulator (it opens a window) and anything that flashes hardware.
4. Tell me anything in the code that conflicts with DESIGN.md or that
   DESIGN.md doesn't cover.
5. Then go through the open questions in DESIGN.md with me, one at a time.
   Record answers in DESIGN.md, and record firm choices as
   docs/decisions/NNNN-<slug>.md using the template.

Show me each doc change before saving it.
