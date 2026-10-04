# Overhead

Handheld satellite tracker: a realtime radar-style view of satellites passing
over a set location, running on an ESP32-S3 with a Sharp memory LCD.
Rust Cargo workspace.

## Start here
1. docs/HANDOFF.md — where the last session left off
2. docs/PLAN.md — the current milestone; stay inside it
3. docs/DESIGN.md — product and interface intent

docs/decisions/ holds settled choices. Don't reverse one silently; propose a
new decision instead. docs/PROGRESS.md is history; read it only when you need
past context.

## Hardware
- ESP32-S3-DevKitC-1-N8R8 (8 MB flash, 8 MB PSRAM)
- Adafruit 2.7" Sharp Memory Display breakout (#4694): 400×240, 1-bit

## Workspace
- `core/` → `overhead-core` — `no_std`; orbits, projection, app state. Currently a stub.
- `render/` → `overhead-render` — `no_std`; draws into any embedded-graphics
  `DrawTarget<BinaryColor>`. Depends on core. Currently a stub.
- `sim/` → `overhead-sim` — macOS simulator: SDL2 window via
  embedded-graphics-simulator, 400×240 at 2× (LcdWhite theme). Depends on render.
- `tools/` → `overhead-tools` — offline data conversion (coastlines, later).
  Currently hello-world.
- `firmware/` — does not exist yet; excluded from the workspace; needs the
  Espressif toolchain.

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
- End of session: follow the steps in .pi/prompts/handoff.md.
