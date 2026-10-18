# 0040 — White-backed bottom-left notice contract

Date: 2026-10-17
Status: accepted (bounded M3 contract; implemented and user visually accepted; width later amended by 0048)

## Decision
Implement [0039's bottom-left condition-driven notices](0039-bottom-left-condition-driven-notices.md)
in the existing opt-in wide renderer, without waiting for the revised glyph
stack. The user approved a small borderless white backing drawn over crossing
scene graphics and authorized the notice-only implementation slice. Keep the
current elevation capsule, glyph, square diagnostics and all app/physics behavior.

Subsequently, [0041](0041-circular-elevation-only-contract.md) replaces the
capsule/filled elevation with a circular cutout, label and separator. That
separate slice leaves this accepted notice contract unchanged; references to
preserving 0037 below describe the original notice-only implementation scope.
[0048](0048-refined-empty-view-notices.md) subsequently accepts longer empty-view
statuses and a wider backing for future implementation; the raster bounds below
describe the **current executable**, not approved-but-unimplemented 0048.

## Why
Transparent corner text can become unreadable when scene graphics cross it.
A small opaque backing guarantees clean notice pixels without a collision
search, moving objects or resizing the radar. The approved tradeoff is temporary
local visual occlusion, including the selected marker, while notices apply.
Implementing notices separately avoids inventing the unresolved glyph semantics.

## Consequences

### Geometry and compositing
Coordinates are zero-based in the existing 400×240 image.

- Use the existing 6×10 bitmap font, black on white, at `x=12`. Each complete
  notice occupies one line. Stack only active lines, in order: `NO FRAME` or
  `NO MARKERS`, then `TRACK ERROR`, then `INGEST WARN`.
- Bottom-anchor the last text baseline at `y=222`, with 13px baseline spacing.
  One, two and three lines start at baseline 222, 209 and 196 respectively.
  Uppercase ink therefore matches the reviewed mock's last ink row at 222.
- Before drawing text, clear one rectangular block covering the active text's
  character cells plus 2px padding on every side. Use the longest active line
  for width, not the entire corner. No border, icon, shadow or decorative frame.
  The 6×10 cell top is 7px above its baseline; the backing is thus at `x=10`,
  from `y=213 - 13*(line_count-1)` through `y=226`, inclusive. Width is
  `6*longest_line_length + 4`: at most 70px (`x=10..79`), with height at most
  40px (`y=187..226`).
- Draw this overlay last. Within its bounds it may visually cover Earth,
  markers (including the selected diamond), marker IDs and arrows. This is an
  explicit local compositing exception to the unobscured/transparent behavior
  of earlier wide-view decisions, not a change to object eligibility or geometry.
  Do not relocate, skip or reclassify objects, modify selection, or shrink the
  viewport. Host/app counts describe eligible markers/arrows, not unoccluded ink.
- No active notices means neither text nor backing. Every frame already clears
  and redraws the scene, so resolved notices restore underlying graphics; a
  smaller active block must not leave a stale larger mask. No reserved empty box.
- Remove notices from the right-hand strip. Keep 0037's capsule/elevation exactly
  as implemented, including clearing both without selection. No separator or
  revised glyph is added in this slice.

### State and scope
0039 owns notice conditions: viewport emptiness is not complete empty sky;
tracking and ingestion warnings remain explicit alongside successful markers;
no timeout, acknowledgement or warning history. Catalogue-load warnings remain
latched for the current simulator run, which has no reload. `NO FRAME` remains
a renderer-supported state; the simulator still exits on invalid-time errors.

No dependencies, new CLI controls, glyph mappings, animation, selection policy,
projection, prediction, fetching, production policy, hardware or frozen-M2
changes. The accepted full-stack direction remains a separate unimplemented slice.

## Verification
- Renderer pixel tests cover exact text/backing bounds, one/two/three-line layout,
  opaque padding and gaps over crossing graphics, and no-op healthy composition.
- Lifecycle tests cover valid markers plus both warnings, empty/missing frames,
  transitions to fewer/no notices, recovery, unchanged samples/counts/selection,
  unchanged strip pixels and scene pixels outside the backing.
- CLI snapshots check explicit failures and omitted ingestion records with
  successful markers, using synthetic failure controls labelled as such. Retain
  independent real-data geometry/elevation and square-default regressions.
- Run `cargo check --workspace` and app/render/simulator tests serially. Compare
  pre/post healthy wide and square snapshots and empty-view changes; visually
  review headless outputs separately from passing automated checks.

Implementation belongs in `render/src/lib.rs` with CLI coverage in
`sim/tests/snapshot.rs`; [simulator usage](../../sim/README.md#wide-radar-comparison)
owns current behavior and [notice review reproduction](../../sim/README.md#notice-overlay-review). No physical-display legibility
or target-performance claim follows from host snapshots.

Implementation verification passed: `cargo check --workspace` and
`cargo test -p overhead-app -p overhead-render -p overhead-sim` (21 app, 22 renderer,
7 simulator unit and 8 CLI tests), run serially. Headless healthy square and wide
20°/100° snapshots are byte-identical to the pre-change executable. At 0.1°,
radar differences are confined to the new notice backing; the old strip notices
are removed. Actual selected-plus-warnings, empty-plus-warnings and an ISS
marker/ID crossing the backing were inspected. Fault/omission inputs in those
warning examples are explicit synthetic controls. User visual acceptance of the
executable output was still pending at that implementation handoff. Wrap-up
additionally reran workspace check/tests, formatting, app/render no-default-feature
checks and strict all-target app/render/sim Clippy successfully. The fresh-checkout README recipe
was executed headlessly and reproduced all four warning/crossing examples
byte-for-byte.

The user subsequently confirmed that they reviewed the executable snapshots in
the preceding session and that they look good. This closes the executable notice
visual-review gate; it does not approve the revised glyph stack or establish
physical-display legibility or target performance. No new implementation or
check/test run accompanied that confirmation.
