# Overhead — Handoff

_Last updated: 2026-10-05. Status: M1 complete; M2 is next._

## Current state

- All six M1 tasks are complete. `docs/PLAN.md` remains canonical; no M2,
  UI, or firmware implementation was started this session.
- `overhead-track` in the tools crate composes local OMM ingestion → AFSPC
  propagation → TEME/ECEF → WGS-84 geodetic and observer measurements.
  Inputs are explicit: one-object JSON array, UTC ending in Z, observer
  latitude/longitude in degrees and ellipsoidal height in km.
- Reports name/NORAD ID, epoch/requested time, signed elapsed minutes,
  TEME position/velocity, ECEF position, geodetic coordinates, range,
  azimuth, and elevation with units. Undefined azimuth is explicit;
  failures return exit code 1, stderr diagnostics, and no partial report.
- Core APIs and physical conventions (decisions 0006–0008) are unchanged.
  Core's standalone default build remains no_std/allocation-free; `omm`
  needs alloc. Tools now depend on core with `omm` and existing serde_json;
  no new external crate or version was introduced.
- Render remains a stub, sim remains static, firmware does not exist.
  The original `overhead-tools` binary remains hello-world; explicitly
  select `--bin overhead-track` for the new runner.

## What changed this session (by file)

- `tools/src/bin/overhead-track.rs` — host-only CLI, validation, pipeline,
  unit-labelled reporting, and undefined-azimuth formatting test.
- `tools/tests/track.rs` — six executable integration tests: all 12 Skyfield
  ISS references, deterministic output, geodetic report consistency,
  arguments/time/location validation, input-file/OMM/element errors,
  boundary sites, pre-epoch time, and propagation failure.
- `tools/Cargo.toml`, `Cargo.lock` — host dependencies, no new packages.
- `tools/README.md` — usage, scope, reproducible T0/T1/T2 commands at four
  reference sites, error contract, and verification instructions.
- `README.md` — project description and documentation links.
- `docs/PLAN.md`, `docs/PROGRESS.md`, `docs/HANDOFF.md` — M1 completion.
  No new physical or architectural decision needed beyond existing plans.

## Verification

All passed this session:
- `cargo check --workspace`
- `cargo test --workspace` — 45 tests total (seven new tests)
- `cargo check -p overhead-core --no-default-features --lib`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all --check`
- `git diff --check`
- Manual CLI run at T0 / named Vallado site; all 12 reference pipelines
  also exercised twice by executable tests for output repeatability.

CLI Skyfield comparisons retain the 0.1 km / 0.01° gates. Existing fixture
bytes and independent generators are unchanged; no regeneration was needed.
Simulator was not launched; no flashing or toolchain changes.

## Try it

From the workspace root:

```sh
cargo run -p overhead-tools --bin overhead-track -- \
  core/tests/fixtures/iss-25544.json 2026-10-04T12:43:41.833056Z \
  39.007 -104.883 2.187
```

See `tools/README.md` for all T0/T1/T2 commands and input/output conventions.

## Next concrete step

Refine M2's first task into session-sized work: explicit catalogue group
configuration, local datasets, and merge/deduplication by NORAD ID. Settle
conflicting/invalid record handling before implementing. Keep live network
fetching in M6 and real-data UI in M3; do not silently expand the runner
into pass prediction or presentation work.

Confirm available ESP32 hardware for the early propagation benchmark and
minimal Sharp refresh check. Ask before flashing or changing toolchains.
Catalogue capacity, prediction workload, and update cadence remain
provisional until measured; do not choose them from host timings alone.

## Open questions / deferred choices

- Available benchmark hardware and final S3 operating budget.
- M2 catalogue limits, pass event semantics, prediction accuracy/cost.
- Projection/zoom semantics and presentation policies await M3+ feedback.
- Real location source (0004), Starlink layer (0003), and RTC (0002) deferred.

## Known broken / risks

- No failing checks/tests. Embedded performance remains unmeasured; S3
  double-precision propagation/prediction cost is the main compute risk.
- sgp4 2.4's simplified calendar helper mishandles dates after February 2100.
  Coordinate rotation avoids it using Chrono elapsed-time arithmetic;
  upstream propagation element-epoch handling remains unchanged.
- Inverse geodetic conversion targets terrestrial/satellite positions, not
  ambiguous deep-interior normal coordinates; non-convergence is an error.
- Runner has no freshness policy: the fixture is for reproducible historical
  measurements, not live tracking. Printed precision is not orbit accuracy.
- Minimal Sharp refresh validation stays early; full firmware is M6.
  Target 20 Hz and handle the scarce panel/ribbon gently.
- Cached elements do not provide accurate time after cold boot without Wi-Fi.

## Gotchas worth remembering

- Pass propagation time, not element epoch, to `teme_to_ecef`. It supports
  1957–2100, rejects explicit leap seconds, and converts positions only;
  Earth-fixed velocity needs an additional Earth-rotation term.
- Engine units are km/radians; CLI angles are degrees. Observer height is
  WGS-84 ellipsoidal, not MSL. Negative elevations are valid geometry.
- Azimuth is `None` for horizontal/slant range <= 1e-12 (zenith/nadir);
  coincident positions are errors. At poles, supplied longitude defines
  the local compass basis. Inverse longitude on the exact polar axis is zero.
- Fixture tests need no Python/network. Regenerate only with pinned
  independent tools; Skyfield's bundled DUT1 differs intentionally from UT1≈UTC.
- Chrono's general `.format()` needs its alloc feature; the host runner uses
  date/time Display instead, without changing Chrono feature configuration.
- Crates use overhead-* names to avoid the Rust core library name collision.
  SDL2 on Apple Silicon needs -L /opt/homebrew/lib (.cargo/config.toml).
- Keep host I/O out of core/render and presentation state out of physical math.
