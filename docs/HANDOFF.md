# Overhead — Handoff

_Last updated: 2026-10-18 UTC. Next: seek separate authorization for M4 notice implementation._

## State
[M3 complete](PLAN.md#m3--minimal-real-data-radar-complete): accepted moving,
zoomable real-data radar, opt-in wide view/five fields/transitions and debug;
square diagnostics remain default. [Simulator usage](../sim/README.md) owns
executable reproduction. M4's first **agreed, unimplemented** contract is
[0048's wide empty-view refinement](decisions/0048-refined-empty-view-notices.md):
`SKY OUTSIDE VIEW` for arrows, `NONE ABOVE HORIZON` only for clean nonempty
loaded catalogues with none above, otherwise conservative `NO MARKERS`.
[Native-size comparisons](mockups/0048-empty-view-proposal.md) are illustrative,
not executable output. Current wide replay still says `NO MARKERS` in both cases.

## This session
- From clean `main` at `ac31fc7`, reviewed 0033/0039/0040, app/render code
  and simulator tests. User chose M4 ahead of M2 and approved the contract.
- Captured two headless historical wide snapshots: demo (16 accepted, 4 arrows,
  0 visible) and one ISS record (0 above horizon); retained the two binary
  comparison PNGs/provenance in `docs/mockups/0048-*`.
- Recorded `docs/decisions/0048-refined-empty-view-notices.md`; updated PLAN,
  DESIGN, 0039/0040 links and HANDOFF. No code/fixture/firmware/M2 changes.

## Next bounded step / limits
Ask for **separate implementation authorization**. If granted, update only
wide notice classification/backing in `render/src/lib.rs`, plus focused
pixel/lifecycle and `sim/tests/snapshot.rs` headless tests; preserve square,
selection, arrows, glyphs, physical truth and independent warnings. Review fresh
executable output separately; mocks do not authorize it. M2 budgets/scheduling,
RAM/safe stack, Sharp bring-up and RWX remain open; keep
[0023's frozen evidence](decisions/0023-frozen-m2-revision-isolation.md) isolated.
Circular-radar/field/graph alternatives remain unapproved exploration.

## Verification
This docs/images session ran two **headless** simulator snapshots and checked
links/anchors, image size/binary pixels, instruction consistency and whitespace.
No Rust checks/tests (no code change), SDL window or hardware access. Earlier
M3 checks in [PLAN](PLAN.md#m3--minimal-real-data-radar-complete) were on
`64db11a`, not new validation. No target/panel claim or executable acceptance.
