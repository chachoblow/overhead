# Overhead — Handoff

_Last updated: 2026-10-06. Status: M1 and pre-M2 hardening complete; refine M2 catalogue work next._

## Current state

- All six M1 tasks and all three pre-M2 hardening tasks are complete.
  `docs/PLAN.md` remains canonical. No catalogue, prediction, UI, or firmware
  implementation was started this session.
- The core physical pipeline is independently verified; `overhead-track`
  composes local OMM ingestion → AFSPC propagation → Earth-fixed/geodetic
  position → observer range/azimuth/elevation with explicit inputs.
- Shared UTC validation now covers element epochs, requested propagation
  times, low-level rotation, and CLI requested-time parsing: years 1957–2100
  inclusive, ordinary fractional seconds accepted, explicit leap seconds
  rejected. Signed naive UTC elapsed time does not insert leap seconds.
- Decision 0009 settles the API hardening. `TemeState` fields are private with
  read-only accessors and an absolute `datetime()`. Normal conversion is
  `state.to_ecef()`, which uses that timestamp automatically.
- Distinct `TemePosition` / `EcefPosition` wrappers prevent accidental frame
  substitution. Use `from_km([x, y, z])` for explicit synthetic input and
  `.km()` to extract raw components. Constructors label frames, not validity;
  existing geometry/conversion validation is preserved.
- Physical conventions in decisions 0006–0008 and all reference gates are
  unchanged. Default core remains no_std/allocation-free; `omm` requires alloc.
- Render remains a stub, sim remains static, firmware does not exist.
  The original `overhead-tools` binary remains hello-world; explicitly select
  `--bin overhead-track` for the useful runner.

## What changed this session (by file)

- `core/src/time.rs` — shared `validate_utc_time` and `TimeError`.
- `core/src/position.rs` — modest TEME/ECEF types and compile-fail frame tests.
- `core/src/ingest.rs` — shared time validation; added `LeapSecondEpoch` while
  preserving `ImplausibleEpoch(year)` diagnostics.
- `core/src/propagate.rs` — requested-time validation, read-only timestamp-bound
  state, `to_ecef()`, and compile-fail retiming test.
- `core/src/coordinates.rs`, `core/src/observer.rs`, `core/src/lib.rs` — typed
  positions at geometry boundaries, common time validation, public exports.
- `core/tests/time_contract.rs` — supported/rejected time boundaries, leap
  seconds, signed subsecond elapsed time, and bound-time rotation regressions.
- `core/tests/{coordinates,observer,propagate}.rs` — migrated to typed API;
  independent propagated pipelines now exercise `state.to_ecef()`.
- `tools/src/bin/overhead-track.rs`, `tools/tests/track.rs` — shared validator
  and bound conversion; added invalid element-epoch executable coverage.
- `docs/decisions/0009-core-time-and-frame-api.md`, `docs/PLAN.md`,
  `docs/PROGRESS.md`, `docs/HANDOFF.md` — decision and completion record.
- No dependency, fixture, physical-model, or tolerance changes.

## Verification

All passed:
- `cargo check --workspace`
- `cargo test --workspace` and `cargo test --workspace --all-features` —
  50 tests plus 3 compile-fail doctests (previously 45 tests)
- `cargo check -p overhead-core --no-default-features --lib`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all --check`
- `git diff --check`

The new boundary tests were added before behavior changes; they reproduced
propagation accepting a requested time just before 1957. All independent
reference gates remain intact, including 12 CLI Skyfield comparisons at
0.1 km / 0.01° exercised twice for repeatability. No fixture regeneration,
simulator launch, hardware flashing, or toolchain changes.

## Try it

From the workspace root:

```sh
cargo run -p overhead-tools --bin overhead-track -- \
  core/tests/fixtures/iss-25544.json 2026-10-04T12:43:41.833056Z \
  39.007 -104.883 2.187
```

See `tools/README.md` for T0/T1/T2 commands and report conventions. Successful
output format is unchanged; invalid inputs fail with stderr and no partial
report.

## Next concrete step

Refine **M2's first task** before implementation:

1. Define explicit catalogue group configuration using local datasets.
2. Settle merging/deduplication by NORAD ID, including conflicting records,
   invalid records, and provenance. Do not silently select a conflict policy.
3. Implement and test that bounded headless catalogue slice. Keep live
   fetching in M6 and real-data UI in M3.

Confirm available ESP32 hardware for an early propagation benchmark and a
minimal Sharp refresh check. Ask before flashing or changing toolchains.
Catalogue capacity, prediction workload, and update cadence remain provisional
until measured; host timings alone must not set the S3 budget.

## Open questions / deferred choices

- M2 catalogue conflict/invalid-record semantics and group configuration API.
- Available benchmark hardware and final S3 operating budget.
- Catalogue limits, pass event semantics, prediction accuracy/cost.
- Projection/zoom semantics and presentation policies await M3+ feedback.
- Real location source (0004), Starlink layer (0003), and RTC (0002) deferred.

## Known broken / risks

- No known failing checks. Embedded performance is unmeasured; S3
  double-precision propagation/prediction cost remains the main compute risk.
- sgp4 2.4's simplified epoch calendar helper mishandles dates after February
  2100. Rotation avoids it with Chrono elapsed-time arithmetic; upstream
  propagation element-epoch handling remains unchanged. Era acceptance is
  not a guarantee of propagation accuracy or fresh elements.
- Low-level `teme_to_ecef(position, time)` intentionally remains available for
  synthetic/reference inputs; that caller still owns timestamp correctness.
  Explicitly extracting and relabeling arrays can bypass frame protection.
- Inverse geodetic conversion targets terrestrial/satellite positions, not
  ambiguous deep-interior normal coordinates; non-convergence is an error.
- Runner has no freshness policy: fixtures are historical, not live tracking.
  Printed precision is not orbit accuracy.
- Minimal Sharp refresh validation stays early; full firmware is M6.
  Target 20 Hz and handle the scarce panel/ribbon gently.
- Cached elements do not provide accurate time after cold boot without Wi-Fi.

## Gotchas worth remembering

- Normal pipeline: `satellite.state_at(time)?` → `state.to_ecef()?` →
  `ecef_to_geodetic(ecef)` / `ecef_to_look_angles(ecef, observer)`.
  State field reads now use accessors; position components use `.km()`.
- Frame conversion is position-only. TEME velocity remains km/s; an
  Earth-fixed velocity needs the additional Earth-rotation term.
- Engine units are km/radians; CLI angles are degrees. Observer height is
  WGS-84 ellipsoidal, not MSL. Negative elevations are valid geometry.
- Azimuth is `None` for horizontal/slant range <= 1e-12 (zenith/nadir);
  coincident positions are errors. At poles, supplied longitude defines
  the local compass basis. Inverse longitude on the exact polar axis is zero.
- Fixtures need no Python/network. Regenerate only with pinned independent
  tools; Skyfield's bundled DUT1 differs intentionally from UT1≈UTC.
- Chrono's general `.format()` needs alloc; the runner uses date/time Display.
- Crates use overhead-* names to avoid the Rust core library name collision.
  SDL2 on Apple Silicon needs -L /opt/homebrew/lib (.cargo/config.toml).
- Keep host I/O out of core/render and presentation state out of physical math.
