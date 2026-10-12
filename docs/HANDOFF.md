# Overhead — Handoff

_Last updated: 2026-10-12 UTC. Next session: review M3 radar/information-area proportions._

## Current state / user direction
The perspective/lens-zoom slice replaces the ground map under
[0026](decisions/0026-fixed-camera-perspective-slice.md), with
[0027's 75,000 km fixed camera](decisions/0027-higher-fixed-camera-overview.md).
The user prefers this higher view, likes zoom-magnified screen motion and the
fuller overview, and explicitly likes the empty outer band as framing.
Keep that breathing room; do not auto-fit or add objects merely to fill it.
Physical tracking and true horizon eligibility remain unchanged.
The user deferred radar/panel review to the next session; no more UI work now.

## Implementation / next session
`app/src/projection.rs` shares the fixed camera between ECEF targets and the
WGS-84 limb; `app/src/lib.rs` caches presentation apart from physical state.
`render/src/lib.rs` draws the outline/readouts; `sim/src/config.rs` uses FOV,
not the old map-radius semantics. App/render/CLI regressions cover the geometry.
Read PLAN/DESIGN; [sim usage](../sim/README.md) has current run/snapshot commands.
Next is radar/panel proportions and which readouts earn space; no production
population, cadence or panel-content decision has been made. Stay in M3.
At the pinned weather/GPS time/site 20°/100°/5° show 6/22/4 markers (SKY 22).
For a quick run use `cargo run -p overhead-sim -- --demo --fov-deg 100`;
the unchanged small demo has no markers at default 20° but four at 100°.
No slowdown, prediction integration or fetching work is included.

## Verification / prerequisites / risks
Fresh staged-source Git export, empty target: offline locked workspace check,
default/all-feature tests, core/app/render no-default checks, strict Clippy/fmt
and simulator build pass. Snapshot examples reproduce expected counts from an
unrelated directory with SDL unavailable; two-second SDL dummy-loop smoke passes.
Existing Rust tool/dependency caches and Homebrew SDL2 are required; no ignored
build artifacts are needed. User visual feedback is received; no agent interactive
input review, hardware access, firmware build or toolchain change was performed.
M2 limits/cadences/overload/scheduling and whole-device RAM/safe stack remain open;
Sharp bring-up and RWX are separate. Frozen evidence uses
[0023's pinned revision](decisions/0023-frozen-m2-revision-isolation.md), not this
workspace; no frozen evidence was renewed. Host throughput is not target capacity.
