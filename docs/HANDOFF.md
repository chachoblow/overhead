# Overhead — Handoff

_Last updated: 2026-10-17 UTC. Next: scope the first glyph slice._

## Current state / agreed direction
[0033's wide radar/arrows](decisions/0033-wide-radar-comparison-contract.md) and
[0034's labels](decisions/0034-selected-marker-identification.md) are implemented;
square diagnostics remain default. The user confirmed review and acceptance of
[0035's 70° hide / 60° restore](decisions/0035-label-zoom-visibility.md) for now.
Keep configurable hysteresis, selected diamonds, label placement and warnings.
[0036](decisions/0036-shared-glyph-capsule.md) selects one shared rounded glyph
capsule: 1px outline, one centered break on each long side, black on white.
This is visual direction only; glyphs/capsule/transitions are not implemented.

## Session changes
Docs/static image only: `docs/{DESIGN,PLAN,HANDOFF,PROGRESS}.md`, decisions
0032/0035/0036 and `docs/mockups/0036-glyph-capsule.png`. The retained
[reference and provenance](DESIGN.md#glyph-capsule-reference) replace reliance on
chat or temporary files. Circular/info-brick, noise-field and population-graph
mocks were exploration, not an approved pivot. No code, dependency, lockfile,
fixture, firmware or frozen-M2 changes.

## Next step / boundaries
Agree a bounded first glyph contract before coding. Recommendation, not approved
scope: static true-elevation glyph inside the shared capsule; decide footprint,
position, exact enclosure geometry and absent-selection/warning coexistence.
Category mappings, non-elevation scales, speed/period semantics and transitions
remain open; do not copy illustrative mock values into policy. Keep approved
radar/arrows, visible-only selection and square baseline. No history buffers,
prediction, panning, slowdown, fetching or production limits/cadences implied.

## Verification / risks
Wrap-up: Markdown local links/anchors, retained PNG dimensions/1-bit mode and
radar-pixel provenance, instruction consistency and diff whitespace checked.
Earlier: headless snapshots, temporary real-data mock export and a passing
`cargo check --workspace`. No Rust checks/tests rerun for this docs/image wrap-up.
No agent interactive, physical-display or performance validation. Build/CLI checks
serially (prior relink ENOENT). M2 budgets/scheduling, RAM/safe stack, Sharp bring-up
and RWX remain open; frozen evidence uses
[0023's pinned revision](decisions/0023-frozen-m2-revision-isolation.md).
