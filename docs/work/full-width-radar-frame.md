# Full-width radar frame (0089, without trails and orbit)
Status: active
Started: 2026-11-22. Scope: PLAN M8 item 1; decision [0089](../decisions/0089-full-width-radar-revision.md)

## Outcome
The wide view (simulator `--wide` and the device app) draws 0089's frame on
`main`, properly built and tested: 400 px radar, left pills, octagon markers
and below-horizon hollow diamonds with cleared halos, selected octagon with
pulsing ring, sphere-shaded coastline, scale ruler, haloed arrows, 0048
notices. The strip, glyph, visibility-pictogram and ID-label code is gone.
Trails and the selected orbit are item 2.

## Done when
- `cargo check --workspace`, `cargo test --workspace` and clippy on the
  touched crates pass; firmware host tests pass; the device build compiles.
- Host tests cover: below-horizon selection rule and Earth occlusion (app),
  pill contents/geometry, marker/halo pixels, ruler steps and labels, tone
  table shape and its bake into the coast raster, frame without time/data
  (notices), every pixel inside 400×240.
- Fresh simulator snapshots (selected, no selection, wide zoom, tight zoom,
  error notice) reviewed by the user.
- Board image flashed via `firmware/mac.sh`; the user accepts it on the panel.

## Boundaries
- No trails or orbit line (item 2); no new propagation.
- Square/circle comparison viewports and the debug page unchanged.
- Frozen M2 evidence untouched; historical firmware diagnostics that only
  measured the old strip (glyph traces, period cache) are retired, pointing
  at the last pre-0089 revision, rather than kept compiling.
- No toolchain change; no release (release 16 waits for item 2).

## Human gates
- Scope, old design, release, pills: asked and answered 2026-11-22 (new M8
  now; replace; release 16 waits and carries 0088; pills as on the board).
- Snapshot review and panel acceptance: at the end.

## Plan
- [x] App: `Camera::hidden_by_earth` (ellipsoid), `RadarApp::below_horizon_markers`;
      0035 label zoom (`identification`) removed.
- [x] Render: `Layout::wide()` 400×240; coast raster width from `WIDE_WIDTH`;
      shading baked into the coast raster per lens (`Mask::shade`).
- [x] Render: `render/src/wide.rs` (pills, markers, halos, selected ring,
      arrows, ruler, shading); glyphs/visibility/presentation/label removed.
- [x] Callers: sim (plain `draw`, `sim/examples/visual.json` added), firmware
      `draw_cached` → `draw`; `sharp-profile` and `sharp-cache` retired.
- [x] Tests rewritten/added; clippy clean.
- [ ] Snapshots; firmware dev image; flash; user review.
- [ ] Docs: DESIGN, sim README, firmware README, status notes on superseded
      decisions (0032, 0034, 0035, 0038, 0046, 0050, 0061, 0078).

## Discoveries and assumptions
- Board-test reference: branch `radar-fav-c-board-test` (`4901556`),
  `render/src/fav_c.rs`.
- Sphere shading depends only on lens and layout, so it is applied when the
  coast raster (0059) is built: zero per-frame cost.
- Pulse phase comes from the frame's app time, as on the board.

## Verification
- 2026-11-22: `cargo check --workspace` ok; `cargo test --workspace` 331
  passed (strip/label tests removed); render with `representation-measurement`
  79 passed; clippy clean (app, render, sim, all targets); firmware host tests
  38 passed; `cargo check --release` of device+dev-settings, sharp-lines,
  sharp-natural, sharp-replay, live-utc-radar, live-utc-fresh-iss,
  coast-prepared, coast-representation images ok.

## Open questions
