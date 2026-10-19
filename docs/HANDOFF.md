# Overhead — Handoff

_Last updated: 2026-10-18 UTC. Next: scope one bounded M4 offline coastline-legibility comparison; do not implement by default._

## State
[M3 complete](PLAN.md#m3--minimal-real-data-radar-complete). Default simulator
is square/diagnostic; opt-in wide radar has five true-state fields, transitions,
text-first debug, [0048's accepted empty-view notices](decisions/0048-refined-empty-view-notices.md)
and [0049's accepted stable selection](decisions/0049-stable-wide-selection.md).
An above-horizon selected ID survives a viewport exit with current strip/debug
values; Tab picks an in-view marker, genuine loss falls back or clears. Square/
circular stay visible-only. [Interactive review](../sim/README.md#wide-radar-comparison)
is required to see continuity; a standalone snapshot has no selection history.

## This session
- Started on clean `main` at `d43c3bb`. Retained a native 1-bit range/no-extra-
  readout comparison and [provenance](mockups/0050-range-vs-no-readout.md): two
  selected cases and one no-selection case; the off-screen-selected case is an
  **illustrative composite**, not executable selection-continuity evidence.
- The user chose [0050](decisions/0050-keep-range-in-debug-and-defer-display-prediction.md):
  leave range in debug and defer display prediction/countdown study for now.
  Existing core pass search and [offline tool](../tools/README.md#overhead-passes)
  remain intact. No renderer/app/core implementation or new field was approved.
  PLAN/DESIGN record the scoped M4 boundary; no M4 checkbox was completed.

## Next bounded step / limits
Propose a native 1-bit **coastline legibility comparison** over the existing
perspective Earth at a few zooms, with selected/empty radar examples and no
silent data-source or raster choice. First check offline source provenance and
projection/occlusion fit; ask before implementation or adopting a source.
Do not reopen accepted range/selection/notices or add a countdown. Circular/
field/graph alternatives remain exploration. M2 budgets/scheduling, target
RAM/safe stack, Sharp output and RWX remain open; keep [0023's frozen evidence](decisions/0023-frozen-m2-revision-isolation.md) isolated.

## Verification / limits
Docs/PNG-only session: headless source snapshots were generated for the
comparison; retained PNG is 816×816, 1-bit, with local link and whitespace
checks. No Rust check/tests were rerun because no code/build inputs changed.
No interactive replay, physical-display/target validation, hardware, firmware
or frozen-M2 work. Historical 0049 checks/acceptance were not freshly rerun.
