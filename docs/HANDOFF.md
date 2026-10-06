# Overhead — Handoff

_Last updated: 2026-10-05. Status: M1 complete; core API hardening before M2 is next._

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
  needs alloc. Tools depend on core with `omm` and serde_json.
- User agreed to a small API hardening follow-up before M2 after the codebase
  walkthrough. Exact API choices remain open; no code changes were made.
- Render remains a stub, sim remains static, firmware does not exist.
  The original `overhead-tools` binary remains hello-world; explicitly
  select `--bin overhead-track` for the new runner.

## What changed this session (by file)

- `docs/PLAN.md` — added a bounded pre-M2 hardening task: time validation,
  timestamp-bound conversion, and evaluation of frame-specific position types.
- `docs/HANDOFF.md` — recorded review findings and the new next-session priority.
- `docs/PROGRESS.md` — appended the walkthrough and verification record.
- No implementation or dependency changes. No new API decision is settled yet.
  A formatting-only change to `core/tests/fixtures/iss-25544.json` was verified
  to preserve JSON contents, then reverted at the user's request.

## Verification

All passed this session:
- `cargo check --workspace`
- `cargo test --workspace` — 45 tests total; no new tests this session
- `cargo check -p overhead-core --no-default-features --lib`
- Manual CLI run at T0 / named Vallado site; all 12 reference pipelines
  also exercised twice by executable tests for output repeatability.

CLI Skyfield comparisons retain the 0.1 km / 0.01° gates. The ISS fixture's
original bytes are restored; no fixture regeneration was performed. All-feature
tests, Clippy, and formatting were not rerun this session. Simulator was not
launched; no flashing or toolchain changes.

## Try it

From the workspace root:

```sh
cargo run -p overhead-tools --bin overhead-track -- \
  core/tests/fixtures/iss-25544.json 2026-10-04T12:43:41.833056Z \
  39.007 -104.883 2.187
```

See `tools/README.md` for all T0/T1/T2 commands and input/output conventions.

## Next concrete step

Complete the **Pre-M2 core API hardening** follow-up in `docs/PLAN.md` first:

1. Clarify and consistently enforce the time contract. Ingestion checks epoch
   years but does not explicitly reject leap-second epochs; `state_at()` does
   not check requested years or explicit leap seconds. CLI requested-time
   parsing and `teme_to_ecef()` do both. Add boundary tests and decide the
   intended contract before changing behavior.
2. Bind propagated state to its absolute timestamp and make the normal ECEF
   conversion use that timestamp automatically. Today callers can silently
   pair the right TEME position with the wrong rotation time.
3. Consider distinct TEME/ECEF position types: both currently use `[f64; 3]`,
   so passing an unrotated TEME position to observer geometry compiles.
   Choose a modest API, not a generic units framework.

These are API misuse risks and a validation inconsistency, not demonstrated
incorrect CLI calculations. Public geodetic fields are validated at conversion;
changing their construction is not a required part of this cleanup. Preserve
physical models, no_std/allocation-free defaults, and reference tolerances.
No geometry rewrite or new dependencies are planned; settle exact API choices
next session and record any architectural decision then.

After hardening, refine M2's first task: explicit catalogue group configuration,
local datasets, and merge/deduplication by NORAD ID. Settle conflicting/invalid
record handling before implementing. Keep live fetching in M6 and real-data UI
in M3; do not expand the cleanup into catalogue, prediction, or presentation work.

Confirm available ESP32 hardware for the early propagation benchmark and
minimal Sharp refresh check. Ask before flashing or changing toolchains.
Catalogue capacity, prediction workload, and update cadence remain
provisional until measured; do not choose them from host timings alone.

## Open questions / deferred choices

- Exact time-validation contract, timestamp-bound conversion API, and scope of
  frame-specific position types for the pre-M2 hardening follow-up.
- Available benchmark hardware and final S3 operating budget.
- M2 catalogue limits, pass event semantics, prediction accuracy/cost.
- Projection/zoom semantics and presentation policies await M3+ feedback.
- Real location source (0004), Starlink layer (0003), and RTC (0002) deferred.

## Known broken / risks

- Walkthrough checks/tests passed. Core time-validation gaps and frame/time
  API misuse risks are described above. Embedded performance remains unmeasured; S3
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
