# Overhead — Handoff

_Last updated: 2026-10-04. Status: roadmap agreed; M1 current, not started._

## Current state

- User agreed an engine-first roadmap: verified calculations → catalogue/pass
  prediction/operating budget → real-data radar → useful display → refined
  motion → standalone device. docs/PLAN.md is the canonical task list.
- Decision 0005 records why calculations precede substantial UI work and how
  tuning configuration differs from presentation-specific policies.
- Decisions 0001–0004 remain unchanged: on-device SGP4, Wi-Fi/SNTP, curated
  catalogue, configured location.
- Code is unchanged: core/render are stubs, tools is hello-world, and sim
  draws one static frame via show_static. No frame loop, State, or draw API.
- Prior handoff reports workspace check/tests and simulator launch passing.
  They were not rerun in this documentation-only session.

## What changed this session (by file)

- docs/PLAN.md — M0 complete; M1 current with small tasks; M2–M6 outcomes,
  early hardware checks, configuration boundaries, and deferred scope.
- docs/decisions/0005-engine-first-implementation.md — accepted engine-first
  approach and its rationale/caveats.
- docs/DESIGN.md — clarified product focus versus implementation order.
- .pi/qrspi/implementation-roadmap/context.md — brief planning context; points
  to PLAN.md rather than duplicating task status.
- docs/HANDOFF.md and docs/PROGRESS.md — recorded the agreed roadmap.

## Next concrete step

Start M1's research task: check current SGP4/OMM APIs and no_std compatibility,
propose dependencies with rationale, and identify independent reference cases.
Establish time, coordinate-frame, unit, and tolerance conventions. Do not jump
straight into UI work or assume parser placement before checking compatibility.

## Open questions / deferred choices

- Dependency/API choices, parser boundary, and reference cases are M1 work.
- Confirm which ESP32 board is available for the first benchmark; confirm final
  budget on the S3. Ask before flashing or changing toolchains.
- Exact groups, catalogue size, prediction parameters, and cadence are tunable;
  workload limits must be based on measurements, including prediction cost.
- Projection/zoom semantics and presentation policies get UI feedback in M3+.
- Real location source (0004), Starlink layer (0003), and RTC (0002) stay deferred.

## Known broken / risks

- No newly identified breakage; no code changed this session.
- Main compute risk: SGP4 and future-pass searches on the S3, whose doubles are
  emulated. Begin the propagation benchmark as soon as that path works.
- Minimal Sharp display/refresh validation stays early; full firmware is M6.
  Target 20 Hz and handle the scarce panel/ribbon gently.
- Cached elements do not provide accurate time after a cold boot without Wi-Fi.

## Gotchas worth remembering

- Crates use overhead-* names to avoid the Rust core library name collision.
- SDL2 on Apple Silicon needs -L /opt/homebrew/lib (.cargo/config.toml).
- Keep host I/O out of no_std core/render; explain new dependencies first.
- Keep true physical state separate from visually slowed marker state.
