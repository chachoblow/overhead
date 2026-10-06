# Overhead

Rust workspace for a handheld satellite radar. Hardware and interface intent
live in docs/DESIGN.md.

## Start here
1. docs/HANDOFF.md — where the last session left off
2. docs/PLAN.md — the current milestone; stay inside it
3. docs/DESIGN.md — product and interface intent

docs/decisions/ holds settled choices. Don't reverse one silently; propose a
new decision instead. docs/PROGRESS.md is history; read it only when you need
past context.

## Workspace
- `core/` → `overhead-core`: ingestion, propagation, coordinates, observer geometry.
  Default no_std/allocation-free; optional `omm` parsing requires alloc, not std.
- `render/` → `overhead-render`: no_std, embedded-graphics `BinaryColor`; stub.
- `sim/` → `overhead-sim`: static SDL2 simulator, 400×240 at 2×, LcdWhite theme.
- `tools/` → `overhead-tools`: headless runner and offline fixture generators;
  usage in tools/README.md. Default binary is still a placeholder.
- `firmware/`: not created; excluded from workspace; needs Espressif toolchain.

## Commands
- Check: `cargo check --workspace`
- Test: `cargo test --workspace`
- Simulator: `cargo run -p overhead-sim` (opens an SDL2 window; needs Homebrew
  SDL2, linker path set in .cargo/config.toml)
- Firmware build: none yet (firmware crate not created)

Never flash hardware or change the toolchain without asking first.

## Rules
- Rendering and orbit logic live in platform-agnostic crates; the simulator
  and firmware stay thin.
- The display is 1-bit. No grayscale or color assumptions.
- After any code change, run the check command and fix errors before calling
  the work done.
- Explain why before adding a dependency.
- Use web search for crate APIs and datasheets instead of guessing; esp-hal
  and related crates change quickly.
- Keep each doc focused: HANDOFF = next-session context, PLAN = scope/status,
  DESIGN = product intent, decisions = rationale/contracts, fixture README =
  provenance/regeneration. Link rather than repeat; code/tests own API details.
- End of session: follow the steps in .pi/prompts/handoff.md.
