# Overhead — Handoff

_Last updated: 2026-10-04. Status: M1 in progress; position transforms implemented._

## Current state

- Engine-first roadmap remains agreed. docs/PLAN.md is the canonical task list;
  M1's first four tasks are complete: research, ingestion/validation,
  explicit-time propagation, and Earth-fixed/geodetic position transforms.
- `overhead-core` validates elements into `Satellite`; `Satellite::state_at`
  returns TEME position/velocity. Propagation remains AFSPC-compatible (0006).
- New public geometry API: `teme_to_ecef(position_km, utc)`,
  `ecef_to_geodetic(position_km)`, and `GeodeticPosition::to_ecef()`.
  Angles are radians, lengths km. These convert positions only, not velocity.
- Decision 0007 clarifies TEME at propagation time (not a frame frozen at
  element epoch), IAU-1982 GMST rotation, WGS-84 boundaries/errors, and
  independent reference validation. UT1≈UTC and no polar motion remain.
- Default core remains no_std/allocation-free; optional `omm` needs alloc.
  `libm` is now direct as well as transitive, with no new runtime library.
- Observer-relative measurements and the headless runner remain in M1.
  Render remains a stub, tools' Rust binary is hello-world, and sim is static.

## What changed this session (by file)

- docs/notes/implementation-roadmap.md — moved historical roadmap context
  out of the retired workflow directory; marked it as historical and linked
  to PLAN and decision 0005 instead of retaining workflow instructions.
- docs/notes/earth-coordinates.md and
  docs/notes/earth-coordinates-verification.md — moved implementation context
  and completed checks into docs, with historical labels and cross-links.
- docs/HANDOFF.md, docs/PROGRESS.md — recorded the documentation relocation.
  Removed the empty workflow directories. No code, milestone, or decision
  changes; observer geometry remains next.

## Verification

All passed in the coordinate implementation session (commit `3db9435`):
- `cargo check --workspace`
- `cargo test --workspace` — 27 tests total (12 new coordinate tests)
- `cargo check -p overhead-core --no-default-features --lib`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all --check`
- Independent fixture regeneration matches byte-for-byte; `git diff --check`.

Simulator was not launched; no hardware flashing or toolchain changes.
This documentation-only follow-up did not rerun builds/tests. Documentation
links and `git diff --check` were checked after the relocation.

## Next concrete step

Implement observer-relative range, azimuth, and elevation for an explicit
WGS-84 location in `overhead-core`. Reuse `GeodeticPosition::to_ecef()` for
observer position and the new TEME→ECEF position path. Follow decision 0006's
SEZ/azimuth conventions; define zenith/coincident-position and pole behavior,
then add independent reference and analytical geometric edge-case tests.

After that: a reproducible headless runner to complete M1. No UI work yet.

## Open questions / deferred choices

- Confirm which ESP32 board is available for the propagation benchmark;
  confirm final budget on the S3. Ask before flashing or changing toolchains.
- Catalogue limits, prediction workload, and cadence await measurement in M2.
- Projection/zoom semantics and presentation policies get UI feedback in M3+.
- Real location source (0004), Starlink layer (0003), and RTC (0002) stay deferred.

## Known broken / risks

- No failing checks/tests. Embedded target performance is still unmeasured;
  S3 double-precision propagation/prediction cost remains the main compute risk.
- sgp4 2.4's simplified calendar helper mishandles dates after February 2100.
  The new coordinate path avoids it with Chrono elapsed-time arithmetic;
  upstream propagation element-epoch handling remains unchanged.
- Inverse geodetic conversion targets terrestrial/satellite positions, not
  ambiguous deep-interior normal coordinates; non-convergence is an error.
- Minimal Sharp refresh validation stays early; full firmware is M6.
  Target 20 Hz and handle the scarce panel/ribbon gently.
- Cached elements do not provide accurate time after a cold boot without Wi-Fi.

## Gotchas worth remembering

- Pass the propagation timestamp, not the element epoch, to `teme_to_ecef`.
  It supports 1957–2100, rejects explicit leap seconds, and converts positions
  only: velocity conversion would need an additional Earth-rotation term.
- Geodetic longitude is [-π, π), zero on the exact polar axis; height is above
  the WGS-84 ellipsoid, not mean sea level. Keep km/radians in engine APIs.
- Fixture tests need no Python/network; regenerate only via the documented
  pinned offline tools, never from the implementation under test.
- Crates use overhead-* names to avoid the Rust core library name collision.
- SDL2 on Apple Silicon needs -L /opt/homebrew/lib (.cargo/config.toml).
- Keep host I/O out of core/render and presentation state out of physical math.
