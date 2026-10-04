# Overhead — Handoff

_Last updated: 2026-10-04. Status: M0 complete. Next session: plan M1._

## Current state

- Docs are the source of truth: DESIGN.md (intent, no open questions),
  PLAN.md (M0 done, M1 TBD), decisions/0001–0004 (SGP4 on-device, Wi-Fi
  data + SNTP time, curated catalogue, hardcoded location).
- Code is minimal scaffolding: `core` and `render` are cargo-new stubs,
  `tools` is hello-world, `sim` draws one static frame (radar circle +
  label) via `show_static`. No frame loop, no `State`, no `draw()` exists.
- `cargo check --workspace` and `cargo test --workspace` pass.
  `cargo run -p overhead-sim` opens the window.
- Repo hygiene fixed: .gitignore added, target/ and stale per-crate
  Cargo.locks untracked, docs/HANDOFF.md case normalized.

## What changed this session (by file)

- AGENTS.md — crate list and verified commands filled in.
- docs/DESIGN.md — all six open questions resolved; readout panel and
  pre-knob zoom recorded as intent.
- docs/decisions/0001–0004 — new.
- docs/PLAN.md — M0 checked off.
- docs/HANDOFF.md — corrected false claims (a frame loop was described
  but never existed), then rewritten as this file.
- .gitignore — new; ~2100 tracked build artifacts removed.

## Next concrete step

Plan M1 in PLAN.md. Likely order (from earlier exploration): SGP4
benchmark on the old ESP32 → sim frame loop + zoom input → projection →
map data → real satellites → slow-down → polish → firmware bring-up.

## Open questions

None blocking. Deferred by decision: real location source (0004),
Starlink layer (0003), RTC (0002). Catalogue group selection is a tuning
question once the benchmark lands.

## Known broken / risks

- Nothing broken.
- Main risk: SGP4 speed on the S3 (single-precision FPU; doubles are
  emulated). The benchmark sizes the catalogue — do it early in M1.
- Sharp panel: design for 20 Hz refresh; the display is the scarce part
  (restock 2027) — handle the ribbon gently.

## Gotchas worth remembering

- Crates can't be named `core` (clashes with Rust's core lib); packages
  are prefixed `overhead-*`.
- SDL2 on Apple Silicon needs `-L /opt/homebrew/lib` (set in
  .cargo/config.toml).
- no_std: `libm` for float math, `heapless::String` for text.
