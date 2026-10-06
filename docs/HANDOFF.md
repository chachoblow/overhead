# Overhead — Handoff

_Last updated: 2026-10-06. Status: M1, pre-M2 hardening, and review fixes complete; refine M2 catalogue work next._

## Current state

- All six M1 tasks, three pre-M2 hardening tasks, and two review fixes are
  complete. `docs/PLAN.md` remains canonical. M2 implementation has not started.
- The verified pipeline is local OMM ingestion → AFSPC propagation → bound-time
  Earth-fixed/geodetic conversion → observer range/azimuth/elevation.
- `Satellite::state_at` now rejects NaN/infinite position or velocity with
  `PropagateError::NonFinite`, even if upstream SGP4 returns success.
- With the `omm` feature, deserialize JSON into `OmmElements` (or a vector of
  them), then pass `.elements()` to `Satellite::from_elements`. The adapter
  checks explicit Earth/TEME/UTC/SGP4 metadata; omission uses CelesTrak defaults.
  Nulls, non-strings, duplicates, and contradictory declarations are rejected.
  Unrelated extra fields remain allowed. `.into_elements()` transfers ownership.
- Numeric/epoch validation remains separate. Raw `sgp4::Elements` does not
  retain physical metadata and cannot detect contradictions after parsing.
- Shared UTC validation supports 1957–2100, ordinary fractional seconds, and
  signed naive UTC elapsed time; explicit leap seconds are rejected.
- `TemeState` is read-only and timestamp-bound; use `state.to_ecef()`.
  `TemePosition`/`EcefPosition` label frames; `.km()` extracts raw components.
- Default core remains no_std/allocation-free. OMM requires alloc, not std.
  Serde is now an optional direct dependency; no new library/version was added.
- Render remains a stub, sim remains static, firmware does not exist. Select
  `--bin overhead-track`; the original tools binary is still hello-world.

## What changed this session (by file)

- `core/src/propagate.rs` — finite-output guard and `PropagateError::NonFinite`.
- `core/src/omm.rs` — reusable `OmmElements` metadata-checking deserializer.
- `core/src/{lib,ingest}.rs` — feature-gated export and ingestion guidance.
- `core/Cargo.toml`, `Cargo.lock` — optional direct serde dependency under OMM.
- `core/tests/propagate.rs`, `core/tests/omm.rs` — overflow and metadata regressions.
- `tools/src/bin/overhead-track.rs`, `tools/tests/track.rs` — adapter migration,
  explicit metadata acceptance/rejection, and propagation-stage error coverage.
- `tools/README.md`, decision 0010, `docs/{PLAN,PROGRESS,HANDOFF}.md` — behavior,
  rationale, completion, and handoff record.
- No physical-model, tolerance, or fixture changes; no catalogue/UI/firmware work.

## Verification

All passed:
- `cargo check --workspace`
- `cargo test --workspace` and `cargo test --workspace --all-features` —
  58 tests plus 3 compile-fail doctests (previously 50 tests).
- `cargo check -p overhead-core --no-default-features --lib`
- `cargo check -p overhead-core --no-default-features --features omm --lib`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all --check` and `git diff --check`.

Before implementation, new tests reproduced successful NaN states for finite
extreme mean motion/drag, accepted contradictory metadata, and the CLI detecting
NaNs only at coordinate conversion. These now pass. All independent reference
gates remain unchanged, including 12 CLI Skyfield comparisons exercised twice.
No fixture regeneration, simulator launch, flashing, or toolchain changes.

## Try it

```sh
cargo run -p overhead-tools --bin overhead-track -- \
  core/tests/fixtures/iss-25544.json 2026-10-04T12:43:41.833056Z \
  39.007 -104.883 2.187
```

See `tools/README.md` for T0/T1/T2 commands and report conventions. Existing
successful output is unchanged; failures produce stderr and no partial report.

## Next concrete step

Refine **M2's first task** before implementation:
1. Define explicit catalogue group configuration using local datasets.
2. Settle NORAD-ID deduplication, conflicts, invalid records, and provenance.
   Do not silently choose a conflict policy; distinguish element epoch from
   fetch timestamp, and use the checked OMM adapter at the ingestion boundary.
3. Implement/test that bounded headless slice. Live fetching stays M6; UI stays M3.

Confirm available hardware for the early propagation benchmark and minimal
Sharp refresh check; ask before flashing or changing toolchains. Measure mixed
LEO/GNSS/GEO workloads and later prediction, not just one ISS propagation.
Catalogue capacity and cadence remain provisional until measured on the S3.

Review follow-ups for M2: broaden independent references to resonant/low-inclination
GEO, applicable half-day resonant objects, negative propagation times, and a
non-LEO observer pipeline. These are coverage recommendations, not demonstrated
normal-pipeline failures. Keep physical passes separate from zoom-dependent
screen traversal and visual slow-down.

## Open questions / known risks

- M2 catalogue conflict/invalid-record semantics and group configuration API.
- Hardware availability, S3 double-precision compute budget, catalogue limits,
  scheduling, and prediction event semantics/accuracy/cost remain unmeasured.
- No known failing checks. Finite outputs are not a physical-accuracy guarantee;
  runner freshness policy remains intentionally absent.
- sgp4 2.4's epoch calendar helper mishandles dates after February 2100.
  Rotation avoids it; upstream propagation epoch handling remains unchanged.
- Low-level `teme_to_ecef(position, time)` still permits caller timestamp misuse;
  explicit array relabeling can bypass frame labels. Use the bound-state path.
- Inverse geodetic conversion targets terrestrial/satellite positions, not
  ambiguous deep-interior coordinates; non-convergence is an error.
- OMM metadata checks cover the four named declarations, not full CCSDS schema
  validation or every upstream element field's semantics.
- Projection/zoom/presentation policies await M3+ feedback. Location source,
  Starlink/full-catalogue layer, and RTC remain deferred.
- Cached elements do not provide accurate time after cold boot without Wi-Fi.
- Minimal Sharp validation stays early; target 20 Hz and handle panel/ribbon gently.

## Gotchas

- Normal geometry: `satellite.state_at(time)?` → `state.to_ecef()?` →
  `ecef_to_geodetic(ecef)` / `ecef_to_look_angles(ecef, observer)`.
- Frame conversion is position-only; raw TEME velocity is km/s, not Earth-fixed.
- Engine units are km/radians; CLI angles are degrees. Heights are WGS-84
  ellipsoidal, not MSL. Negative elevations are valid geometry.
- Vertical azimuth is `None` for horizontal/slant range <= 1e-12; coincident
  positions are errors. Supplied polar longitude defines the local compass basis;
  inverse longitude on the exact polar axis is zero.
- Fixtures need no Python/network. Regenerate only with pinned independent tools;
  Skyfield's bundled DUT1 intentionally differs from UT1≈UTC.
- Chrono's general `.format()` needs alloc; the runner uses date/time Display.
- SDL2 on Apple Silicon needs the linker path in `.cargo/config.toml`.
- Keep host I/O out of core/render and presentation state out of physical math.
