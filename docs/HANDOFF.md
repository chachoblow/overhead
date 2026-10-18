# Overhead — Handoff

_Last updated: 2026-10-18 UTC. Next: review M3 exit criteria before selecting new work._

## State
[0047](decisions/0047-text-first-debug-access-contract.md) is implemented and the
user accepted its fresh executable debug output, separately from layout approval
and implementation authorization. Wide radar, five selected fields and
[0046's transitions](decisions/0046-selected-data-glyph-transition-contract.md)
remain accepted. Square/default, normal notices and static snapshots are unchanged.
[Simulator usage](../sim/README.md#debug-access) owns controls and reproduction;
retained mock pages remain illustrative, not executable evidence.

## This session
- `render/src/debug.rs`: independent no_std text-first page with bounded names/
  causes, explicit overflow guidance, true values and unsampled/absent states.
- `sim/src/config.rs`, `sim/src/main.rs`: wide-only `v` and `--wide --debug`;
  replay/Tab/zoom remain active, return discards hidden glyph motion, debug
  snapshots are rejected. App/core truth and complete host reports are unchanged.
- `render/src/debug_tests.rs`, `sim/src/interactive_tests.rs`,
  `sim/tests/snapshot.rs`: formatting, state/lifecycle, input and CLI regressions.
  Updated usage, 0047 and PLAN to record implementation and visual acceptance.

## Next bounded step
Review [M3's done-when criteria](PLAN.md#m3--minimal-real-data-radar-in-progress)
against accepted output and tests; recommend milestone closure or identify one
specific remaining gap before requesting another implementation slice. M3 has
not been marked complete and no follow-on implementation is authorized.
Circular-radar/field/graph alternatives remain unapproved exploration. M2 budgets/
scheduling, RAM/safe stack, Sharp bring-up and RWX remain open; keep
[0023's frozen evidence](decisions/0023-frozen-m2-revision-isolation.md) isolated.

## Verification / limits
Wrap-up freshly passed `cargo check --workspace`, `cargo test --workspace --quiet`,
`cargo check -p overhead-app -p overhead-render --no-default-features`,
`cargo clippy -p overhead-app -p overhead-render -p overhead-sim -p overhead-tools --all-targets -- -D warnings`,
and `cargo fmt --all --check`. Earlier pre/post weather/GPS square/wide-normal
PNGs and stderr matched at 0.1°, 20° and 100°. The agent launched no SDL window;
user visual acceptance is separate from input/presentation tests. No dependency,
fixture, firmware/toolchain or frozen-M2 changes; no display/S3 validation or 20 Hz claim.
