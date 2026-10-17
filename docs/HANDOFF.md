# Overhead — Handoff

_Last updated: 2026-10-17 UTC. Next: scope a selection-change transition contract._

## Current state / agreed direction
[0045](decisions/0045-static-group-category-contract.md) is implemented and user
visually accepted, completing the five static fields in opt-in wide view. Category
uses explicit group roles with **Station > Navigation > Weather > Unknown**;
per-ID mapping and conflict-to-Unknown were rejected proposals. Executable review
was separate from approval of the icons/lifecycle. 0040–0044 notices/data fields
remain accepted and unchanged. Square diagnostics remain default; counts,
selection and ID hysteresis are unchanged. No selection clears all five fields,
retaining the separator. Transitions and final normal/debug access remain open.

## Session changes
`app/src/{category,lib}.rs` resolves retained valid memberships, including older
contributors. `tools/src/catalogue.rs` parses optional group roles; `sim/src/main.rs`
passes them through. `render/src/{category,lib}.rs` draws approved fixed masks.
Extended loader/app/render/CLI tests; example manifests declare Weather/Navigation
and ISS-only Station, while the mixed demo stays uncategorized. One local
tools→app dependency/lockfile edge, no new external package. Core propagation,
orbital fixtures, firmware/toolchains and frozen evidence are unchanged.
[DESIGN](DESIGN.md#static-category-contract) owns retained preview provenance;
[usage](../sim/README.md#static-category-glyph) reproduces fresh review frames.

## Next step / boundaries
Recommend a **design-only selection-change transition contract**: settle which
field(s) animate, duration, interruption and selection-loss behavior before code.
Preserve true physical readouts and the accepted static baseline; animation is
agreed direction, not an implementation contract. No new selection/collision
policy, production catalogue, prediction, fetching, scheduling or hardware work.

## Verification / risks
Wrap-up freshly passed serial workspace check/tests, fmt, app/render no-default-
feature checks and strict all-target app/render/sim/tools Clippy. The documented
category recipe reran headlessly; exact preview parity, retained PNGs, links/anchors
and whitespace checked. Earlier 13-case comparisons preserved square bytes,
wide pixels outside Cat and diagnostic reports (0045); faults are synthetic.
No interactive/display/target validation or 20 Hz budget claim. M2 budgets/scheduling,
RAM/safe stack, Sharp bring-up and RWX remain open; preserve [0023's frozen revision](decisions/0023-frozen-m2-revision-isolation.md).
