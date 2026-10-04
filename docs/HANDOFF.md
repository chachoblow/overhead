# Overhead — Handoff

_Last updated: 2026-10-04. Status: M1 in progress; observer measurements implemented._

## Current state

- Engine-first roadmap remains agreed. docs/PLAN.md is the canonical task list;
  M1's first five tasks are complete, through observer-relative geometry.
  The reproducible headless runner is the remaining M1 task. No UI work yet.
- `Satellite::state_at(time)` returns TEME position/velocity using AFSPC
  propagation. Position-only `teme_to_ecef(position, time)`,
  `ecef_to_geodetic(position)`, and `GeodeticPosition::to_ecef()` are available.
- New API: `ecef_to_look_angles(target_km, observer)` returns `LookAngles`
  with positive `range_km`, `azimuth_rad: Option<f64>`, and `elevation_rad`.
  `ObservationError` wraps coordinate failures or reports coincident positions.
- Decision 0008 defines vertical singularities and longitude-defined polar
  axes. Existing AFSPC propagation, WGS-84, GMST-only rotation, UT1≈UTC, and
  no-polar-motion conventions are unchanged.
- Default core remains no_std/allocation-free; optional `omm` needs alloc.
  No Rust dependencies were added. Render remains a stub, tools' Rust binary
  remains hello-world, and sim remains static.

## What changed this session (by file)

- core/src/observer.rs — public look-angle types/errors and validated ECEF
  observer geometry; SEZ rotation, canonical azimuth, explicit singularities.
- core/src/lib.rs — exports the new API.
- core/tests/observer.rs — 11 tests covering independent references, full ISS
  pipeline, analytical geometry, poles, verticals, invalid values, and overflow.
- core/tests/fixtures/observer.json — 27 pymap3d geometry references and
  12 Skyfield pipeline references using the unchanged ISS OMM at T0/T1/T2.
- tools/generate_observer_fixtures.py — pinned, independent offline generator.
- core/tests/fixtures/README.md — provenance, versions, commands, tolerances,
  and differences between Overhead and Skyfield Earth-orientation models.
- docs/decisions/0008-observer-geometry-conventions.md — settled API/boundary
  and validation choices; no existing physical conventions reversed.
- docs/PLAN.md, docs/PROGRESS.md, docs/HANDOFF.md — task completion and handoff.

## Verification

All passed this session:
- `cargo check --workspace`
- `cargo test --workspace` — 38 tests total, including 11 new observer tests
- `cargo check -p overhead-core --no-default-features --lib`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all --check`
- Independent observer fixture regeneration matches byte-for-byte.
- `git diff --check`.

Skyfield pipeline maximum errors: 0.04110 km range, 0.004672° azimuth,
0.004897° elevation, within the agreed 0.1 km / 0.01° gates. Isolated
pymap3d geometry meets tighter 1e-8 km / 1e-9° gates.
Simulator was not launched; no hardware flashing or toolchain changes.

## Next concrete step

Add a reproducible headless host runner for fixture, explicit UTC timestamp,
and WGS-84 observer location inputs. Keep file I/O and reporting outside core;
use the optional `omm` feature. Compose ingestion → propagation → TEME/ECEF →
geodetic and observer measurements. Print satellite identity and physical
measurements with explicit units, and handle errors/undefined azimuth clearly.
Exercise the checked-in ISS at documented T0/T1/T2 and known locations, with
complete-pipeline checks and documented reproducible commands. This completes
M1; no UI work or network fetching is needed.

## Open questions / deferred choices

- Confirm available ESP32 benchmark hardware and final S3 operating budget.
  Ask before flashing or changing toolchains.
- Catalogue limits, prediction workload, and cadence await M2 measurement.
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
- Minimal Sharp refresh validation stays early; full firmware is M6.
  Target 20 Hz and handle the scarce panel/ribbon gently.
- Cached elements do not provide accurate time after a cold boot without Wi-Fi.

## Gotchas worth remembering

- Pass the propagation timestamp, not the element epoch, to `teme_to_ecef`.
  It supports 1957–2100, rejects explicit leap seconds, and converts positions
  only: velocity conversion needs an additional Earth-rotation term.
- Engine units are km/radians; observer height is WGS-84 ellipsoidal, not MSL.
  Negative elevations are valid geometry, not a visibility decision.
- Azimuth is `None` when horizontal/slant range <= 1e-12 (zenith/nadir);
  exactly coincident positions are errors. At a pole the supplied longitude
  defines the local compass basis. Inverse geodetic longitude on the exact
  polar axis is still conventionally zero, per 0007.
- Fixture tests need no Python/network. Regenerate only via pinned independent
  tools; Skyfield's bundled DUT1 estimate intentionally differs from UT1≈UTC.
- Crates use overhead-* names to avoid the Rust core library name collision.
- SDL2 on Apple Silicon needs -L /opt/homebrew/lib (.cargo/config.toml).
- Keep host I/O out of core/render and presentation state out of physical math.
